//! «Apagado» y «sin red» (T013, FR-012).
//!
//! - **Apagado**: un `Recorder` apagado no mide, no reserva memoria y no crea archivos. La
//!   ausencia de reservas se comprueba con un asignador global que cuenta, por hilo, cada
//!   llamada a `alloc`: diez mil rondas de todos los métodos no deben producir ninguna.
//! - **Sin red**: `cargo tree` del crate no contiene ninguna biblioteca de red, y el código
//!   fuente no usa `std::net`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use dokumentz_metrics::results::RunInfo;
use dokumentz_metrics::{FLAG, Recorder, names};

// ---------------------------------------------------------------------------------------------
// Asignador que cuenta reservas del hilo actual
// ---------------------------------------------------------------------------------------------

thread_local! {
    // `const` y sin destructor: se puede tocar desde dentro del asignador sin reentrar en él.
    static RESERVAS: Cell<usize> = const { Cell::new(0) };
}

struct Contador;

// SAFETY: delega en `System`; solo suma a un contador local del hilo.
unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        RESERVAS.with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        RESERVAS.with(|c| c.set(c.get() + 1));
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, nuevo: usize) -> *mut u8 {
        RESERVAS.with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(ptr, layout, nuevo) }
    }
}

#[global_allocator]
static ASIGNADOR: Contador = Contador;

/// Reservas hechas por `f` en este hilo.
fn reservas_de(f: impl FnOnce()) -> usize {
    let antes = RESERVAS.with(Cell::get);
    f();
    RESERVAS.with(Cell::get) - antes
}

/// Usa todos los métodos públicos del Recorder `rondas` veces.
fn ejercitar(r: &Recorder, rondas: usize, info: &mut Option<RunInfo>) {
    if let Some(i) = info.take() {
        r.set_run(i);
    }
    for i in 0..rondas {
        r.mark(names::LAUNCH);
        r.mark_at(names::WINDOW_READY, 1.5);
        {
            let _tramo = r.span(names::PAINT_MS);
        }
        let _ = r.span(names::DRAW_BLOCK_MS).finish();
        r.span(names::ENCODE_MS).cancel();
        r.sample(names::CELLS_DRAWN_PER_FRAME, i as f64);
        r.count(names::PLACEHOLDER_FRAMES, 1);
        assert!(r.report().is_none());
        assert!(matches!(r.write_json(), Ok(None)));
        assert!(!r.is_on());
        assert!(r.out_path().is_none());
    }
}

// ---------------------------------------------------------------------------------------------
// Apagado
// ---------------------------------------------------------------------------------------------

#[test]
fn apagado_no_reserva_memoria() {
    let r = Recorder::off();
    let mut info = Some(RunInfo::default()); // se construye fuera de la ventana medida
    let n = reservas_de(|| ejercitar(&r, 10_000, &mut info));
    assert_eq!(n, 0, "un Recorder apagado hizo {n} reservas de memoria");
}

#[test]
fn sin_bandera_ni_variable_el_recorder_nace_apagado_y_sin_reservas() {
    let args: Vec<OsString> = vec![OsString::from("--open"), OsString::from("a.xlsx")];
    let r = Recorder::parse(args, None).unwrap();
    let mut info = None;
    assert_eq!(reservas_de(|| ejercitar(&r, 100, &mut info)), 0);
}

#[test]
fn el_contador_de_reservas_no_es_vacuo() {
    // Control: encendido sí reserva (la serie y su nombre), así que el cero de arriba significa algo.
    let r = Recorder::to_path("no-se-escribe.json");
    let n = reservas_de(|| {
        r.sample("serie", 1.0);
        r.count("contador", 1);
    });
    assert!(n > 0);
}

#[test]
fn apagado_no_crea_archivos() {
    let dir = tempfile::tempdir().unwrap();
    let r = Recorder::off();
    ejercitar(&r, 100, &mut None);
    assert!(r.write_json().unwrap().is_none());
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        0,
        "el directorio debe seguir vacío"
    );
}

#[test]
fn encendido_no_escribe_nada_hasta_pedirlo() {
    // Ni al crearlo, ni al medir, ni al soltarlo: solo `write_json` escribe.
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("sub").join("m.json");
    {
        let r = Recorder::parse(
            [OsString::from(FLAG), salida.clone().into_os_string()],
            None,
        )
        .unwrap();
        assert!(r.is_on());
        ejercitar_encendido(&r);
    }
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        0,
        "se escribió algo sin pedirlo"
    );
    assert!(!salida.exists());
}

fn ejercitar_encendido(r: &Recorder) {
    r.mark(names::LAUNCH);
    r.sample("x", 1.0);
    r.count("c", 1);
    drop(r.span("t_ms"));
    assert!(r.report().is_some());
}

// ---------------------------------------------------------------------------------------------
// Sin red
// ---------------------------------------------------------------------------------------------

/// Bibliotecas de red y de ejecución asíncrona que no deben aparecer nunca en la clausura.
const PROHIBIDAS: &[&str] = &[
    "reqwest",
    "hyper",
    "tokio",
    "mio",
    "socket2",
    "ureq",
    "curl",
    "isahc", // T013
    "h2",
    "http",
    "hyper-util",
    "native-tls",
    "rustls",
    "openssl",
    "async-std",
    "smol",
    "attohttpc",
    "surf",
    "tungstenite",
    "quinn",
    "trust-dns-resolver",
    "hickory-resolver",
];

#[test]
fn cargo_tree_no_contiene_bibliotecas_de_red() {
    let manifiesto = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
    let salida = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "dokumentz-metrics",
            "-e",
            "normal,build",
            "--target",
            "all",
        ])
        .args(["--prefix", "none", "--manifest-path", manifiesto])
        .output()
        .expect("no se pudo ejecutar `cargo tree`");
    assert!(
        salida.status.success(),
        "`cargo tree` falló:\n{}",
        String::from_utf8_lossy(&salida.stderr)
    );
    let texto = String::from_utf8_lossy(&salida.stdout);
    let nombres: Vec<&str> = texto
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    // Comprobaciones de cordura: el análisis no está leyendo una salida vacía o equivocada.
    assert_eq!(nombres.first(), Some(&"dokumentz-metrics"), "{texto}");
    assert!(nombres.contains(&"serde_json"), "{texto}");

    let halladas: Vec<&&str> = nombres.iter().filter(|n| PROHIBIDAS.contains(n)).collect();
    assert!(
        halladas.is_empty(),
        "dependencias de red en dokumentz-metrics: {halladas:?}\n{texto}"
    );
}

#[test]
fn el_codigo_fuente_no_usa_la_red() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut revisados = 0;
    for entrada in fs::read_dir(&src).unwrap() {
        let ruta = entrada.unwrap().path();
        if ruta.extension().is_some_and(|e| e == "rs") {
            let texto = fs::read_to_string(&ruta).unwrap();
            for patron in [
                "std::net",
                "TcpStream",
                "TcpListener",
                "UdpSocket",
                "ToSocketAddrs",
            ] {
                assert!(!texto.contains(patron), "{} usa `{patron}`", ruta.display());
            }
            revisados += 1;
        }
    }
    assert!(revisados >= 3, "solo se revisaron {revisados} archivos");
}
