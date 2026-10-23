//! T016: geometría de la hoja (`col_x/row_y/col_at/row_at`, extensión y construcción desde el
//! motor). La medición de 100 000 filas es `#[ignore]` y se corre en `--release`:
//!
//! ```text
//! cargo test -p dokumentz-sheet --release --test geometry -- --ignored --nocapture
//! ```

mod common;

use std::time::Instant;

use dokumentz_sheet::geometry::{DEFAULT_COL_W, DEFAULT_ROW_H, MAX_COLS, MAX_ROWS};
use dokumentz_sheet::{COL_HEADER_H, ROW_HEADER_W, SheetGeometry, View};
use ironcalc_base::types::{Cell, Col, Row};
use ironcalc_base::{COLUMN_WIDTH_FACTOR, Model, ROW_HEIGHT_FACTOR, UserModel};

use common::{abrir, crear_tamanos};

fn cerca(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn constantes_de_cabecera() {
    assert_eq!(ROW_HEADER_W, 46.0);
    assert_eq!(COL_HEADER_H, 25.0);
}

#[test]
fn sin_excepciones_todo_es_por_defecto() {
    let g = SheetGeometry::default();
    assert_eq!(g.col_x(1), 0.0);
    assert_eq!(g.col_x(2), 90.0);
    assert_eq!(g.row_y(1), 0.0);
    assert_eq!(g.row_y(3), 50.0);
    assert_eq!((g.col_w(5), g.row_h(5)), (90.0, 25.0));
    assert_eq!(g.col_at(0.0), 1);
    assert_eq!(g.col_at(89.999), 1);
    assert_eq!(g.col_at(90.0), 2);
    assert_eq!(g.row_at(24.999), 1);
    assert_eq!(g.row_at(25.0), 2);
    assert_eq!(g.total_w(), f64::from(MAX_COLS) * DEFAULT_COL_W);
    assert_eq!(g.total_h(), f64::from(MAX_ROWS) * DEFAULT_ROW_H);
    // Fuera de la hoja: se acota, no se desborda.
    assert_eq!(g.col_at(-50.0), 1);
    assert_eq!(g.row_at(f64::NAN), 1);
    assert_eq!(g.col_at(1e15), MAX_COLS);
    assert_eq!(g.row_at(f64::INFINITY), MAX_ROWS);
    assert_eq!(
        g.row_y(0),
        0.0,
        "una fila fuera de rango se acota a la primera"
    );
    assert_eq!(g.row_y(MAX_ROWS + 5), g.total_h());
    assert_eq!((g.col_exceptions(), g.row_exceptions()), (0, 0));
}

#[test]
fn tamanos_personalizados_y_ocultos() {
    // Columna 2 de 180 px, columna 4 oculta; fila 2 de 46,875 px, fila 4 oculta.
    let g = SheetGeometry::new(
        DEFAULT_COL_W,
        DEFAULT_ROW_H,
        vec![(2, 180.0), (4, 0.0)],
        vec![(2, 46.875), (4, 0.0)],
        6,
        5,
    );
    let xs: Vec<f64> = (1..=6).map(|c| g.col_x(c)).collect();
    assert_eq!(xs, vec![0.0, 90.0, 270.0, 360.0, 360.0, 450.0]);
    let ys: Vec<f64> = (1..=6).map(|r| g.row_y(r)).collect();
    assert!(
        ys.iter()
            .zip([0.0, 25.0, 71.875, 96.875, 96.875, 121.875])
            .all(|(a, b)| cerca(*a, b)),
        "{ys:?}"
    );
    assert_eq!((g.col_w(2), g.col_w(4), g.col_w(5)), (180.0, 0.0, 90.0));
    // col_at: los bordes pertenecen a la columna que empieza ahí; la oculta se salta.
    assert_eq!(g.col_at(0.0), 1);
    assert_eq!(g.col_at(89.9), 1);
    assert_eq!(g.col_at(90.0), 2);
    assert_eq!(g.col_at(269.9), 2);
    assert_eq!(g.col_at(270.0), 3);
    assert_eq!(g.col_at(359.9), 3);
    assert_eq!(
        g.col_at(360.0),
        5,
        "la columna 4 está oculta: x = 360 cae en la 5"
    );
    assert_eq!(g.col_at(449.9), 5);
    assert_eq!(g.col_at(450.0), 6);
    assert_eq!(g.row_at(96.0), 3);
    assert_eq!(g.row_at(96.875), 5, "la fila 4 está oculta");
    // Ida y vuelta para toda columna y fila visibles.
    for c in (1..=40).filter(|c| *c != 4) {
        assert_eq!(g.col_at(g.col_x(c)), c, "col {c}");
        assert_eq!(
            g.col_at(g.col_x(c) + g.col_w(c) / 2.0),
            c,
            "mitad de col {c}"
        );
    }
    for r in (1..=40).filter(|r| *r != 4) {
        assert_eq!(g.row_at(g.row_y(r)), r, "fila {r}");
    }
    assert_eq!((g.used_rows(), g.used_cols()), (6, 5));
}

/// Generador congruencial lineal (sin dependencias) para casos reproducibles.
struct Lcg(u64);
impl Lcg {
    fn siguiente(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn coincide_con_un_calculo_ingenuo_en_configuraciones_aleatorias() {
    let mut lcg = Lcg(7);
    for caso in 0..60 {
        // Tamaños ingenuos para las posiciones 1..=300; unos 40 con excepción, algunos ocultos,
        // algunos repetidos (gana el primero) y alguno igual al valor por defecto.
        let n = 300;
        let mut tamanos = vec![DEFAULT_COL_W; n + 1];
        let mut excepciones = Vec::new();
        for _ in 0..40 {
            let i = 1 + (lcg.siguiente() as usize) % n;
            let t = match lcg.siguiente() % 4 {
                0 => 0.0,
                1 => DEFAULT_COL_W,
                _ => 10.0 + (lcg.siguiente() % 300) as f64 / 2.0,
            };
            if !excepciones.iter().any(|(j, _)| *j == i as i32) {
                tamanos[i] = t;
            }
            excepciones.push((i as i32, t));
        }
        let g = SheetGeometry::new(DEFAULT_COL_W, DEFAULT_ROW_H, excepciones, vec![], 0, 0);
        let x_de = |i: usize| -> f64 { tamanos[1..i].iter().sum() };
        for (i, tam) in tamanos.iter().enumerate().skip(1) {
            assert!(cerca(g.col_x(i as i32), x_de(i)), "caso {caso}, col_x({i})");
            assert!(cerca(g.col_w(i as i32), *tam), "caso {caso}, col_w({i})");
        }
        // col_at contra «la primera columna cuyo borde derecho supera la posición».
        let limite = x_de(n);
        for _ in 0..400 {
            let p = (lcg.siguiente() % (limite as u64 * 100)) as f64 / 100.0;
            let esperado = (1..=n).find(|i| x_de(i + 1) > p + 1e-7).unwrap() as i32;
            let obtenido = g.col_at(p);
            // En un borde exacto la comparación ingenua y la nuestra pueden diferir por el
            // error de redondeo: se acepta si p está a menos de 1e-6 del borde.
            if obtenido != esperado {
                let borde = g.col_x(obtenido.max(esperado));
                assert!(
                    cerca(p, borde),
                    "caso {caso}: col_at({p}) = {obtenido}, esperado {esperado}"
                );
            }
        }
    }
}

#[test]
fn un_millon_de_filas_sinteticas() {
    // 1 000 000 de filas con altura propia (ninguna igual al valor por defecto) más las
    // 48 576 restantes por defecto. Alturas: 10, 12,5, 15, 17,5 o 20.
    let n = 1_000_000_i32;
    let alturas: Vec<(i32, f64)> = (1..=n)
        .map(|i| (i, 10.0 + f64::from(i % 5) * 2.5))
        .collect();
    let t = Instant::now();
    let g = SheetGeometry::new(DEFAULT_COL_W, DEFAULT_ROW_H, vec![], alturas, n, 3);
    println!("construir con 1 000 000 de excepciones: {:?}", t.elapsed());
    assert_eq!(g.row_exceptions(), n as usize);

    // Suma exacta con enteros (en medios puntos): 20 + 5*(i % 5) / 2.
    let mut suma_dobles: u64 = 0;
    for i in 1..=n as u64 {
        suma_dobles += 20 + 5 * (i % 5);
    }
    let alto_total = suma_dobles as f64 / 2.0;
    assert!(cerca(g.row_y(n + 1), alto_total));
    assert!(cerca(
        g.total_h(),
        alto_total + f64::from(MAX_ROWS - n) * DEFAULT_ROW_H
    ));

    // Ida y vuelta en una muestra.
    for r in (1..=n).step_by(9_973).chain([1, n, n + 1, n + 2, MAX_ROWS]) {
        assert_eq!(g.row_at(g.row_y(r)), r, "fila {r}");
        let mitad = g.row_y(r) + g.row_h(r) / 2.0;
        assert_eq!(g.row_at(mitad), r, "mitad de la fila {r}");
    }
    // Una posición cualquiera cae entre los bordes de la fila que se devuelve.
    let mut lcg = Lcg(99);
    for _ in 0..2_000 {
        let y = (lcg.siguiente() % (g.total_h() as u64)) as f64 + 0.25;
        let r = g.row_at(y);
        assert!(
            g.row_y(r) <= y + 1e-6 && y < g.row_y(r + 1) + 1e-6,
            "y = {y}, fila {r}"
        );
    }
    assert_eq!(g.row_at(g.total_h() + 1.0), MAX_ROWS);
}

#[test]
fn la_extension_es_el_rango_usado_mas_una_pantalla() {
    // 100 filas y 10 columnas usadas, tamaños por defecto.
    let g = SheetGeometry::new(DEFAULT_COL_W, DEFAULT_ROW_H, vec![], vec![], 100, 10);
    let vista = View::default(); // control de 1280 x 800
    let (w, h) = g.extent(&vista);
    assert!(cerca(vista.cells_w(), 1280.0 - 46.0));
    assert!(cerca(vista.cells_h(), 800.0 - 25.0));
    assert!(cerca(w, 10.0 * 90.0 + vista.cells_w()));
    assert!(cerca(h, 100.0 * 25.0 + vista.cells_h()));
    // Hoja vacía: solo la pantalla.
    let vacia = SheetGeometry::default();
    let (w, h) = vacia.extent(&vista);
    assert!(cerca(w, vista.cells_w()) && cerca(h, vista.cells_h()));
    // Nunca pasa del tamaño total de la hoja.
    let llena = SheetGeometry::new(
        DEFAULT_COL_W,
        DEFAULT_ROW_H,
        vec![],
        vec![],
        MAX_ROWS,
        MAX_COLS,
    );
    let (w, h) = llena.extent(&vista);
    assert!(cerca(w, llena.total_w()) && cerca(h, llena.total_h()));
}

#[test]
fn construida_desde_un_xlsx_coincide_con_el_motor() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_tamanos(dir.path());
    let mut core = abrir(&ruta);
    let g = core.geometry(0).unwrap();
    // Valores esperados del fixture (ver `common::crear_tamanos`).
    assert_eq!(g.col_w(1), 90.0);
    // `rust_xlsxwriter` guarda 20 caracteres como 20,7109375 (suma el relleno de 5 px de
    // Excel); el motor lo multiplica por 9.
    assert!(
        cerca(g.col_w(2), 20.7109375 * COLUMN_WIDTH_FACTOR),
        "{}",
        g.col_w(2)
    );
    assert_eq!(g.col_w(4), 0.0, "la columna D está oculta");
    assert!(
        cerca(g.row_h(2), 30.0 * ROW_HEIGHT_FACTOR),
        "30 puntos x 1,5625"
    );
    assert_eq!(g.row_h(4), 0.0, "la fila 4 está oculta");
    assert_eq!(g.row_h(1), 25.0);
    // Rango usado: A1:E6.
    assert_eq!((g.used_rows(), g.used_cols()), (6, 5));

    // Un oráculo independiente: el mismo archivo abierto con IronCalc y sus getters.
    let um = UserModel::from_model(
        ironcalc::import::load_from_xlsx(ruta.to_str().unwrap(), "en", "UTC", "en").unwrap(),
    );
    for c in 1..=30 {
        assert!(
            cerca(g.col_w(c), um.get_column_width(0, c).unwrap()),
            "ancho de col {c}"
        );
    }
    for r in 1..=30 {
        assert!(
            cerca(g.row_h(r), um.get_row_height(0, r).unwrap()),
            "alto de fila {r}"
        );
    }
}

/// Hoja sintética para probar `from_worksheet` sin pasar por un archivo.
fn modelo_con(filas: Vec<Row>, cols: Vec<Col>) -> Model<'static> {
    let mut m = Model::new_empty("sintetico", "en", "UTC", "en").unwrap();
    m.workbook.worksheets[0].rows = filas;
    m.workbook.worksheets[0].cols = cols;
    m
}

fn fila(r: i32, puntos: f64, oculta: bool) -> Row {
    Row {
        r,
        height: puntos,
        custom_format: false,
        custom_height: true,
        s: 0,
        hidden: oculta,
    }
}

fn columna(min: i32, max: i32, caracteres: f64, personalizada: bool, oculta: bool) -> Col {
    Col {
        min,
        max,
        width: caracteres,
        custom_width: personalizada,
        hidden: oculta,
        style: None,
    }
}

#[test]
fn rangos_repetidos_y_solapados_siguen_la_regla_del_motor() {
    // Misma regla que `Worksheet::get_column_width` / `row_height`: gana la primera entrada
    // que contiene la fila o columna, aunque deje el valor por defecto.
    let m = modelo_con(
        vec![
            fila(3, 20.0, false),
            fila(3, 40.0, false), // repetida: no cuenta
            fila(5, 16.0, false), // 16 pt = 25 px = por defecto
            fila(7, 9.0, true),   // oculta
        ],
        vec![
            columna(2, 4, 10.0, true, false),
            columna(3, 6, 20.0, true, false), // solapa: C y D ya las decidió la primera
            columna(8, 9, 99.0, false, false), // sin ancho propio: por defecto
            columna(10, 10, 5.0, true, true), // oculta gana a custom_width
            columna(1, 3, 77.0, true, false), // solapa con 2..=4 en B y C; A sí queda
        ],
    );
    let ws = &m.workbook.worksheets[0];
    let g = SheetGeometry::from_worksheet(ws);
    for c in 1..=20 {
        let motor = ws.get_column_width(c).unwrap();
        assert!(
            cerca(g.col_w(c), motor),
            "col {c}: geometría {} motor {motor}",
            g.col_w(c)
        );
    }
    for r in 1..=20 {
        let motor = ws.row_height(r).unwrap();
        assert!(
            cerca(g.row_h(r), motor),
            "fila {r}: geometría {} motor {motor}",
            g.row_h(r)
        );
    }
    // Solo se guarda lo distinto del valor por defecto.
    assert_eq!(g.row_exceptions(), 2, "fila 3 y fila 7 oculta");
    // Y un rango de toda la hoja con ancho por defecto no crea excepciones.
    let m = modelo_con(vec![], vec![columna(1, MAX_COLS, 10.0, false, false)]);
    let g = SheetGeometry::from_worksheet(&m.workbook.worksheets[0]);
    assert_eq!(g.col_exceptions(), 0);
    // Rangos que se salen de la hoja se recortan.
    let m = modelo_con(vec![], vec![columna(16_380, 99_999, 20.0, true, false)]);
    let g = SheetGeometry::from_worksheet(&m.workbook.worksheets[0]);
    assert_eq!(g.col_exceptions(), 5);
    assert_eq!(g.col_w(MAX_COLS), 180.0);
}

#[test]
fn el_rango_usado_sale_de_los_datos_de_la_hoja() {
    let mut m = modelo_con(vec![], vec![]);
    let g = SheetGeometry::from_worksheet(&m.workbook.worksheets[0]);
    assert_eq!((g.used_rows(), g.used_cols()), (0, 0), "hoja vacía");
    let ws = &mut m.workbook.worksheets[0];
    ws.sheet_data
        .set_cell(12, 3, Cell::NumberCell { v: 1.0, s: 0 });
    ws.sheet_data
        .set_cell(4, 9, Cell::NumberCell { v: 2.0, s: 0 });
    let g = SheetGeometry::from_worksheet(&m.workbook.worksheets[0]);
    assert_eq!((g.used_rows(), g.used_cols()), (12, 9));
}

// ---------------------------------------------------------------------------
// Medición (T016): construir la geometría de 100 000 filas, en --release.
// ---------------------------------------------------------------------------

fn mediana(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

#[test]
#[ignore = "medición: correr con --release (objetivo <= 20 ms para 100 000 filas)"]
fn medir_construccion_de_100000_filas() {
    if cfg!(debug_assertions) {
        panic!("no se mide con `debug` (regla 6): use --release");
    }
    // 100 000 filas, todas con altura propia (peor caso), 20 columnas con datos y 3 columnas
    // de ancho propio, como el A1 sintético de spec.md.
    let filas: Vec<Row> = (1..=100_000)
        .map(|r| fila(r, 16.0 + f64::from(r % 3), false))
        .collect();
    let cols = vec![
        columna(3, 3, 30.0, true, false),
        columna(5, 6, 12.0, true, false),
    ];
    let mut m = modelo_con(filas, cols);
    {
        let ws = &mut m.workbook.worksheets[0];
        for r in 1..=100_000 {
            for c in 1..=20 {
                ws.sheet_data.set_cell(
                    r,
                    c,
                    Cell::NumberCell {
                        v: f64::from(r * c),
                        s: 0,
                    },
                );
            }
        }
    }
    let ws = &m.workbook.worksheets[0];
    let mut total = Vec::new();
    let mut solo_dimension = Vec::new();
    for _ in 0..9 {
        let t = Instant::now();
        let g = SheetGeometry::from_worksheet(ws);
        total.push(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(g.row_exceptions(), 100_000 - rows_con_25(ws));
        let t = Instant::now();
        let d = ws.dimension();
        solo_dimension.push(t.elapsed().as_secs_f64() * 1000.0);
        assert_eq!((d.max_row, d.max_column), (100_000, 20));
    }
    let (total, dimension) = (mediana(total), mediana(solo_dimension));
    println!(
        "from_worksheet (100 000 filas con altura propia, 2 000 000 de celdas): {total:.2} ms"
    );
    println!("  de ellos, Worksheet::dimension() (rango usado): {dimension:.2} ms");

    // Las consultas después de construir.
    let g = SheetGeometry::from_worksheet(ws);
    let t = Instant::now();
    let mut acc = 0.0;
    for r in (1..=100_000).step_by(7) {
        acc += g.row_y(r) + f64::from(g.row_at(g.row_y(r) + 1.0));
    }
    let consultas_ms = t.elapsed().as_secs_f64() * 1000.0;
    assert!(acc > 0.0);
    println!(
        "{} consultas row_y + row_at: {consultas_ms:.2} ms",
        100_000 / 7
    );
    registrar(total, dimension, consultas_ms);
    assert!(
        total <= 20.0,
        "objetivo T016: <= 20 ms; se midieron {total:.2} ms"
    );
}

/// Filas de `ws` cuya altura calculada coincide con el valor por defecto (no son excepción).
fn rows_con_25(ws: &ironcalc_base::types::Worksheet) -> usize {
    ws.rows
        .iter()
        .filter(|f| !f.hidden && (f.height * ROW_HEIGHT_FACTOR - 25.0).abs() < f64::EPSILON)
        .count()
}

/// Guarda la medición en `results/dev/T016.json` (solo en --release; regla 6).
fn registrar(construir_ms: f64, dimension_ms: f64, consultas_ms: f64) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/dev");
    std::fs::create_dir_all(&dir).unwrap();
    let redondear = |x: f64| (x * 100.0).round() / 100.0;
    let json = serde_json::json!({
        "tarea": "T016",
        "perfil": "release",
        "ironcalc": "0.9.1",
        "filas_con_altura_propia": 100_000,
        "celdas": 2_000_000,
        "from_worksheet_ms_mediana_de_9": redondear(construir_ms),
        "de_ello_dimension_ms": redondear(dimension_ms),
        "consultas_row_y_row_at_14285_ms": redondear(consultas_ms),
        "objetivo_ms": 20,
        "maquina": "Apple M1, 16 GB, macOS (ver research.md)",
    });
    std::fs::write(
        dir.join("T016.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();
}
