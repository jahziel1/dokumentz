//! T031 [MIDE]: costo de `CommitEdit` sobre un A2 sintético (R-04 de `plan.md`).
//!
//! Libro de 100 000 filas x 20 columnas (2 M de celdas) con 10 000 fórmulas que dependen de una
//! sola celda (U1, `=$U$1*A{fila}` en V2:V10001), el mismo que usan `tests/undo_deps.rs` y
//! `tests/formulas_es_en.rs`. IronCalc 0.9.1 reevalúa **todo el libro** en cada escritura, así
//! que lo que se mide es el recorrido de las celdas más las fórmulas, no solo las dependientes.
//!
//! Qué se mide (ms, cada serie con 15 repeticiones tras 3 de calentamiento):
//!
//! * `commit_edit_total_ms`: `apply(CommitEdit)` sobre U1, sin pausa. Es lo que ve el usuario (M-05).
//! * `evaluate_solo_ms`: `recalculate()` sin ningún cambio: lo que cuesta evaluar.
//! * `commit_edit_pausado_ms`, `evaluate_tras_pausa_ms`, `commit_edit_con_pausa_total_ms`: lo
//!   mismo con `pause_evaluation()`: escribir sin evaluar, evaluar una vez al final, y el total.
//! * `commit_celda_independiente_ms`: escribir en una celda que ningún cálculo usa.
//! * `undo_ms`, `redo_ms` y `paste_3x3_ms` (solo `execute`), `ui_state_ms` y `ui_state_tras_*_ms`
//!   (el estado justo después: deshacer, rehacer y pegar descartan la geometría de la hoja y el
//!   estado la reconstruye).
//! * `escrituras_x9_sin_pausa_ms` / `escrituras_x9_con_pausa_ms`: nueve escrituras seguidas (lo que
//!   hace un pegado de 3 x 3 o un relleno) con una evaluación por escritura o con una sola al final.
//! * Escalas: el mismo `CommitEdit` con 200 000, 1 M y 2 M de celdas (`commit_edit_total_ms_<filas>`),
//!   con 100 dependientes en vez de 10 000 y con casi solo fórmulas (cota inferior de lo que
//!   tardaría un recálculo incremental).
//!
//! ```text
//! cargo run -p dokumentz-sheet --example bench_edit --release --features metrics -- \
//!     --metrics-out results/dev/T031.json
//! ```
//!
//! Sin `--metrics-out` mide y muestra el resumen pero no escribe ningún archivo.

#[path = "../tests/common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use dokumentz_metrics::results::{App, FileInfo};
use dokumentz_metrics::{Histogram, Recorder, RunInfo};
use dokumentz_sheet::{Advance, Command, Core, Dir, EditMode, Lang, OpenOptions};
use rust_xlsxwriter::{Formula, Workbook};

const CALENTAMIENTO: usize = 3;
const REPETICIONES: usize = 15;
/// Columna de la celda de entrada (U) y de las dependientes (V), base 1.
const COL_ENTRADA: i32 = 21;
const COL_DEPENDIENTES: i32 = 22;
const DEPENDIENTES: u32 = 10_000;

struct Banco {
    rec: Arc<Recorder>,
    series: BTreeMap<String, Histogram>,
}

impl Banco {
    fn anotar(&mut self, serie: &str, ms: f64) {
        self.rec.sample(serie, ms);
        self.series.entry(serie.to_string()).or_default().record(ms);
    }

    /// Mediana de lo anotado en `serie` (`NaN` si no hay muestras).
    fn mediana(&self, serie: &str) -> f64 {
        self.series
            .get(serie)
            .and_then(Histogram::p50)
            .unwrap_or(f64::NAN)
    }
}

fn ms_de<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let r = f();
    (r, t.elapsed().as_secs_f64() * 1000.0)
}

fn opciones() -> OpenOptions {
    OpenOptions {
        lang: Lang::Es,
        locale: "en",
        tz: "UTC",
    }
}

/// `A{fila}` del A2 sintético (columna de enteros; ver `common::crear_a2`).
fn a(fila: i32) -> f64 {
    f64::from((fila - 1) * 7 % 100_000)
}

/// Abre el A2 sintético, lo evalúa una vez y deja la celda activa en U1.
fn abrir_a2(dir: &Path, filas: u32, dependientes: u32) -> Core {
    let ruta = common::crear_a2(dir, filas, 20, dependientes);
    let mut core = Core::open_xlsx(&ruta, opciones()).expect("abrir el A2");
    core.recalculate();
    core.apply(Command::SetViewport {
        w: 1440.0,
        h: 900.0,
        scale: 2.0,
    })
    .expect("SetViewport");
    ir_a_u1(&mut core);
    core
}

/// Deja la celda activa en U1 (Ctrl+Inicio y veinte pasos a la derecha).
fn ir_a_u1(core: &mut Core) {
    core.execute(Command::Home { ctrl: true }).expect("Home");
    for _ in 1..COL_ENTRADA {
        core.execute(Command::Move {
            dir: Dir::Right,
            extend: false,
        })
        .expect("Move");
    }
    assert_eq!(
        core.ui_state().active,
        (1, COL_ENTRADA),
        "la celda activa es U1"
    );
}

/// Comprueba la primera y la última dependiente: `V{fila}` = `entrada` x `A{fila}`.
fn comprobar(core: &Core, entrada: f64, dependientes: u32) {
    for fila in [2, dependientes as i32 + 1] {
        let v: f64 = core
            .cell_text(0, fila, COL_DEPENDIENTES)
            .expect("celda")
            .parse()
            .expect("número");
        assert_eq!(v, entrada * a(fila), "V{fila} con U1 = {entrada}");
    }
}

/// `CommitEdit` sobre U1 con el valor `v` (`BeginEdit` fuera del tiempo medido).
fn commit_u1(core: &mut Core, v: u32) -> f64 {
    core.apply(Command::BeginEdit {
        mode: EditMode::Replace,
        seed: Some(v.to_string()),
    })
    .expect("BeginEdit");
    let (r, ms) = ms_de(|| {
        core.apply(Command::CommitEdit {
            text: v.to_string(),
            advance: Advance::Stay,
        })
    });
    r.expect("CommitEdit");
    ms
}

fn linea(nombre: &str, h: &Histogram) {
    if let Some(s) = h.summary() {
        println!(
            "{nombre:<38} n={:<3} p50={:>8.2}  p95={:>8.2}  min={:>8.2}  max={:>8.2}",
            s.count, s.p50, s.p95, s.min, s.max
        );
    }
}

/// Libro con casi solo fórmulas: U1 = 2, A2:A10001 con números y 10 000 fórmulas `=$U$1*A{n}`.
/// 20 000 celdas en total; da la cota inferior de un recálculo que solo tocara las fórmulas.
fn crear_solo_formulas(dir: &Path) -> std::path::PathBuf {
    let ruta = dir.join("solo_formulas.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_number(0, 20, 2).expect("U1");
    for r in 1..=DEPENDIENTES {
        ws.write_number(r, 0, f64::from(r * 7 % 100_000))
            .expect("A");
        ws.write_formula(
            r,
            21,
            Formula::new(format!("=$U$1*A{}", r + 1)).set_result("0"),
        )
        .expect("fórmula");
    }
    wb.save(&ruta).expect("guardar");
    ruta
}

fn main() {
    let rec = Arc::new(Recorder::from_args_env());
    if cfg!(debug_assertions) {
        eprintln!("AVISO: compilación debug; los números no valen (nunca se mide con debug).");
    }
    rec.set_run(RunInfo {
        run_id: "T031".into(),
        app: {
            let mut app = App::new("dokumentz-sheet-bench_edit", dokumentz_sheet::version());
            app.lang = Some("es".into());
            app
        },
        file: Some(FileInfo {
            id: "A2-sintetico".into(),
            sha256: None,
            rows: Some(100_000),
            cols: Some(20),
        }),
        measure: "M-05".into(),
        scenario: "bench_edit: CommitEdit sobre U1 con 10 000 dependientes (nucleo, sin interfaz)"
            .into(),
        cold: false,
        primary: Some("commit_edit_total_ms".into()),
        ..RunInfo::default()
    });
    let mut b = Banco {
        rec,
        series: BTreeMap::new(),
    };

    let dir = tempfile::tempdir().expect("directorio temporal");
    let (mut core, abrir_ms) = ms_de(|| abrir_a2(dir.path(), 100_000, DEPENDIENTES));
    println!(
        "A2 sintético 100 000 x 20 + {DEPENDIENTES} dependientes: generar, abrir y evaluar en {abrir_ms:.0} ms"
    );
    let mut valor = 10;

    // 1. CommitEdit sin pausa (lo que ve el usuario).
    for i in 0..CALENTAMIENTO + REPETICIONES {
        valor += 1;
        let ms = commit_u1(&mut core, valor);
        comprobar(&core, f64::from(valor), DEPENDIENTES);
        if i >= CALENTAMIENTO {
            b.anotar("commit_edit_total_ms", ms);
        }
    }

    // 2. Evaluar sin cambiar nada.
    for i in 0..CALENTAMIENTO + REPETICIONES {
        let ((), ms) = ms_de(|| core.recalculate());
        if i >= CALENTAMIENTO {
            b.anotar("evaluate_solo_ms", ms);
        }
    }

    // 3. Con pause_evaluation: escribir sin evaluar, evaluar una vez, y el total.
    for i in 0..CALENTAMIENTO + REPETICIONES {
        valor += 1;
        core.apply(Command::BeginEdit {
            mode: EditMode::Replace,
            seed: Some(valor.to_string()),
        })
        .expect("BeginEdit");
        let t = Instant::now();
        core.pause_evaluation();
        let (r, escribir) = ms_de(|| {
            core.apply(Command::CommitEdit {
                text: valor.to_string(),
                advance: Advance::Stay,
            })
        });
        r.expect("CommitEdit");
        core.resume_evaluation();
        let ((), evaluar) = ms_de(|| core.recalculate());
        let total = t.elapsed().as_secs_f64() * 1000.0;
        comprobar(&core, f64::from(valor), DEPENDIENTES);
        if i >= CALENTAMIENTO {
            b.anotar("commit_edit_pausado_ms", escribir);
            b.anotar("evaluate_tras_pausa_ms", evaluar);
            b.anotar("commit_edit_con_pausa_total_ms", total);
        }
    }

    // 4. Una celda que ningún cálculo usa (la columna C, fila 50 000): cuesta lo mismo.
    for i in 0..CALENTAMIENTO + REPETICIONES {
        let texto = format!("nota {i}");
        let (r, ms) = ms_de(|| core.set_cell_input(0, 50_000, 3, &texto));
        r.expect("set_cell_input");
        if i >= CALENTAMIENTO {
            b.anotar("commit_celda_independiente_ms", ms);
        }
    }

    // 5. Deshacer y rehacer (E08 midió ≈ 32,6 ms) y el estado para la interfaz.
    for i in 0..CALENTAMIENTO + REPETICIONES {
        valor += 1;
        commit_u1(&mut core, valor);
        // `execute` (deshacer y rehacer) y `ui_state` aparte: deshacer y rehacer descartan la
        // geometría de la hoja y el primer `ui_state` (o `draw_block`) la reconstruye.
        let (r, undo) = ms_de(|| core.execute(Command::Undo));
        r.expect("Undo");
        let (_, estado_undo) = ms_de(|| core.ui_state());
        let (r, redo) = ms_de(|| core.execute(Command::Redo));
        r.expect("Redo");
        let (_, estado_redo) = ms_de(|| core.ui_state());
        comprobar(&core, f64::from(valor), DEPENDIENTES);
        if i >= CALENTAMIENTO {
            b.anotar("undo_ms", undo);
            b.anotar("redo_ms", redo);
            b.anotar("ui_state_tras_undo_ms", estado_undo);
            b.anotar("ui_state_tras_redo_ms", estado_redo);
        }
    }
    for _ in 0..200 {
        let (_, ms) = ms_de(|| core.ui_state());
        b.anotar("ui_state_ms", ms);
    }

    // 6. Pegar 3 x 3 lejos de las fórmulas (Y1) y deshacerlo.
    for _ in 0..4 {
        core.execute(Command::Move {
            dir: Dir::Right,
            extend: false,
        })
        .expect("Move");
    }
    for i in 0..CALENTAMIENTO + REPETICIONES {
        let (r, ms) =
            ms_de(|| core.execute(Command::PasteText("1\t2\t3\n4\t5\t6\n7\t8\t9".into())));
        r.expect("PasteText");
        let (_, estado) = ms_de(|| core.ui_state());
        core.execute(Command::Undo).expect("Undo");
        if i >= CALENTAMIENTO {
            b.anotar("paste_3x3_ms", ms);
            b.anotar("ui_state_tras_paste_ms", estado);
        }
    }

    // 7. Nueve escrituras seguidas: una evaluación por escritura o una sola al final.
    for i in 0..3 + 7 {
        let (_, sin) = ms_de(|| {
            for k in 0..9 {
                core.set_cell_input(0, 60_000 + k, 3, "x")
                    .expect("escribir");
            }
        });
        let (_, con) = ms_de(|| {
            core.pause_evaluation();
            for k in 0..9 {
                core.set_cell_input(0, 60_000 + k, 4, "y")
                    .expect("escribir");
            }
            core.resume_evaluation();
            core.recalculate();
        });
        if i >= 3 {
            b.anotar("escrituras_x9_sin_pausa_ms", sin);
            b.anotar("escrituras_x9_con_pausa_ms", con);
        }
    }

    // 8. Escalas: el mismo CommitEdit con menos celdas, con menos dependientes y con casi solo
    //    fórmulas.
    drop(core);
    for (filas, dependientes, etiqueta) in [
        (10_001u32, DEPENDIENTES, "10001"),
        (50_000, DEPENDIENTES, "50000"),
        (100_000, 100, "100000_100dep"),
    ] {
        let d = tempfile::tempdir().expect("directorio temporal");
        let mut c = abrir_a2(d.path(), filas, dependientes);
        let serie = format!("commit_edit_total_ms_{etiqueta}");
        for i in 0..2 + 9 {
            valor += 1;
            let ms = commit_u1(&mut c, valor);
            comprobar(&c, f64::from(valor), dependientes);
            if i >= 2 {
                b.anotar(&serie, ms);
            }
        }
    }
    {
        let d = tempfile::tempdir().expect("directorio temporal");
        let ruta = crear_solo_formulas(d.path());
        let mut c = Core::open_xlsx(&ruta, opciones()).expect("abrir");
        c.recalculate();
        ir_a_u1(&mut c);
        for i in 0..2 + 9 {
            valor += 1;
            let ms = commit_u1(&mut c, valor);
            comprobar(&c, f64::from(valor), DEPENDIENTES);
            if i >= 2 {
                b.anotar("commit_edit_total_ms_solo_formulas", ms);
            }
        }
    }

    // Resumen.
    println!("\n--- ms (medianas, p95, mínimo y máximo) ---");
    for (nombre, h) in &b.series {
        linea(nombre, h);
    }
    let total = b.mediana("commit_edit_total_ms");
    println!(
        "CommitEdit sin pausa: {total:.1} ms (evaluar sin cambios: {:.1} ms; evaluar tras el cambio: {:.1} ms)",
        b.mediana("evaluate_solo_ms"),
        b.mediana("evaluate_tras_pausa_ms")
    );
    println!(
        "Con pausa: escribir {:.2} ms + evaluar {:.1} ms = {:.1} ms de punta a punta",
        b.mediana("commit_edit_pausado_ms"),
        b.mediana("evaluate_tras_pausa_ms"),
        b.mediana("commit_edit_con_pausa_total_ms")
    );
    println!(
        "Nueve escrituras: {:.0} ms (una evaluación por escritura) frente a {:.0} ms (pausa y una evaluación)",
        b.mediana("escrituras_x9_sin_pausa_ms"),
        b.mediana("escrituras_x9_con_pausa_ms")
    );
    println!(
        "Pegar 3 x 3: {:.1} ms (+ UiState {:.1} ms). Deshacer: {:.1} ms (+ UiState {:.1} ms). Rehacer: {:.1} ms (+ UiState {:.1} ms). UiState normal: {:.4} ms.",
        b.mediana("paste_3x3_ms"),
        b.mediana("ui_state_tras_paste_ms"),
        b.mediana("undo_ms"),
        b.mediana("ui_state_tras_undo_ms"),
        b.mediana("redo_ms"),
        b.mediana("ui_state_tras_redo_ms"),
        b.mediana("ui_state_ms")
    );
    println!(
        "Escala (ms por CommitEdit): 200 mil celdas {:.1}, 1 M {:.1}, 2 M {:.1}; con 100 dependientes (2 M) {:.1}; casi solo fórmulas (20 mil celdas) {:.1}",
        b.mediana("commit_edit_total_ms_10001"),
        b.mediana("commit_edit_total_ms_50000"),
        total,
        b.mediana("commit_edit_total_ms_100000_100dep"),
        b.mediana("commit_edit_total_ms_solo_formulas")
    );
    match b.rec.write_json() {
        Ok(Some(ruta)) => println!("\nJSON escrito en {}", ruta.display()),
        Ok(None) => println!("\nSin --metrics-out: no se escribió ningún archivo."),
        Err(e) => {
            eprintln!("no se pudo escribir el JSON: {e}");
            std::process::exit(1);
        }
    }
}
