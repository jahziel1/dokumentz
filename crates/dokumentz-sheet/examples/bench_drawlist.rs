//! T030 [MIDE]: costo de `draw_block` recorriendo la hoja como un desplazamiento real.
//!
//! Genera en un directorio temporal un libro de 100 000 filas x 20 columnas (A1 sintético: números,
//! textos, fechas y estilos; más 10 000 fórmulas), lo abre con el núcleo y lo recorre con
//! `ScrollBy`/`ScrollTo` pidiendo cada vez el bloque de pantalla más márgenes (±1 altura
//! vertical y ±½ ancho horizontal, `plan.md` §6.3). La serie `draw_block_ms` recoge la
//! **construcción** de cada bloque (las respuestas de la caché no cuentan; ver `Core::draw_block`).
//!
//! Escenarios: `rueda` (la hoja entera de arriba abajo en pasos de 100 px), `horizontal` (ida y
//! vuelta por las columnas), `diagonal` (trackpad), `saltos` (arrastre de la barra: 300 posiciones
//! al azar, cada una reconstruye) y `final` (salto al final de la hoja).
//!
//! ```text
//! cargo run -p dokumentz-sheet --example bench_drawlist --release --features metrics -- \
//!     --metrics-out results/dev/T030.json
//! ```
//!
//! Sin `--metrics-out` (ni `DOKUMENTZ_METRICS_OUT`) mide y muestra el resumen en pantalla pero no
//! escribe ningún archivo. `--rows N` cambia las filas (para una prueba rápida).
//!
//! Presupuesto interno (`plan.md` §2.6, M-03): p95 de `draw_block` <= 3 ms. Nunca se mide con
//! `debug`: el JSON queda marcado como inválido (`debug_build`).

#[path = "../tests/common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use dokumentz_metrics::results::{App, FileInfo};
use dokumentz_metrics::{Histogram, Recorder, RunInfo, names};
use dokumentz_sheet::metric_names as nombres;
use dokumentz_sheet::{Command, Core, DrawList, Lang, OpenOptions};

/// Control de la cuadrícula en px lógicos (un MacBook de 14"): 1440 x 900, escala 2.
const CONTROL: (f32, f32, f32) = (1440.0, 900.0, 2.0);
/// Presupuesto interno de `draw_block` (p95, ms).
const PRESUPUESTO_P95_MS: f64 = 3.0;
const COLUMNAS: u16 = 20;
const FORMULAS: u32 = 10_000;

struct Banco {
    core: Core,
    rec: Arc<Recorder>,
    /// Último bloque devuelto: si el nuevo es el mismo `Arc`, fue una respuesta de la caché.
    ultimo: Option<Arc<DrawList>>,
    /// Márgenes de la petición, en px de hoja (`plan.md` §6.3).
    margen: (f32, f32),
    /// Series propias del banco (tiempos medidos desde fuera), por escenario.
    series: BTreeMap<String, Histogram>,
    pasos: u64,
}

impl Banco {
    /// Anota una muestra en el `Recorder` (JSON) y en las series locales (pantalla).
    fn anotar(&mut self, serie: &str, ms: f64) {
        self.rec.sample(serie, ms);
        self.local(serie, ms);
    }

    /// Anota una muestra solo en las series locales (pantalla): la del núcleo ya está en el JSON.
    fn local(&mut self, serie: &str, ms: f64) {
        self.series.entry(serie.to_string()).or_default().record(ms);
    }

    /// Un cuadro: pide el bloque del área visible. Si el núcleo tuvo que construirlo, anota su
    /// tiempo (medido desde fuera) en la serie del escenario y codifica el bloque.
    fn cuadro(&mut self, escenario: &str) {
        let req = self.core.view().request(self.margen.0, self.margen.1);
        let t = Instant::now();
        let bloque = self.core.draw_block(req).expect("draw_block");
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        self.pasos += 1;
        self.rec.sample(
            names::CELLS_DRAWN_PER_FRAME,
            f64::from(self.core.drawn_cells_last()),
        );
        if self
            .ultimo
            .as_ref()
            .is_some_and(|a| Arc::ptr_eq(a, &bloque))
        {
            return;
        }
        self.anotar(&format!("construir_{escenario}_ms"), ms);
        // `draw_block_ms` (del núcleo) ya junta todos los escenarios en el JSON.
        self.local("construir_todos_ms", ms);
        // Lo que enviaría Tauri por el puente: codificar el bloque nuevo.
        let t = Instant::now();
        let bytes = self.core.encode_block(&bloque);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        self.local("codificar_externo_ms", ms);
        self.anotar("bloque_bytes", bytes.len() as f64);
        self.ultimo = Some(bloque);
    }

    fn scroll_by(&mut self, dx: f64, dy: f64) {
        let t = Instant::now();
        self.core
            .apply(Command::ScrollBy { dx, dy })
            .expect("ScrollBy");
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        self.anotar("apply_scroll_by_ms", ms);
    }

    fn scroll_to(&mut self, x: f64, y: f64) {
        self.core
            .apply(Command::ScrollTo { x, y })
            .expect("ScrollTo");
    }

    fn max_scroll(&mut self) -> (f64, f64) {
        self.core.sheet_metrics().max_scroll
    }
}

fn argumento(nombre: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == nombre)
        .and_then(|i| args.get(i + 1).cloned())
}

fn linea(nombre: &str, h: &Histogram) {
    match h.summary() {
        Some(s) => println!(
            "{nombre:<34} n={:<6} p50={:>7.3}  p95={:>7.3}  p99={:>7.3}  max={:>7.3}",
            s.count, s.p50, s.p95, s.p99, s.max
        ),
        None => println!("{nombre:<34} sin muestras"),
    }
}

fn main() {
    let rec = Arc::new(Recorder::from_args_env());
    let filas: u32 = argumento("--rows").map_or(100_000, |v| v.parse().expect("--rows N"));
    let formulas = FORMULAS.min(filas.saturating_sub(1));
    if cfg!(debug_assertions) {
        eprintln!("AVISO: compilación debug; los números no valen (nunca se mide con debug).");
    }

    let dir = tempfile::tempdir().expect("directorio temporal");
    let t = Instant::now();
    let ruta = common::crear_a2(dir.path(), filas, COLUMNAS, formulas);
    println!(
        "libro {filas} x {COLUMNAS} (+{formulas} fórmulas) generado en {:.1} s",
        t.elapsed().as_secs_f64()
    );

    rec.set_run(RunInfo {
        run_id: "T030".into(),
        app: {
            let mut app = App::new("dokumentz-sheet-bench_drawlist", dokumentz_sheet::version());
            app.lang = Some("es".into());
            app
        },
        file: Some(FileInfo {
            id: "A1-sintetico".into(),
            sha256: None,
            rows: Some(u64::from(filas)),
            cols: Some(u64::from(COLUMNAS)),
        }),
        measure: "M-03".into(),
        scenario: "bench_drawlist: recorrido de la hoja (nucleo, sin interfaz)".into(),
        cold: false,
        primary: Some(nombres::DRAW_BLOCK_MS.into()),
        ..RunInfo::default()
    });

    let opts = OpenOptions {
        lang: Lang::Es,
        locale: "en",
        tz: "UTC",
    };
    let t = Instant::now();
    let mut core = Core::open_xlsx_with_recorder(&ruta, opts, Arc::clone(&rec)).expect("abrir");
    println!(
        "abrir (load_ms): {:.1} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    // La primera orden con tamaño construye la geometría de la hoja (la extensión de las barras
    // la necesita): se mide aparte porque ocurre una sola vez, al abrir.
    let t = Instant::now();
    core.apply(Command::SetViewport {
        w: CONTROL.0,
        h: CONTROL.1,
        scale: CONTROL.2,
    })
    .expect("SetViewport");
    let geometria_ms = t.elapsed().as_secs_f64() * 1000.0;
    rec.sample("set_viewport_con_geometria_ms", geometria_ms);
    let alto = core.view().cells_h() as f32;
    let ancho = core.view().cells_w() as f32;
    let mut banco = Banco {
        core,
        rec: Arc::clone(&rec),
        ultimo: None,
        // ±1 altura vertical y ±½ ancho horizontal.
        margen: (ancho / 2.0, alto),
        series: BTreeMap::new(),
        pasos: 0,
    };

    // Lo que hace la interfaz al abrir: el primer bloque.
    let m = banco.core.sheet_metrics();
    banco.cuadro("primero");
    println!(
        "SetViewport con la geometría de la hoja: {geometria_ms:.2} ms; extensión {:.0} x {:.0} px; primer bloque {} celdas",
        m.extent.0,
        m.extent.1,
        banco.core.drawn_cells_last()
    );

    // 1. Rueda: la hoja entera de arriba abajo, 100 px por paso.
    let t = Instant::now();
    banco.scroll_to(0.0, 0.0);
    let (_, max_y) = banco.max_scroll();
    let mut y = 0.0;
    while y < max_y {
        banco.scroll_by(0.0, 100.0);
        banco.cuadro("rueda");
        y += 100.0;
    }
    println!(
        "rueda: {} cuadros en {:.1} s",
        banco.pasos,
        t.elapsed().as_secs_f64()
    );

    // 2. Horizontal: de la columna A a la derecha del rango usado y de vuelta, 40 px por paso,
    //    a la altura de los datos (fila 5 000).
    let y_datos = banco.core.geometry(0).expect("geometría").row_y(5_000);
    banco.scroll_to(0.0, y_datos);
    let max_x = banco.max_scroll().0.max(1.0);
    for _ in 0..2 {
        let mut x = 0.0;
        while x < max_x {
            banco.scroll_by(40.0, 0.0);
            banco.cuadro("horizontal");
            x += 40.0;
        }
        while x > 0.0 {
            banco.scroll_by(-40.0, 0.0);
            banco.cuadro("horizontal");
            x -= 40.0;
        }
    }

    // 3. Diagonal (trackpad): 3 000 pasos de (12, 90) px desde el principio.
    banco.scroll_to(0.0, 0.0);
    for _ in 0..3_000 {
        banco.scroll_by(12.0, 90.0);
        banco.cuadro("diagonal");
    }

    // 4. Saltos de la barra: 300 posiciones al azar (semilla fija); cada una reconstruye.
    let (max_x, max_y) = banco.max_scroll();
    let mut semilla: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut azar = move || {
        semilla = semilla
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (semilla >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..300 {
        banco.scroll_to(azar() * max_x.min(2_000.0), azar() * max_y);
        banco.cuadro("saltos");
    }

    // 5. Final de la hoja.
    banco.scroll_to(0.0, max_y);
    banco.cuadro("final");

    // Resumen.
    println!("\n--- desde fuera (Instant alrededor de draw_block), ms ---");
    for (nombre, h) in &banco.series {
        linea(nombre, h);
    }
    let todos = banco.series.get("construir_todos_ms");
    let p95 = todos.and_then(Histogram::p95).unwrap_or(f64::NAN);
    println!(
        "\nbloques construidos: {}; cuadros pedidos: {}",
        todos.map_or(0, Histogram::len),
        banco.pasos
    );
    let veredicto = if p95 <= PRESUPUESTO_P95_MS {
        "CUMPLE"
    } else {
        "NO CUMPLE"
    };
    println!(
        "p95 de construir un bloque = {p95:.3} ms (presupuesto {PRESUPUESTO_P95_MS} ms): {veredicto}"
    );

    // Lo que anotó el propio núcleo con la característica `metrics`.
    match rec.report().and_then(|r| r.internal) {
        Some(internos) => {
            println!("\n--- lo que anotó el núcleo en el Recorder ---");
            for (nombre, h) in &internos.histograms {
                println!(
                    "{nombre:<34} n={:<6} p50={:>9.3}  p95={:>9.3}  p99={:>9.3}  max={:>9.3}",
                    h.count, h.p50, h.p95, h.p99, h.max
                );
            }
            for (nombre, valor) in &internos.counters {
                println!("{nombre:<34} {valor}");
            }
        }
        None => {
            println!("\n(Recorder apagado: sin --metrics-out no se registra ni se escribe nada)")
        }
    }
    match rec.write_json() {
        Ok(Some(ruta)) => println!("\nJSON escrito en {}", ruta.display()),
        Ok(None) => println!("\nSin --metrics-out: no se escribió ningún archivo."),
        Err(e) => {
            eprintln!("no se pudo escribir el JSON: {e}");
            std::process::exit(1);
        }
    }
}
