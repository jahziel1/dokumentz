//! E08 (T025): deshacer y rehacer con dependientes (FR-008, US2.10).
//!
//! Sobre un A2 sintético, una celda (`U1`) de la que dependen 10 000 fórmulas (`V2:V10001`,
//! `=$U$1*A{fila}`): editar → cambian todos los dependientes → `Undo` → todos vuelven →
//! `Redo` → se reaplican.
//!
//! * `--release` (la verificación de la etapa): el A2 completo de 100 000 × 20 y la medición de
//!   los tiempos de editar, deshacer y rehacer (`results/dev/T025.json`).
//! * `debug`: la variante pequeña, de 10 001 × 20 (las 10 000 dependientes son las mismas, pero
//!   hay 10 veces menos celdas que evaluar), sin mediciones: nunca se mide con `debug`.

mod common;

use std::time::Instant;

use dokumentz_sheet::{Command, Core, Lang, OpenOptions};
use serde_json::json;

use common::{crear_a2, mediana, redondear, registrar_medicion};

const DEPENDIENTES: u32 = 10_000;
/// Columna de la celda de entrada (U) y de las dependientes (V), base 1.
const COL_ENTRADA: i32 = 21;
const COL_DEPENDIENTES: i32 = 22;

/// Filas del libro: completo en `--release`, pequeño en `debug`.
fn filas() -> u32 {
    if cfg!(debug_assertions) {
        DEPENDIENTES + 1
    } else {
        100_000
    }
}

/// Valor de `A{fila}` en el A2 sintético (columna de números enteros, `fila` base 1).
fn a(fila: i32) -> f64 {
    f64::from((fila - 1) * 7 % 100_000)
}

fn abrir_a2(dir: &std::path::Path) -> Core {
    let ruta = crear_a2(dir, filas(), 20, DEPENDIENTES);
    let mut core = Core::open_xlsx(
        &ruta,
        OpenOptions {
            lang: Lang::Es,
            locale: "en",
            tz: "UTC",
        },
    )
    .unwrap();
    core.recalculate();
    core
}

fn numero(core: &Core, fila: i32, col: i32) -> f64 {
    core.cell_text(0, fila, col)
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("celda ({fila}, {col}) no es un número"))
}

/// Comprueba las 10 000 dependientes: `V{fila}` = `entrada` × `A{fila}`.
fn comprobar_dependientes(core: &Core, entrada: f64) {
    assert_eq!(numero(core, 1, COL_ENTRADA), entrada, "U1");
    for fila in 2..=(DEPENDIENTES as i32 + 1) {
        let esperado = entrada * a(fila);
        let v = numero(core, fila, COL_DEPENDIENTES);
        assert_eq!(v, esperado, "V{fila} con U1 = {entrada}");
    }
}

#[test]
fn editar_deshacer_rehacer_con_diez_mil_dependientes() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir_a2(dir.path());
    assert!(!core.can_undo(), "abrir y recalcular no es una edición");
    comprobar_dependientes(&core, 2.0);

    // Editar U1 → cambian los valores.
    core.set_cell_input(0, 1, COL_ENTRADA, "5").unwrap();
    comprobar_dependientes(&core, 5.0);
    assert!(core.can_undo());
    assert!(!core.can_redo());

    // Undo → todos los dependientes vuelven.
    core.execute(Command::Undo).unwrap();
    comprobar_dependientes(&core, 2.0);
    assert!(!core.can_undo());
    assert!(core.can_redo());

    // Redo → se reaplican.
    core.execute(Command::Redo).unwrap();
    comprobar_dependientes(&core, 5.0);
    assert!(core.can_undo());
    assert!(!core.can_redo());
}

#[test]
fn varias_ediciones_se_deshacen_en_orden_y_se_rehacen() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir_a2(dir.path());
    for v in ["3", "4", "10"] {
        core.set_cell_input(0, 1, COL_ENTRADA, v).unwrap();
    }
    comprobar_dependientes(&core, 10.0);
    core.undo().unwrap();
    comprobar_dependientes(&core, 4.0);
    core.undo().unwrap();
    comprobar_dependientes(&core, 3.0);
    core.undo().unwrap();
    comprobar_dependientes(&core, 2.0);
    core.undo().unwrap(); // nada que deshacer: no hace nada
    comprobar_dependientes(&core, 2.0);
    core.redo().unwrap();
    core.redo().unwrap();
    comprobar_dependientes(&core, 4.0);
    // Una edición nueva descarta lo que quedaba por rehacer.
    core.set_cell_input(0, 1, COL_ENTRADA, "7").unwrap();
    assert!(!core.can_redo());
    comprobar_dependientes(&core, 7.0);
}

#[test]
fn deshacer_y_rehacer_actualizan_lo_que_se_pinta() {
    // La lista de dibujo refleja cada paso: `data_version` sube y el bloque en caché se
    // descarta.
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir_a2(dir.path());
    core.set_viewport(1500.0, 400.0, 1.0);
    // Las columnas U y V están a la derecha de la ventana: se lleva la vista hasta ellas.
    let (x, _) = (core.geometry(0).unwrap().col_x(COL_ENTRADA), 0.0);
    core.execute(Command::ScrollTo { x, y: 0.0 }).unwrap();
    let texto_v2 = |core: &mut Core| {
        let req = core.view().request(0.0, 0.0);
        let bloque = core.draw_block(req).unwrap();
        let celda = bloque
            .find_cell(2, COL_DEPENDIENTES)
            .expect("V2 en el bloque");
        bloque.cell_text(celda).to_string()
    };
    assert_eq!(texto_v2(&mut core), "14");
    let v0 = core.data_version();
    core.set_cell_input(0, 1, COL_ENTRADA, "5").unwrap();
    assert_eq!(texto_v2(&mut core), "35");
    core.undo().unwrap();
    assert!(core.data_version() > v0);
    assert_eq!(texto_v2(&mut core), "14");
    core.redo().unwrap();
    assert_eq!(texto_v2(&mut core), "35");
}

#[test]
fn deshacer_con_una_edicion_abierta_no_hace_nada() {
    // El deshacer del texto que se está escribiendo es de la interfaz; el del libro espera.
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir_a2(dir.path());
    core.set_cell_input(0, 1, COL_ENTRADA, "5").unwrap();
    core.execute(Command::BeginEdit {
        mode: dokumentz_sheet::EditMode::Replace,
        seed: Some("9".into()),
    })
    .unwrap();
    core.execute(Command::Undo).unwrap();
    assert_eq!(
        numero(&core, 1, COL_ENTRADA),
        5.0,
        "el libro no cambia durante la edición"
    );
    core.execute(Command::CancelEdit).unwrap();
    core.execute(Command::Undo).unwrap();
    assert_eq!(numero(&core, 1, COL_ENTRADA), 2.0);
}

/// Medición de T025 (solo `--release`): tiempo de editar, deshacer y rehacer la celda de la que
/// dependen 10 000 fórmulas, y el de deshacer más volver a pintar (incluye reconstruir la
/// geometría, que deshacer invalida).
#[test]
#[cfg_attr(debug_assertions, ignore = "medición: solo en --release")]
fn medicion_t025_editar_deshacer_rehacer_sobre_a2() {
    const REPETICIONES: usize = 5;
    let ms = |t: Instant| t.elapsed().as_secs_f64() * 1000.0;
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir_a2(dir.path());
    core.set_viewport(1500.0, 400.0, 1.0);
    let req = core.view().request(0.0, 0.0);
    core.draw_block(req).unwrap(); // construye la geometría una vez
    let (mut editar, mut deshacer, mut rehacer, mut deshacer_y_pintar) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for i in 0..REPETICIONES {
        let t = Instant::now();
        core.set_cell_input(0, 1, COL_ENTRADA, &format!("{}", 5 + i))
            .unwrap();
        editar.push(ms(t));
        let t = Instant::now();
        core.undo().unwrap();
        deshacer.push(ms(t));
        let t = Instant::now();
        core.redo().unwrap();
        rehacer.push(ms(t));
        let t = Instant::now();
        core.undo().unwrap();
        let req = core.view().request(0.0, 0.0);
        core.draw_block(req).unwrap();
        deshacer_y_pintar.push(ms(t));
        comprobar_dependientes(&core, 2.0);
    }
    let serie = |v: &[f64]| {
        json!({"mediana_ms": redondear(mediana(v.to_vec())),
               "muestras_ms": v.iter().map(|x| redondear(*x)).collect::<Vec<_>>()})
    };
    println!(
        "T025 — editar {:.1} ms; deshacer {:.1} ms; rehacer {:.1} ms; deshacer + pintar {:.1} ms",
        mediana(editar.clone()),
        mediana(deshacer.clone()),
        mediana(rehacer.clone()),
        mediana(deshacer_y_pintar.clone()),
    );
    registrar_medicion(
        "T025",
        json!({
            "libro": {"filas": filas(), "columnas": 20, "dependientes": DEPENDIENTES, "perfil": "release"},
            "editar_u1": serie(&editar),
            "deshacer": serie(&deshacer),
            "rehacer": serie(&rehacer),
            "deshacer_y_pintar": serie(&deshacer_y_pintar),
            "nota": "deshacer y rehacer descartan la geometría; deshacer_y_pintar incluye reconstruirla",
        }),
    );
}
