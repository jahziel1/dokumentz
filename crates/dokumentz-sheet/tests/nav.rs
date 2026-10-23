//! E07 (T020–T022): puntería, selección, navegación y visibilidad de la celda activa.
//!
//! * `hit_*` (T020): `Core::hit_test`, `Click` y `DragTo` con columnas de distinto ancho y con
//!   desplazamiento.
//! * El resto de las pruebas de teclado (T021) usa una hoja de 100 000 filas y una vacía.
//! * `visible_*` (T022): tras cualquier secuencia de movimientos la celda que se mueve está
//!   dentro del área visible y el motor y el núcleo coinciden en la celda superior izquierda.
//!
//! Medidas de las pruebas de teclado: ventana de celdas de 1000 x 500 px (control de
//! 1046 x 525 con las cabeceras), es decir, 20 filas de 25 px por página y 11 columnas de
//! 90 px y un trozo.

mod common;

use std::sync::OnceLock;

use dokumentz_sheet::{COL_HEADER_H, Command, Core, Dir, Hit, ROW_HEADER_W};
use rust_xlsxwriter::Workbook;

use common::{abrir, crear_empty, crear_tamanos, opciones};

/// Filas de la hoja grande.
const FILAS: u32 = 100_000;
/// Última fila y última columna de la hoja.
const ULTIMA_FILA: i32 = 1_048_576;
const ULTIMA_COL: i32 = 16_384;
/// Filas por página con la ventana de las pruebas (500 px / 25 px).
const PAGINA: i32 = 20;

/// Bytes de la hoja grande, generados una sola vez para todas las pruebas.
///
/// * Columna A: números 1..=`FILAS`, sin huecos.
/// * Columna B: texto en B1:B10 y en B20:B30 (hueco de B11 a B19).
/// * D50: un número suelto.
fn bytes_hoja_grande() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for r in 0..FILAS {
            let fila = r + 1;
            ws.write_number(r, 0, f64::from(fila)).unwrap();
            if (1..=10).contains(&fila) || (20..=30).contains(&fila) {
                ws.write_string(r, 1, format!("b{fila}")).unwrap();
            }
            if fila == 50 {
                ws.write_number(r, 3, 7).unwrap();
            }
        }
        wb.save_to_buffer().expect("generar la hoja grande")
    })
}

/// Ventana de las pruebas de teclado: 1000 x 500 px de celdas.
fn ventana(core: &mut Core) {
    core.set_viewport(1046.0, 525.0, 1.0);
}

fn hoja_grande() -> Core {
    let mut core = Core::open_xlsx_bytes(bytes_hoja_grande(), "datos", opciones())
        .expect("abrir la hoja grande");
    ventana(&mut core);
    core
}

fn hoja_vacia() -> Core {
    let mut core = Core::new_empty(opciones()).expect("libro vacío");
    ventana(&mut core);
    core
}

fn con_tamanos() -> (tempfile::TempDir, Core) {
    let dir = tempfile::tempdir().unwrap();
    let core = abrir(&crear_tamanos(dir.path()));
    (dir, core)
}

// --- Ayudantes de lectura y de ejecución ---------------------------------------------------

fn activa(core: &Core) -> (i32, i32) {
    core.selection().active
}

fn foco(core: &Core) -> (i32, i32) {
    core.selection().focus
}

fn rango(core: &Core) -> (i32, i32, i32, i32) {
    core.selection().range
}

fn desplazamiento(core: &Core) -> (f64, f64) {
    (core.view().scroll_x, core.view().scroll_y)
}

fn mover(core: &mut Core, dir: Dir, veces: usize) {
    for _ in 0..veces {
        core.move_active(dir, false).unwrap();
    }
}

fn extender(core: &mut Core, dir: Dir, veces: usize) {
    for _ in 0..veces {
        core.move_active(dir, true).unwrap();
    }
}

/// Punto del control (px lógicos) en el centro de la celda `(fila, col)` con el desplazamiento
/// actual. La celda debe estar a la vista.
fn centro(core: &mut Core, fila: i32, col: i32) -> (f32, f32) {
    let (sx, sy) = desplazamiento(core);
    let g = core.geometry(core.view().sheet).unwrap();
    let x = g.col_x(col) + g.col_w(col) / 2.0 - sx;
    let y = g.row_y(fila) + g.row_h(fila) / 2.0 - sy;
    (ROW_HEADER_W + x as f32, COL_HEADER_H + y as f32)
}

// ============================================================================================
// T020 — hit_test, Click y DragTo
// ============================================================================================
//
// `tamanos.xlsx` (ver `common`): columna B ancha (186,3984375 px), D oculta, las demás de 90 px;
// fila 2 alta (46,875 px), fila 4 oculta, las demás de 25 px. Los bordes se leen de la
// geometría (`col_x`, `row_y`); la prueba comprueba que `hit_test` cae en la celda correcta a
// ambos lados de cada borde.

/// Coordenada x del control del borde izquierdo de la columna `col`, con desplazamiento `sx`.
fn borde_x(core: &mut Core, col: i32, sx: f64) -> f32 {
    let g = core.geometry(core.view().sheet).unwrap();
    ROW_HEADER_W + (g.col_x(col) - sx) as f32
}

/// Coordenada y del control del borde superior de la fila `fila`, con desplazamiento `sy`.
fn borde_y(core: &mut Core, fila: i32, sy: f64) -> f32 {
    let g = core.geometry(core.view().sheet).unwrap();
    COL_HEADER_H + (g.row_y(fila) - sy) as f32
}

#[test]
fn hit_celdas_con_columnas_de_distinto_ancho() {
    let (_dir, mut core) = con_tamanos();
    // Columnas, en la fila 1 (y = 30): (columna, columna visible anterior). El borde izquierdo
    // pertenece a su columna; la D está oculta y su lugar lo ocupa la E.
    for (col, anterior) in [(2, 1), (3, 2), (5, 3), (6, 5), (7, 6)] {
        let x = borde_x(&mut core, col, 0.0);
        assert_eq!(
            core.hit_test(x, 30.0),
            Hit::Cell { row: 1, col },
            "borde de {col}"
        );
        assert_eq!(
            core.hit_test(x - 0.01, 30.0),
            Hit::Cell {
                row: 1,
                col: anterior
            },
            "justo antes de {col}"
        );
    }
    // Las dos primeras columnas miden 90 y 186,4 px: dentro de cada una, la misma columna.
    assert_eq!(core.hit_test(46.0, 30.0), Hit::Cell { row: 1, col: 1 });
    assert_eq!(
        core.hit_test(46.0 + 89.9, 30.0),
        Hit::Cell { row: 1, col: 1 }
    );
    let x_b = borde_x(&mut core, 2, 0.0);
    assert_eq!(
        core.hit_test(x_b + 100.0, 30.0),
        Hit::Cell { row: 1, col: 2 }
    );
    assert_eq!(
        core.hit_test(x_b + 186.0, 30.0),
        Hit::Cell { row: 1, col: 2 }
    );
    // Filas, en la columna A (x = 60). La fila 4 está oculta.
    for (fila, anterior) in [(2, 1), (3, 2), (5, 3), (6, 5)] {
        let y = borde_y(&mut core, fila, 0.0);
        assert_eq!(
            core.hit_test(60.0, y),
            Hit::Cell { row: fila, col: 1 },
            "borde de {fila}"
        );
        assert_eq!(
            core.hit_test(60.0, y - 0.01),
            Hit::Cell {
                row: anterior,
                col: 1
            },
            "justo antes de {fila}"
        );
    }
    assert_eq!(core.hit_test(60.0, 25.0), Hit::Cell { row: 1, col: 1 });
}

#[test]
fn hit_con_desplazamiento() {
    let (_dir, mut core) = con_tamanos();
    assert_eq!(core.scroll_to(100.0, 30.0), (100.0, 30.0));
    // La esquina superior izquierda del área de celdas está en la hoja en (100, 30): columna B,
    // fila 2.
    assert_eq!(core.hit_test(46.0, 25.0), Hit::Cell { row: 2, col: 2 });
    // Los bordes de columna y de fila se corren con el desplazamiento.
    let x = borde_x(&mut core, 3, 100.0);
    assert_eq!(core.hit_test(x - 0.01, 80.0), Hit::Cell { row: 3, col: 2 });
    assert_eq!(core.hit_test(x, 80.0), Hit::Cell { row: 3, col: 3 });
    let y = borde_y(&mut core, 3, 30.0);
    assert_eq!(core.hit_test(100.0, y - 0.01), Hit::Cell { row: 2, col: 2 });
    assert_eq!(core.hit_test(100.0, y), Hit::Cell { row: 3, col: 2 });
    // Las cabeceras siguen el desplazamiento: la de filas cuenta filas, la de columnas, columnas.
    assert_eq!(core.hit_test(20.0, y), Hit::RowHeader { row: 3 });
    assert_eq!(core.hit_test(x, 10.0), Hit::ColHeader { col: 3 });
    assert_eq!(core.hit_test(x - 0.01, 10.0), Hit::ColHeader { col: 2 });
    // Con el desplazamiento al máximo, la primera columna que se ve es otra.
    let (sx, _) = core.scroll_to(1e9, 0.0);
    assert!(sx > 100.0);
    let g = core.geometry(0).unwrap();
    assert_eq!(
        core.hit_test(46.0, 30.0),
        Hit::Cell {
            row: 1,
            col: g.col_at(sx)
        }
    );
}

#[test]
fn hit_cabeceras_esquina_y_fuera() {
    let (_dir, mut core) = con_tamanos();
    assert_eq!(core.hit_test(10.0, 10.0), Hit::Corner);
    assert_eq!(core.hit_test(0.0, 0.0), Hit::Corner);
    assert_eq!(core.hit_test(45.9, 24.9), Hit::Corner);
    assert_eq!(core.hit_test(10.0, 25.0), Hit::RowHeader { row: 1 });
    assert_eq!(core.hit_test(10.0, 25.0 + 30.0), Hit::RowHeader { row: 2 });
    assert_eq!(core.hit_test(46.0, 10.0), Hit::ColHeader { col: 1 });
    assert_eq!(core.hit_test(46.0 + 100.0, 10.0), Hit::ColHeader { col: 2 });
    // Fuera del control, por cualquier lado, y valores no finitos.
    let (w, h) = (core.view().viewport_w, core.view().viewport_h);
    for (x, y) in [
        (-0.1, 50.0),
        (50.0, -0.1),
        (w, 50.0),
        (50.0, h),
        (w + 100.0, h + 100.0),
        (f32::NAN, 50.0),
        (50.0, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NAN),
    ] {
        assert_eq!(core.hit_test(x, y), Hit::Outside, "({x}, {y})");
    }
    // El último punto del control todavía es una celda.
    assert!(matches!(core.hit_test(w - 0.5, h - 0.5), Hit::Cell { .. }));
}

#[test]
fn hit_en_el_borde_de_la_hoja_y_mas_alla() {
    // Una hoja vacía llevada al final: la última fila y columna quedan pegadas al borde.
    let mut core = hoja_vacia();
    core.go_to_edge(Dir::Down, false).unwrap();
    core.go_to_edge(Dir::Right, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, ULTIMA_COL));
    let (w, h) = (core.view().viewport_w, core.view().viewport_h);
    assert_eq!(
        core.hit_test(w - 0.5, h - 0.5),
        Hit::Cell {
            row: ULTIMA_FILA,
            col: ULTIMA_COL
        }
    );
    // La penúltima fila está justo encima de la última (25 px).
    assert_eq!(
        core.hit_test(w - 0.5, h - 25.5),
        Hit::Cell {
            row: ULTIMA_FILA - 1,
            col: ULTIMA_COL
        }
    );
    assert_eq!(core.hit_test(w, h - 0.5), Hit::Outside);
    // Las cabeceras de fila y de columna también dan filas y columnas lejanas.
    assert_eq!(
        core.hit_test(10.0, h - 0.5),
        Hit::RowHeader { row: ULTIMA_FILA }
    );
    assert_eq!(
        core.hit_test(w - 0.5, 10.0),
        Hit::ColHeader { col: ULTIMA_COL }
    );
}

#[test]
fn hit_clic_en_una_celda_la_selecciona() {
    let (_dir, mut core) = con_tamanos();
    let (x, y) = centro(&mut core, 3, 2);
    assert_eq!(
        core.click(x, y, false).unwrap(),
        Hit::Cell { row: 3, col: 2 }
    );
    assert_eq!(activa(&core), (3, 2));
    assert_eq!(rango(&core), (3, 2, 3, 2));
    assert!(core.selection().is_single_cell());
    // Mayús+clic extiende desde la celda activa; el ancla no se mueve.
    let (x, y) = centro(&mut core, 5, 5);
    core.click(x, y, true).unwrap();
    assert_eq!(activa(&core), (3, 2));
    assert_eq!(foco(&core), (5, 5));
    assert_eq!(rango(&core), (3, 2, 5, 5));
    // Un clic sin Mayús vuelve a una celda.
    let (x, y) = centro(&mut core, 1, 1);
    core.click(x, y, false).unwrap();
    assert_eq!(rango(&core), (1, 1, 1, 1));
}

#[test]
fn hit_clic_con_desplazamiento_elige_la_celda_de_la_hoja() {
    let (_dir, mut core) = con_tamanos();
    core.scroll_to(100.0, 30.0);
    // El borde de la columna C y de la fila 3 (más medio píxel) es la celda C3.
    let (x, y) = (
        borde_x(&mut core, 3, 100.0) + 0.5,
        borde_y(&mut core, 3, 30.0) + 0.5,
    );
    assert_eq!(
        core.click(x, y, false).unwrap(),
        Hit::Cell { row: 3, col: 3 }
    );
    assert_eq!(activa(&core), (3, 3));
    // Un clic en una celda ya visible no mueve la vista.
    assert_eq!(desplazamiento(&core), (100.0, 30.0));
}

#[test]
fn hit_clic_en_las_cabeceras_selecciona_filas_y_columnas_enteras() {
    let (_dir, mut core) = con_tamanos();
    // Cabecera de la fila 3: la fila entera; la celda activa, en la primera columna visible.
    assert_eq!(
        core.click(10.0, 25.0 + 80.0, false).unwrap(),
        Hit::RowHeader { row: 3 }
    );
    let sel = core.selection();
    assert_eq!(sel.active, (3, 1));
    assert_eq!(sel.range, (3, 1, 3, ULTIMA_COL));
    assert!(sel.is_entire_rows() && !sel.is_entire_cols());
    // Una fila entera no alarga la extensión desplazable a los 16 384 columnas.
    let m = core.sheet_metrics();
    assert!(m.extent.0 < 10_000.0, "{m:?}");

    // Cabecera de la columna C (con la columna B primera visible por el desplazamiento).
    core.scroll_to(100.0, 0.0);
    let x = borde_x(&mut core, 3, 100.0) + 1.0;
    assert_eq!(
        core.click(x, 10.0, false).unwrap(),
        Hit::ColHeader { col: 3 }
    );
    let sel = core.selection();
    assert_eq!(sel.active, (1, 3));
    assert_eq!(sel.range, (1, 3, ULTIMA_FILA, 3));
    assert!(sel.is_entire_cols() && !sel.is_entire_rows());
    assert!(core.sheet_metrics().extent.1 < 10_000.0);

    // Con la vista desplazada a la derecha, la celda activa de una fila entera es la de la
    // primera columna visible, no la A.
    core.scroll_to(100.0, 0.0);
    core.click(10.0, 25.0 + 10.0, false).unwrap();
    assert_eq!(activa(&core), (1, 2));
    assert_eq!(desplazamiento(&core).0, 100.0, "no salta a la columna A");

    // Mayús+clic en otra cabecera de fila extiende las filas desde la activa.
    core.click(10.0, 25.0 + 80.0, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 3, ULTIMA_COL));
    assert_eq!(activa(&core), (1, 2));
}

#[test]
fn hit_clic_en_la_esquina_selecciona_todo_y_fuera_no_hace_nada() {
    let (_dir, mut core) = con_tamanos();
    let (x, y) = centro(&mut core, 2, 2);
    core.click(x, y, false).unwrap();
    assert_eq!(core.click(10.0, 10.0, false).unwrap(), Hit::Corner);
    let sel = core.selection();
    assert_eq!(sel.range, (1, 1, ULTIMA_FILA, ULTIMA_COL));
    assert_eq!(sel.active, (2, 2), "la celda activa no se mueve");
    assert!(sel.is_entire_rows() && sel.is_entire_cols());
    // Toda la hoja seleccionada no ensancha la extensión.
    let m = core.sheet_metrics();
    assert!(m.extent.0 < 10_000.0 && m.extent.1 < 10_000.0, "{m:?}");

    let antes = core.selection();
    let (w, h) = (core.view().viewport_w, core.view().viewport_h);
    assert_eq!(core.click(w + 5.0, 50.0, false).unwrap(), Hit::Outside);
    assert_eq!(core.click(f32::NAN, 50.0, false).unwrap(), Hit::Outside);
    assert_eq!(core.click(50.0, h, true).unwrap(), Hit::Outside);
    assert_eq!(core.selection(), antes);
}

#[test]
fn hit_arrastre_extiende_el_rango_y_vuelve_atras() {
    let (_dir, mut core) = con_tamanos();
    let (x, y) = centro(&mut core, 2, 2);
    core.click(x, y, false).unwrap();
    let (x, y) = centro(&mut core, 5, 5);
    core.drag_to(x, y).unwrap();
    assert_eq!(activa(&core), (2, 2));
    assert_eq!(foco(&core), (5, 5));
    assert_eq!(rango(&core), (2, 2, 5, 5));
    // Seguir arrastrando hacia la izquierda y arriba de la celda inicial invierte el rango.
    let (x, y) = centro(&mut core, 1, 1);
    core.drag_to(x, y).unwrap();
    assert_eq!(activa(&core), (2, 2));
    assert_eq!(foco(&core), (1, 1));
    assert_eq!(rango(&core), (1, 1, 2, 2));
}

#[test]
fn hit_arrastre_sobre_las_cabeceras_extiende_filas_o_columnas() {
    let (_dir, mut core) = con_tamanos();
    core.click(10.0, 25.0 + 30.0, false).unwrap(); // fila 2
    core.drag_to(10.0, 25.0 + 130.0).unwrap(); // fila 6
    assert_eq!(rango(&core), (2, 1, 6, ULTIMA_COL));
    assert_eq!(activa(&core), (2, 1));
    core.click(46.0 + 100.0, 10.0, false).unwrap(); // columna B
    core.drag_to(46.0 + 400.0, 10.0).unwrap(); // columna E (la D está oculta)
    assert_eq!(rango(&core), (1, 2, ULTIMA_FILA, 5));
    assert_eq!(activa(&core), (1, 2));
}

#[test]
fn hit_arrastre_fuera_de_la_cuadricula_se_acota_y_desplaza_la_vista() {
    let mut core = hoja_grande();
    core.click(ROW_HEADER_W + 5.0, COL_HEADER_H + 5.0, false)
        .unwrap();
    assert_eq!(activa(&core), (1, 1));
    // Más allá del borde derecho: la celda que le tocaría (la columna siguiente) y la vista la
    // muestra entera.
    let (w, h) = (core.view().viewport_w, core.view().viewport_h);
    core.drag_to(w + 10.0, COL_HEADER_H + 30.0).unwrap();
    let col = foco(&core).1;
    assert_eq!(col, 12, "x de hoja 1010 es la columna 12 de 90 px");
    let (sx, _) = desplazamiento(&core);
    assert!(sx > 0.0 && sx <= 90.0 * 11.0 && (sx % 90.0).abs() < 1e-9);
    // Hacia abajo, más allá del borde inferior.
    core.drag_to(w + 10.0, h + 60.0).unwrap();
    assert_eq!(foco(&core).0, 23, "y de hoja 560 (585 - 25) es la fila 23");
    assert!(core.selection().contains(5, 5));
    // Un punto no finito se ignora; arrastrar sin un clic dentro de la cuadrícula, también.
    let antes = core.selection();
    core.drag_to(f32::NAN, 30.0).unwrap();
    assert_eq!(core.selection(), antes);
    core.click(-5.0, -5.0, false).unwrap();
    core.drag_to(200.0, 200.0).unwrap();
    assert_eq!(core.selection(), antes);
}

#[test]
fn hit_en_la_hoja_grande_con_el_desplazamiento_muy_lejos() {
    let mut core = hoja_grande();
    core.go_to_edge(Dir::Down, false).unwrap();
    let (_, sy) = desplazamiento(&core);
    assert_eq!(sy, 99_981.0 * 25.0 - 25.0, "fila superior 99 981");
    // Primera fila visible = fila 99 981; la última fila con datos (100 000) es la que se pulsó.
    assert_eq!(
        core.hit_test(60.0, COL_HEADER_H),
        Hit::Cell {
            row: 99_981,
            col: 1
        }
    );
    let (x, y) = centro(&mut core, 100_000, 1);
    assert_eq!(
        core.click(x, y, false).unwrap(),
        Hit::Cell {
            row: 100_000,
            col: 1
        }
    );
    assert_eq!(activa(&core), (100_000, 1));
}

// ============================================================================================
// T021 — Move, Edge, Page, Home (escenario 6 de US1)
// ============================================================================================

#[test]
fn flechas_en_la_hoja_grande_mueven_la_celda_y_el_desplazamiento_por_filas_enteras() {
    let mut core = hoja_grande();
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(rango(&core), (1, 1, 1, 1));
    // Contra los bordes de la hoja no pasa nada.
    mover(&mut core, Dir::Up, 1);
    mover(&mut core, Dir::Left, 1);
    assert_eq!(activa(&core), (1, 1));

    mover(&mut core, Dir::Down, 3);
    mover(&mut core, Dir::Right, 2);
    assert_eq!(activa(&core), (4, 3));
    mover(&mut core, Dir::Up, 1);
    mover(&mut core, Dir::Left, 1);
    assert_eq!(activa(&core), (3, 2));
    assert_eq!(
        desplazamiento(&core),
        (0.0, 0.0),
        "dentro de la pantalla no se desplaza"
    );

    // Por abajo: hasta la fila 28 (borde inferior en 700 px) la vista sube hasta la fila 9
    // (borde superior en 200 px), que deja la 28 pegada al borde inferior de los 500 px.
    mover(&mut core, Dir::Down, 25);
    assert_eq!(activa(&core), (28, 2));
    assert_eq!(desplazamiento(&core), (0.0, 200.0));
    assert_eq!(core.top_left_cell(), (9, 1));
    // Por la derecha: la columna 13 termina en 1170 px; la vista empieza en la columna 3 (180).
    mover(&mut core, Dir::Right, 11);
    assert_eq!(activa(&core), (28, 13));
    assert_eq!(desplazamiento(&core), (180.0, 200.0));
    assert_eq!(core.top_left_cell(), (9, 3));
    // Por arriba y por la izquierda el borde de la celda se pega al de la ventana.
    mover(&mut core, Dir::Up, 20);
    assert_eq!(activa(&core), (8, 13));
    assert_eq!(desplazamiento(&core), (180.0, 175.0));
    mover(&mut core, Dir::Left, 11);
    assert_eq!(activa(&core), (8, 2));
    assert_eq!(desplazamiento(&core), (90.0, 175.0));
    assert_eq!(core.top_left_cell(), (8, 2));
    // El valor de las celdas es el de la hoja (la selección y los datos coinciden).
    assert_eq!(core.cell_text(0, 8, 1).unwrap(), "8");
}

#[test]
fn flecha_con_un_rango_seleccionado_se_mueve_desde_la_celda_activa_y_lo_reduce() {
    let mut core = hoja_grande();
    mover(&mut core, Dir::Down, 4);
    mover(&mut core, Dir::Right, 1); // E: B5
    extender(&mut core, Dir::Down, 2);
    extender(&mut core, Dir::Right, 1);
    assert_eq!(rango(&core), (5, 2, 7, 3));
    mover(&mut core, Dir::Left, 1);
    assert_eq!(activa(&core), (5, 1));
    assert_eq!(rango(&core), (5, 1, 5, 1));
    // En el borde, la flecha que no puede moverse también deja una sola celda.
    extender(&mut core, Dir::Up, 2);
    assert_eq!(rango(&core), (3, 1, 5, 1));
    mover(&mut core, Dir::Left, 1);
    assert_eq!(rango(&core), (5, 1, 5, 1));
}

#[test]
fn ctrl_flecha_va_al_borde_del_bloque_de_datos_en_la_hoja_grande() {
    let mut core = hoja_grande();
    let ir = |core: &mut Core, dir| core.go_to_edge(dir, false).unwrap();

    // Columna A: un bloque de 100 000 filas. Después del bloque, la hoja está vacía.
    ir(&mut core, Dir::Down);
    assert_eq!(activa(&core), (100_000, 1));
    assert_eq!(rango(&core), (100_000, 1, 100_000, 1));
    assert_eq!(core.top_left_cell(), (99_981, 1));
    assert_eq!(desplazamiento(&core), (0.0, 99_980.0 * 25.0));
    ir(&mut core, Dir::Down);
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    ir(&mut core, Dir::Up);
    assert_eq!(
        activa(&core),
        (100_000, 1),
        "desde el vacío, al primer dato"
    );
    ir(&mut core, Dir::Up);
    assert_eq!(
        activa(&core),
        (1, 1),
        "desde un dato, al principio del bloque"
    );
    assert_eq!(desplazamiento(&core), (0.0, 0.0));

    // Columna B: dos bloques (1–10 y 20–30) con un hueco entre ellos.
    mover(&mut core, Dir::Right, 1);
    for esperado in [10, 20, 30, ULTIMA_FILA] {
        ir(&mut core, Dir::Down);
        assert_eq!(activa(&core), (esperado, 2));
    }
    for esperado in [30, 20, 10, 1] {
        ir(&mut core, Dir::Up);
        assert_eq!(activa(&core), (esperado, 2));
    }

    // Fila 1: A1 y B1 con datos; el resto vacío.
    ir(&mut core, Dir::Left);
    assert_eq!(activa(&core), (1, 1));
    ir(&mut core, Dir::Right);
    assert_eq!(activa(&core), (1, 2));
    ir(&mut core, Dir::Right);
    assert_eq!(activa(&core), (1, ULTIMA_COL));
    ir(&mut core, Dir::Left);
    assert_eq!(activa(&core), (1, 2));
    ir(&mut core, Dir::Left);
    assert_eq!(activa(&core), (1, 1));

    // Fila 50: A50 con dato, B50 vacía y D50 con dato: Ctrl+Derecha salta a D50.
    mover(&mut core, Dir::Down, 49);
    assert_eq!(activa(&core), (50, 1));
    ir(&mut core, Dir::Right);
    assert_eq!(activa(&core), (50, 4));
    ir(&mut core, Dir::Right);
    assert_eq!(activa(&core), (50, ULTIMA_COL));
}

#[test]
fn ctrl_flecha_en_una_hoja_vacia_va_al_borde_de_la_hoja() {
    let mut core = hoja_vacia();
    assert_eq!(activa(&core), (1, 1));
    core.go_to_edge(Dir::Up, false).unwrap();
    core.go_to_edge(Dir::Left, false).unwrap();
    assert_eq!(activa(&core), (1, 1));
    core.go_to_edge(Dir::Down, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    core.go_to_edge(Dir::Right, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, ULTIMA_COL));
    core.go_to_edge(Dir::Up, false).unwrap();
    assert_eq!(activa(&core), (1, ULTIMA_COL));
    core.go_to_edge(Dir::Left, false).unwrap();
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(desplazamiento(&core), (0.0, 0.0));
    assert_eq!(core.top_left_cell(), (1, 1));
}

#[test]
fn mayus_flecha_extiende_el_rango_sin_mover_el_ancla() {
    let mut core = hoja_grande();
    extender(&mut core, Dir::Down, 3);
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(foco(&core), (4, 1));
    assert_eq!(rango(&core), (1, 1, 4, 1));
    extender(&mut core, Dir::Right, 1);
    assert_eq!(rango(&core), (1, 1, 4, 2));
    extender(&mut core, Dir::Up, 1);
    assert_eq!(rango(&core), (1, 1, 3, 2));
    extender(&mut core, Dir::Left, 1);
    assert_eq!(rango(&core), (1, 1, 3, 1));
    // Contra el borde de la hoja el rango no cambia.
    extender(&mut core, Dir::Up, 5);
    extender(&mut core, Dir::Left, 2);
    assert_eq!(rango(&core), (1, 1, 1, 1));
    assert_eq!(foco(&core), (1, 1));

    // Desde una celda del medio el rango crece hacia los dos lados del ancla.
    mover(&mut core, Dir::Down, 9);
    mover(&mut core, Dir::Right, 4);
    assert_eq!(activa(&core), (10, 5));
    extender(&mut core, Dir::Up, 2);
    extender(&mut core, Dir::Left, 1);
    assert_eq!(rango(&core), (8, 4, 10, 5));
    assert_eq!(foco(&core), (8, 4));
    extender(&mut core, Dir::Down, 4);
    assert_eq!(rango(&core), (10, 4, 12, 5));
    assert_eq!(foco(&core), (12, 4));
    assert_eq!(activa(&core), (10, 5));
}

#[test]
fn mayus_flecha_mantiene_a_la_vista_el_foco_y_la_flecha_simple_vuelve_a_la_activa() {
    let mut core = hoja_grande();
    extender(&mut core, Dir::Down, 25);
    assert_eq!(foco(&core), (26, 1));
    // El foco (borde inferior en 650 px) queda pegado al borde de la ventana; la celda activa
    // (A1) ya no se ve, como en Office.
    assert_eq!(desplazamiento(&core), (0.0, 150.0));
    assert_eq!(activa(&core), (1, 1));
    // La flecha simple actúa desde la celda activa y la trae de vuelta a la pantalla.
    mover(&mut core, Dir::Down, 1);
    assert_eq!(activa(&core), (2, 1));
    assert_eq!(rango(&core), (2, 1, 2, 1));
    assert_eq!(desplazamiento(&core), (0.0, 25.0));
}

#[test]
fn ctrl_mayus_flecha_extiende_el_foco_hasta_el_borde_del_bloque() {
    let mut core = hoja_grande();
    core.go_to_edge(Dir::Down, true).unwrap();
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(foco(&core), (100_000, 1));
    assert_eq!(rango(&core), (1, 1, 100_000, 1));
    assert_eq!(core.top_left_cell(), (99_981, 1), "se ve el foco");
    core.go_to_edge(Dir::Down, true).unwrap();
    assert_eq!(rango(&core), (1, 1, ULTIMA_FILA, 1));
    core.go_to_edge(Dir::Up, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 100_000, 1));
    core.go_to_edge(Dir::Up, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 1, 1));
    // Hacia la derecha: de A1 a B1 y de ahí al final de la fila.
    core.go_to_edge(Dir::Right, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 1, 2));
    core.go_to_edge(Dir::Right, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 1, ULTIMA_COL));
    // Sin Mayús, el salto parte de la celda activa y deja una sola celda.
    core.go_to_edge(Dir::Down, false).unwrap();
    assert_eq!(rango(&core), (100_000, 1, 100_000, 1));
}

#[test]
fn pagina_mueve_una_pantalla_y_conserva_la_posicion_relativa() {
    let mut core = hoja_grande();
    let pagina = |core: &mut Core, abajo| core.page(abajo, false).unwrap();
    pagina(&mut core, true);
    assert_eq!(activa(&core), (1 + PAGINA, 1));
    assert_eq!(core.top_left_cell(), (1 + PAGINA, 1));
    assert_eq!(desplazamiento(&core), (0.0, f64::from(PAGINA) * 25.0));
    pagina(&mut core, true);
    assert_eq!(activa(&core), (1 + 2 * PAGINA, 1));
    pagina(&mut core, false);
    assert_eq!(activa(&core), (1 + PAGINA, 1));
    pagina(&mut core, false);
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(desplazamiento(&core), (0.0, 0.0));
    pagina(&mut core, false);
    assert_eq!(activa(&core), (1, 1), "contra el principio de la hoja");
    assert_eq!(desplazamiento(&core), (0.0, 0.0));

    // Desde la cuarta fila de la pantalla: sigue siendo la cuarta tras cada página.
    mover(&mut core, Dir::Down, 3);
    mover(&mut core, Dir::Right, 2);
    pagina(&mut core, true);
    assert_eq!(activa(&core), (4 + PAGINA, 3));
    assert_eq!(core.top_left_cell(), (1 + PAGINA, 1));
    pagina(&mut core, true);
    assert_eq!(activa(&core), (4 + 2 * PAGINA, 3));
    pagina(&mut core, false);
    pagina(&mut core, false);
    assert_eq!(activa(&core), (4, 3));
}

#[test]
fn pagina_sigue_el_alto_de_la_ventana() {
    let mut core = hoja_grande();
    // 325 de control = 300 px de celdas = 12 filas por página.
    core.set_viewport(1046.0, 325.0, 1.0);
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (13, 1));
    // 125 + 25 de cabecera... = 100 px de celdas = 4 filas.
    core.set_viewport(1046.0, 125.0, 1.0);
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (17, 1));
    core.page(false, false).unwrap();
    assert_eq!(activa(&core), (13, 1));
}

#[test]
fn mayus_pagina_extiende_el_foco_una_pantalla() {
    let mut core = hoja_grande();
    core.page(true, true).unwrap();
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(foco(&core), (1 + PAGINA, 1));
    assert_eq!(rango(&core), (1, 1, 1 + PAGINA, 1));
    assert_eq!(core.top_left_cell(), (1 + PAGINA, 1));
    core.page(true, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 1 + 2 * PAGINA, 1));
    core.page(false, true).unwrap();
    assert_eq!(rango(&core), (1, 1, 1 + PAGINA, 1));
    // Hacia arriba, más allá de la celda activa, el rango se invierte.
    mover(&mut core, Dir::Down, 1);
    assert_eq!(activa(&core), (2, 1));
    core.page(true, true).unwrap();
    core.page(false, true).unwrap();
    core.page(false, true).unwrap();
    assert_eq!(activa(&core), (2, 1));
    assert_eq!(
        foco(&core),
        (1, 1),
        "las filas por encima de la activa no bajan de 1"
    );
    assert_eq!(rango(&core), (1, 1, 2, 1));
}

#[test]
fn pagina_en_el_ultimo_tramo_de_la_hoja_no_falla() {
    let mut core = hoja_grande();
    core.go_to_edge(Dir::Down, false).unwrap();
    core.go_to_edge(Dir::Down, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    // Con la última fila abajo del todo, `UserModel` no puede dar una pantalla más (se sale de
    // la hoja): la selección se queda en el borde.
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    core.page(true, true).unwrap();
    assert_eq!(rango(&core), (ULTIMA_FILA, 1, ULTIMA_FILA, 1));
    // Subir una pantalla conserva la posición relativa (la última fila queda abajo del todo).
    core.page(false, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA - PAGINA, 1));
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    // Desde un poco más arriba, AvPág sigue sin una pantalla entera y lleva a la última fila.
    mover(&mut core, Dir::Up, 5);
    assert_eq!(activa(&core), (ULTIMA_FILA - 5, 1));
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    comprobar(&mut core, true, "último tramo");

    // Con la vista una fila más arriba que el tope, `UserModel` sí da una página, pero su
    // fila destino se sale de la hoja (no la valida): el núcleo la acota.
    mover(&mut core, Dir::Up, 1);
    core.scroll_by(0.0, -25.0);
    assert_eq!(activa(&core), (ULTIMA_FILA - 1, 1));
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (ULTIMA_FILA, 1));
    // La selección sigue siendo válida: se puede seguir navegando.
    mover(&mut core, Dir::Left, 1);
    mover(&mut core, Dir::Up, 1);
    core.page(false, true).unwrap();
    assert_eq!(foco(&core).0, ULTIMA_FILA - 1 - PAGINA);
    comprobar(&mut core, true, "tras acotar");
}

#[test]
fn inicio_va_a_la_primera_columna_y_ctrl_inicio_a_a1() {
    let mut core = hoja_grande();
    mover(&mut core, Dir::Down, 5);
    mover(&mut core, Dir::Right, 15);
    assert_eq!(activa(&core), (6, 16));
    assert!(desplazamiento(&core).0 > 0.0);
    core.home(false).unwrap();
    assert_eq!(activa(&core), (6, 1));
    assert_eq!(desplazamiento(&core), (0.0, 0.0));
    assert_eq!(core.top_left_cell(), (1, 1));

    // Ctrl+Inicio lleva a A1 y vuelve el desplazamiento al origen desde cualquier sitio.
    core.go_to_edge(Dir::Down, false).unwrap();
    mover(&mut core, Dir::Right, 4);
    assert!(desplazamiento(&core).1 > 1.0e6);
    core.home(true).unwrap();
    assert_eq!(activa(&core), (1, 1));
    assert_eq!(rango(&core), (1, 1, 1, 1));
    assert_eq!(desplazamiento(&core), (0.0, 0.0));
    // Con un rango seleccionado, Inicio lo reduce a una celda.
    extender(&mut core, Dir::Down, 3);
    core.home(false).unwrap();
    assert_eq!(rango(&core), (1, 1, 1, 1));
}

#[test]
fn inicio_salta_las_columnas_ocultas() {
    // En `tamanos.xlsx` la columna D (4) y la fila 4 están ocultas; ninguna es la primera.
    let (_dir, mut core) = con_tamanos();
    let (x, y) = centro(&mut core, 3, 5);
    core.click(x, y, false).unwrap();
    core.home(false).unwrap();
    assert_eq!(activa(&core), (3, 1));
    core.home(true).unwrap();
    assert_eq!(activa(&core), (1, 1));
}

#[test]
fn flechas_saltan_las_filas_y_columnas_ocultas() {
    let (_dir, mut core) = con_tamanos();
    // C3 -> Abajo: la fila 4 está oculta, sigue la 5. Derecha desde C: la D está oculta, la E.
    let (x, y) = centro(&mut core, 3, 3);
    core.click(x, y, false).unwrap();
    mover(&mut core, Dir::Down, 1);
    assert_eq!(activa(&core), (5, 3));
    mover(&mut core, Dir::Right, 1);
    assert_eq!(activa(&core), (5, 5));
    mover(&mut core, Dir::Up, 1);
    assert_eq!(activa(&core), (3, 5));
    mover(&mut core, Dir::Left, 1);
    assert_eq!(activa(&core), (3, 3));
}

#[test]
fn comandos_ejecutan_la_misma_secuencia_que_los_metodos() {
    let mut a = hoja_grande();
    let mut b = hoja_grande();
    let secuencia = [
        Command::Move {
            dir: Dir::Down,
            extend: false,
        },
        Command::Move {
            dir: Dir::Right,
            extend: true,
        },
        Command::Edge {
            dir: Dir::Down,
            extend: true,
        },
        Command::Page {
            down: false,
            extend: false,
        },
        Command::Home { ctrl: false },
        Command::ScrollBy { dx: 0.0, dy: 300.0 },
        Command::Click {
            x: 200.0,
            y: 100.0,
            extend: false,
        },
        Command::DragTo { x: 400.0, y: 300.0 },
        Command::SetViewport {
            w: 800.0,
            h: 400.0,
            scale: 2.0,
        },
        Command::ScrollTo { x: 90.0, y: 50.0 },
    ];
    for cmd in secuencia {
        a.execute(cmd).unwrap();
    }
    a.move_active(Dir::Up, false).unwrap();
    b.move_active(Dir::Down, false).unwrap();
    b.move_active(Dir::Right, true).unwrap();
    b.go_to_edge(Dir::Down, true).unwrap();
    b.page(false, false).unwrap();
    b.home(false).unwrap();
    b.scroll_by(0.0, 300.0);
    b.click(200.0, 100.0, false).unwrap();
    b.drag_to(400.0, 300.0).unwrap();
    b.set_viewport(800.0, 400.0, 2.0);
    b.scroll_to(90.0, 50.0);
    b.move_active(Dir::Up, false).unwrap();
    assert_eq!(a.selection(), b.selection());
    assert_eq!(a.view(), b.view());
}

// ============================================================================================
// T022 — ensure_visible y sincronía con UserModel
// ============================================================================================

const TOL: f64 = 1e-6;

/// ¿Cabe `[inicio, fin]` en la ventana `[desp, desp + ventana]`? Una celda más grande que la
/// ventana solo puede tener su borde inicial pegado al de la ventana.
fn cabe(inicio: f64, fin: f64, desp: f64, ventana: f64) -> bool {
    if fin - inicio > ventana + TOL {
        (inicio - desp).abs() <= TOL
    } else {
        inicio >= desp - TOL && fin <= desp + ventana + TOL
    }
}

/// Invariantes que deben cumplirse tras cualquier comando: el desplazamiento está dentro de la
/// hoja y el motor y el núcleo coinciden en la celda superior izquierda. Con `objetivo`, además,
/// la celda que se mueve (el foco, salvo en el eje que la selección abarca por completo) está
/// dentro del área de celdas, sin quedar debajo de las cabeceras.
fn comprobar(core: &mut Core, objetivo: bool, contexto: &str) {
    let v = *core.view();
    let g = core.geometry(v.sheet).unwrap();
    let sel = core.selection();
    assert_eq!(sel.sheet, v.sheet, "{contexto}");
    assert!(
        sel.contains(sel.active.0, sel.active.1),
        "{contexto}: {sel:?}"
    );
    assert!(
        sel.contains(sel.focus.0, sel.focus.1),
        "{contexto}: {sel:?}"
    );
    assert!(
        v.scroll_x >= 0.0 && v.scroll_x <= (g.total_w() - v.cells_w()).max(0.0) + TOL,
        "{contexto}: scroll_x {} fuera de la hoja",
        v.scroll_x
    );
    assert!(
        v.scroll_y >= 0.0 && v.scroll_y <= (g.total_h() - v.cells_h()).max(0.0) + TOL,
        "{contexto}: scroll_y {} fuera de la hoja",
        v.scroll_y
    );
    assert_eq!(
        core.top_left_cell(),
        (g.row_at(v.scroll_y), g.col_at(v.scroll_x)),
        "{contexto}: el motor y el núcleo no coinciden en la celda superior izquierda (scroll {}, {})",
        v.scroll_x,
        v.scroll_y
    );
    if !objetivo {
        return;
    }
    if !sel.is_entire_cols() {
        let fila = sel.focus.0;
        assert!(
            cabe(g.row_y(fila), g.row_y(fila + 1), v.scroll_y, v.cells_h()),
            "{contexto}: la fila {fila} no está a la vista (scroll_y {}, alto {}, {sel:?})",
            v.scroll_y,
            v.cells_h()
        );
    }
    if !sel.is_entire_rows() {
        let col = sel.focus.1;
        assert!(
            cabe(g.col_x(col), g.col_x(col + 1), v.scroll_x, v.cells_w()),
            "{contexto}: la columna {col} no está a la vista (scroll_x {}, ancho {}, {sel:?})",
            v.scroll_x,
            v.cells_w()
        );
    }
}

/// Generador pseudoaleatorio SplitMix64 con semilla fija (sin dependencias).
struct Aleatorio(u64);

impl Aleatorio {
    fn siguiente(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entero en `0..n`.
    fn menor_que(&mut self, n: u64) -> u64 {
        self.siguiente() % n
    }

    /// `true` con probabilidad `p` por ciento.
    fn con_probabilidad(&mut self, p: u64) -> bool {
        self.menor_que(100) < p
    }

    /// Decimal en `[lo, hi)`.
    fn entre(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.menor_que(1_000_000) as f32 / 1_000_000.0)
    }

    fn dir(&mut self) -> Dir {
        [Dir::Up, Dir::Down, Dir::Left, Dir::Right][self.menor_que(4) as usize]
    }
}

/// Una secuencia de `pasos` comandos aleatorios con la semilla `semilla`, comprobando los
/// invariantes tras cada uno. Las órdenes de teclado y las de ratón sobre la cuadrícula dejan a
/// la vista la celda que se mueve; el desplazamiento con la rueda y el cambio de tamaño de la
/// ventana no (el usuario se aleja a propósito), pero conservan la coherencia.
fn secuencia_aleatoria(core: &mut Core, semilla: u64, pasos: usize, extiende: bool) {
    let mut azar = Aleatorio(semilla);
    // Sin Mayús la selección nunca pasa de una celda (salvo filas o columnas enteras por las
    // cabeceras) y la celda activa es la que debe verse.
    let prob = |p: u64| if extiende { p } else { 0 };
    let mut arrastre_valido = false;
    for paso in 0..pasos {
        let v = *core.view();
        let (cmd, objetivo) = match azar.menor_que(100) {
            0..=39 => (
                Command::Move {
                    dir: azar.dir(),
                    extend: azar.con_probabilidad(prob(25)),
                },
                true,
            ),
            40..=43 => (
                Command::Edge {
                    dir: azar.dir(),
                    extend: azar.con_probabilidad(prob(25)),
                },
                true,
            ),
            44..=57 => (
                Command::Page {
                    down: azar.con_probabilidad(50),
                    extend: azar.con_probabilidad(prob(20)),
                },
                true,
            ),
            58..=61 => (
                Command::Home {
                    ctrl: azar.con_probabilidad(40),
                },
                true,
            ),
            62..=79 => {
                // Puntos hasta 10 px fuera del control: cubre cabeceras, esquina y exterior.
                let (x, y) = (
                    azar.entre(-10.0, v.viewport_w + 10.0),
                    azar.entre(-10.0, v.viewport_h + 10.0),
                );
                let hit = core.hit_test(x, y);
                arrastre_valido = !matches!(hit, Hit::Outside | Hit::Corner);
                (
                    Command::Click {
                        x,
                        y,
                        extend: azar.con_probabilidad(prob(20)),
                    },
                    arrastre_valido,
                )
            }
            // Arrastrar extiende el rango desde el clic: solo en las pruebas que extienden.
            80..=87 if extiende => (
                Command::DragTo {
                    x: azar.entre(-30.0, v.viewport_w + 30.0),
                    y: azar.entre(-30.0, v.viewport_h + 30.0),
                },
                arrastre_valido,
            ),
            80..=87 => (
                Command::Move {
                    dir: azar.dir(),
                    extend: false,
                },
                true,
            ),
            88..=96 => (
                Command::ScrollBy {
                    dx: f64::from(azar.entre(-700.0, 700.0)),
                    dy: f64::from(azar.entre(-1500.0, 1500.0)),
                },
                false,
            ),
            _ => (
                Command::SetViewport {
                    w: azar.entre(150.0, 1400.0),
                    h: azar.entre(80.0, 900.0),
                    scale: 1.0,
                },
                false,
            ),
        };
        core.execute(cmd.clone())
            .unwrap_or_else(|e| panic!("semilla {semilla}, paso {paso}: {cmd:?}: {e}"));
        let contexto = format!("semilla {semilla}, paso {paso}: {cmd:?}");
        comprobar(core, objetivo, &contexto);
        if !extiende && objetivo {
            comprobar_activa(core, &contexto);
        }
    }
}

/// La celda activa está a la vista (en los ejes que la selección no abarca por completo).
fn comprobar_activa(core: &mut Core, contexto: &str) {
    let v = *core.view();
    let g = core.geometry(v.sheet).unwrap();
    let sel = core.selection();
    let (fila, col) = sel.active;
    if !sel.is_entire_cols() {
        assert!(
            cabe(g.row_y(fila), g.row_y(fila + 1), v.scroll_y, v.cells_h()),
            "{contexto}: la celda activa ({fila}, {col}) no está a la vista en vertical"
        );
    }
    if !sel.is_entire_rows() {
        assert!(
            cabe(g.col_x(col), g.col_x(col + 1), v.scroll_x, v.cells_w()),
            "{contexto}: la celda activa ({fila}, {col}) no está a la vista en horizontal"
        );
    }
}

#[test]
fn visible_tras_movimientos_aleatorios_con_tamanos_personalizados() {
    // Cabeceras, columna ancha (180 px), columna y fila ocultas, fila alta (46,875 px) y
    // ventanas desde amplias hasta más pequeñas que una celda.
    for (n, (w, h)) in [
        (1280.0, 800.0),
        (640.0, 360.0),
        (300.0, 150.0),
        (150.0, 80.0),
    ]
    .into_iter()
    .enumerate()
    {
        let (_dir, mut core) = con_tamanos();
        core.set_viewport(w, h, 1.0);
        for semilla in [7, 2024 + n as u64] {
            secuencia_aleatoria(&mut core, semilla, 600, true);
        }
    }
}

#[test]
fn visible_tras_movimientos_aleatorios_en_una_hoja_vacia() {
    for (n, (w, h)) in [(1046.0, 525.0), (500.0, 300.0), (200.0, 100.0)]
        .into_iter()
        .enumerate()
    {
        let mut core = Core::new_empty(opciones()).unwrap();
        core.set_viewport(w, h, 1.0);
        secuencia_aleatoria(&mut core, 99 + n as u64, 600, true);
    }
}

#[test]
fn visible_tras_movimientos_aleatorios_en_la_hoja_grande() {
    let mut core = hoja_grande();
    secuencia_aleatoria(&mut core, 31_337, 500, true);
    let mut core = hoja_grande();
    core.set_viewport(700.0, 330.0, 1.0);
    secuencia_aleatoria(&mut core, 4242, 300, true);
}

#[test]
fn visible_la_celda_activa_tras_movimientos_aleatorios_sin_extender() {
    // Sin Mayús, la celda que se mueve es la activa: eso es lo que dice T022.
    for (n, (w, h)) in [(1280.0, 800.0), (400.0, 200.0), (150.0, 80.0)]
        .into_iter()
        .enumerate()
    {
        let (_dir, mut core) = con_tamanos();
        core.set_viewport(w, h, 1.0);
        secuencia_aleatoria(&mut core, 1000 + n as u64, 600, false);
    }
    let mut vacia = hoja_vacia();
    secuencia_aleatoria(&mut vacia, 77, 600, false);
    let mut grande = hoja_grande();
    secuencia_aleatoria(&mut grande, 78, 400, false);
}

#[test]
fn visible_la_celda_activa_no_queda_bajo_las_cabeceras() {
    // Tras cada tecla la celda activa cae dentro del área de celdas del control, es decir, a la
    // derecha de la cabecera de filas y debajo de la de columnas, en coordenadas del control.
    let mut core = hoja_grande();
    let mut azar = Aleatorio(5);
    for _ in 0..200 {
        let dir = azar.dir();
        if azar.con_probabilidad(10) {
            core.page(azar.con_probabilidad(50), false).unwrap();
        } else {
            core.move_active(dir, false).unwrap();
        }
        let (fila, col) = activa(&core);
        let (x, y) = centro(&mut core, fila, col);
        assert!(x >= ROW_HEADER_W && x < core.view().viewport_w, "x = {x}");
        assert!(y >= COL_HEADER_H && y < core.view().viewport_h, "y = {y}");
        assert_eq!(core.hit_test(x, y), Hit::Cell { row: fila, col });
    }
}

#[test]
fn visible_tras_desplazarse_con_la_rueda_la_tecla_trae_de_vuelta_la_celda_activa() {
    let mut core = hoja_grande();
    mover(&mut core, Dir::Down, 4);
    mover(&mut core, Dir::Right, 1);
    assert_eq!(activa(&core), (5, 2));
    // Con la rueda, a 2 000 px de distancia (y sin tocar la selección).
    core.scroll_to(0.0, 2000.0);
    assert_eq!(core.top_left_cell(), (81, 1));
    assert_eq!(activa(&core), (5, 2));
    // La flecha actúa desde la celda activa (no desde lo que se ve) y la deja a la vista.
    mover(&mut core, Dir::Down, 1);
    assert_eq!(activa(&core), (6, 2));
    // La tecla trae primero la celda activa (fila 5, borde superior en 100 px) a la vista.
    assert_eq!(desplazamiento(&core), (0.0, 100.0));
    comprobar(&mut core, true, "tras la tecla");
    // También para RePág, Ctrl+Flecha e Inicio.
    core.scroll_to(0.0, 50_000.0);
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (6 + PAGINA, 2));
    core.scroll_to(0.0, 50_000.0);
    core.home(false).unwrap();
    assert_eq!(activa(&core), (6 + PAGINA, 1));
    comprobar(&mut core, true, "tras Inicio");
}

#[test]
fn visible_una_celda_a_medias_se_muestra_entera_al_navegar_pero_no_se_toca_si_ya_cabe() {
    let mut core = hoja_grande();
    // Desplazamiento fino (de trackpad): 37 px, la fila 2 empieza en 25 (queda cortada arriba).
    core.scroll_to(0.0, 37.0);
    assert_eq!(core.top_left_cell(), (2, 1));
    // La celda activa A1 (borde superior en 0) no se ve: la tecla vuelve a ella.
    mover(&mut core, Dir::Down, 1);
    assert_eq!(activa(&core), (2, 1));
    assert_eq!(
        desplazamiento(&core),
        (0.0, 0.0),
        "A1 pegada al borde; A2 ya cabe"
    );
    // Ahora un desplazamiento fino que no oculta la celda activa no se corrige.
    core.scroll_to(0.0, 10.0);
    let antes = desplazamiento(&core);
    assert_eq!(antes, (0.0, 10.0));
    core.ensure_visible();
    assert_eq!(desplazamiento(&core), antes, "A2 (25–50) ya se ve entera");
    // `ensure_cell_visible` lleva cualquier celda a la vista, aunque no sea la activa.
    core.ensure_cell_visible(100, 40);
    comprobar(&mut core, false, "ensure_cell_visible");
    let g = core.geometry(0).unwrap();
    let v = *core.view();
    assert!(cabe(g.row_y(100), g.row_y(101), v.scroll_y, v.cells_h()));
    assert!(cabe(g.col_x(40), g.col_x(41), v.scroll_x, v.cells_w()));
    // Valores fuera de la hoja se acotan, sin pánico.
    core.ensure_cell_visible(i32::MAX, i32::MIN);
    core.ensure_cell_visible(0, -3);
    comprobar(&mut core, false, "valores extremos");
}

#[test]
fn visible_el_desplazamiento_con_la_rueda_mantiene_coherente_al_motor() {
    let mut core = hoja_grande();
    for (dx, dy) in [
        (0.0, 0.0),
        (95.0, 51.0),
        (-10.0, 7000.0),
        (3.0, -1.0e9),
        (1e9, 1e9),
    ] {
        core.scroll_by(dx, dy);
        comprobar(&mut core, false, &format!("scroll_by({dx}, {dy})"));
    }
    core.scroll_to(450.0, 1_234_567.0);
    comprobar(&mut core, false, "scroll_to");
    // Y al cambiar el tamaño de la ventana (también cambia lo que ve RePág).
    core.set_viewport(900.0, 400.0, 2.0);
    comprobar(&mut core, false, "set_viewport");
}

#[test]
fn visible_la_extension_cubre_donde_llego_la_navegacion() {
    // En una hoja vacía, RePág baja una pantalla completa (no se queda corta por la extensión
    // del rango usado) y la barra de desplazamiento cubre lo que se ve.
    let mut core = hoja_vacia();
    core.page(true, false).unwrap();
    assert_eq!(activa(&core), (1 + PAGINA, 1));
    assert_eq!(desplazamiento(&core), (0.0, f64::from(PAGINA) * 25.0));
    let m = core.sheet_metrics();
    assert!(m.extent.1 >= f64::from(PAGINA) * 25.0 + 500.0, "{m:?}");
    assert!(m.max_scroll.1 >= desplazamiento(&core).1);
    // La rueda no puede alejarse más allá de lo que ya cubre la extensión.
    let (_, y) = core.scroll_by(0.0, 1.0e9);
    assert_eq!(y, m.max_scroll.1);
    // Volver al principio con la rueda y con Ctrl+Inicio.
    core.scroll_to(0.0, 0.0);
    comprobar(&mut core, false, "scroll al principio");
    core.page(true, false).unwrap();
    core.home(true).unwrap();
    assert_eq!(desplazamiento(&core), (0.0, 0.0));
    assert_eq!(core.top_left_cell(), (1, 1));
}

#[test]
fn visible_la_hoja_seleccionada_en_el_motor_es_la_de_la_vista() {
    let core = hoja_vacia();
    assert_eq!(core.selected_sheet(), 0);
    assert_eq!(core.view().sheet, core.selected_sheet());
    assert_eq!(core.selection().sheet, core.view().sheet);
    // Un libro que ya trae el nombre de la hoja de `small.xlsx`.
    let dir = tempfile::tempdir().unwrap();
    let core = abrir(&common::crear_small(dir.path()));
    assert_eq!(core.view().sheet, core.selected_sheet());
    let dir = tempfile::tempdir().unwrap();
    let core = abrir(&crear_empty(dir.path()));
    assert_eq!(core.view().sheet, core.selected_sheet());
}
