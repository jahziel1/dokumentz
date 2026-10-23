//! E09 (T032): invariante FR-003 — solo se dibuja lo visible.
//!
//! El número de celdas de la lista de dibujo, su tamaño (filas, columnas y bytes codificados) y
//! las celdas que se dibujan por cuadro (`Core::drawn_cells_last`) dependen del **área visible**
//! y de los márgenes de la petición, y **no** del tamaño de la hoja. Se comprueba con libros de
//! 1 000, 100 000 y 1 000 000 filas y el mismo control.
//!
//! * Las pruebas normales (también en `debug`) generan los libros «ralos»: un parche denso de
//!   60 x 12 celdas arriba, en el medio y al final, y un marcador suelto cada décima parte de la
//!   hoja, que fijan el tamaño de la hoja sin pagar un millón de escrituras. Las escrituras van
//!   con la evaluación pausada (`Core::pause_evaluation`).
//! * `libros_densos_*` (solo `--release`, `#[ignore]` en `debug`) generan el `.xlsx` completo
//!   con `rust_xlsxwriter` (2 columnas, hasta un millón de filas: 2 M de celdas, el tamaño de
//!   A2) y lo abren: tarda decenas de segundos y gigas de memoria en `debug`, y ahí no se mide.

mod common;

use dokumentz_sheet::geometry::{DEFAULT_COL_W, DEFAULT_ROW_H};
use dokumentz_sheet::{Command, Core, DrawList, DrawRequest};

use common::{crear_grande, opciones};

const TAMANOS: [i32; 3] = [1_000, 100_000, 1_000_000];
/// Control de las pruebas (cabeceras incluidas): 1046 x 525 px, es decir, 1000 x 500 de celdas.
const CONTROL: (f32, f32) = (1046.0, 525.0);
/// Márgenes de la petición de bloque, como los de la interfaz (§6.3): ±1 altura y ±½ ancho.
const MARGEN: (f32, f32) = (500.0, 500.0);
/// Filas y columnas de cada parche denso.
const PARCHE: (i32, i32) = (60, 12);

/// Una medida del bloque que se pide en una posición del desplazamiento.
#[derive(Debug, Clone, PartialEq)]
struct Medida {
    posicion: &'static str,
    n_rows: u32,
    n_cols: u32,
    celdas: usize,
    por_cuadro: u32,
    bytes: usize,
}

/// Filas y columnas que caben en una petición, como mucho (la cota que depende solo del área).
fn cota(req: &DrawRequest) -> (u32, u32) {
    let filas = (f64::from(req.h + 2.0 * req.margin_y) / DEFAULT_ROW_H).ceil() as u32 + 2;
    let cols = (f64::from(req.w + 2.0 * req.margin_x) / DEFAULT_COL_W).ceil() as u32 + 2;
    (filas, cols)
}

/// Texto de la celda `(fila, col)` del parche: no depende de dónde esté el parche.
fn contenido(fila_en_parche: i32, col: i32) -> String {
    match col % 3 {
        1 => format!("{}", fila_en_parche * col),
        2 => "texto".to_string(),
        _ => "1.5".to_string(),
    }
}

/// Libro ralo de `filas` filas: parches densos en las filas `1`, `filas / 2` y `filas - 59`, y un
/// marcador suelto cada `filas / 10` filas en la columna AD (fuera del control).
fn libro_ralo(filas: i32) -> Core {
    let mut core = Core::new_empty(opciones()).unwrap();
    core.pause_evaluation();
    for inicio in inicios_de_parche(filas) {
        for f in 0..PARCHE.0 {
            for c in 1..=PARCHE.1 {
                core.set_cell_input(0, inicio + f, c, &contenido(f + 1, c))
                    .unwrap();
            }
        }
    }
    for k in 1..=10 {
        core.set_cell_input(0, (filas / 10 * k).min(filas), 30, "marca")
            .unwrap();
    }
    core.set_cell_input(0, filas, 1, "fin").unwrap();
    core.resume_evaluation();
    core.recalculate();
    core.set_viewport(CONTROL.0, CONTROL.1, 1.0);
    core
}

/// Primera fila de los tres parches, en el orden `arriba`, `medio`, `final`.
fn inicios_de_parche(filas: i32) -> [i32; 3] {
    [1, filas / 2, filas - PARCHE.0 + 1]
}

/// Pide el bloque con el área visible alineada al principio de cada parche y mide la lista.
fn medir(core: &mut Core, filas: i32) -> Vec<Medida> {
    let nombres = ["arriba", "medio", "final"];
    let mut medidas = Vec::new();
    for (posicion, inicio) in nombres.into_iter().zip(inicios_de_parche(filas)) {
        let y = core.geometry(0).unwrap().row_y(inicio);
        core.execute(Command::ScrollTo { x: 0.0, y }).unwrap();
        assert_eq!(
            core.view().scroll_y,
            y,
            "{posicion}: el desplazamiento llega"
        );
        let req = core.view().request(MARGEN.0, MARGEN.1);
        let bloque = core.draw_block(req).unwrap();
        comprobar_cotas(&bloque, &req, posicion);
        medidas.push(Medida {
            posicion,
            n_rows: bloque.n_rows,
            n_cols: bloque.n_cols,
            celdas: bloque.cells.len(),
            por_cuadro: core.drawn_cells_last(),
            bytes: Core::encode(&bloque).len(),
        });
    }
    medidas
}

/// Lo que debe cumplir cualquier bloque: cabe en la cota del área pedida y la lista no tiene
/// más celdas que el rectángulo.
fn comprobar_cotas(bloque: &DrawList, req: &DrawRequest, donde: &str) {
    let (filas, cols) = cota(req);
    assert!(
        bloque.n_rows <= filas,
        "{donde}: {} filas > cota {filas}",
        bloque.n_rows
    );
    assert!(
        bloque.n_cols <= cols,
        "{donde}: {} cols > cota {cols}",
        bloque.n_cols
    );
    assert!(
        bloque.cells.len() <= (bloque.n_rows * bloque.n_cols) as usize,
        "{donde}: más celdas que el rectángulo"
    );
    assert!(bloque.covers(req), "{donde}: el bloque cubre lo visible");
}

#[test]
fn el_bloque_depende_del_area_visible_y_no_del_tamano_de_la_hoja() {
    let mut por_tamano = Vec::new();
    for filas in TAMANOS {
        let mut core = libro_ralo(filas);
        // El libro de verdad tiene ese tamaño: la barra de desplazamiento lo refleja.
        let m = core.sheet_metrics();
        assert_eq!(m.used_rows, filas);
        por_tamano.push((filas, m.extent.1, medir(&mut core, filas)));
    }
    // La extensión desplazable crece con la hoja (1000 veces más filas)...
    assert!(por_tamano[2].1 > 100.0 * por_tamano[0].1, "{por_tamano:?}");
    // ...y el bloque no: mismas filas, columnas y celdas, y las mismas celdas por cuadro.
    let (referencia, resto) = por_tamano.split_first().unwrap();
    for (filas, _, medidas) in resto {
        for (a, b) in referencia.2.iter().zip(medidas) {
            assert_eq!(
                (a.n_rows, a.n_cols, a.celdas, a.por_cuadro),
                (b.n_rows, b.n_cols, b.celdas, b.por_cuadro),
                "{} con {} filas frente a {}: {a:?} vs {b:?}",
                a.posicion,
                filas,
                referencia.0
            );
            // El tamaño codificado solo varía por las etiquetas de fila (más dígitos).
            assert!(
                b.bytes <= a.bytes + 8 * b.n_rows as usize,
                "{}: {} bytes con {filas} filas frente a {} con {}",
                a.posicion,
                b.bytes,
                a.bytes,
                referencia.0
            );
        }
    }
    for (filas, extension, medidas) in &por_tamano {
        println!("T032 ralo {filas} filas (extensión {extension} px): {medidas:?}");
    }
    // Y no es trivial: los parches llenan la lista.
    for m in &referencia.2 {
        assert!(m.celdas >= 300, "el parche está en el bloque: {m:?}");
        assert_eq!(m.por_cuadro as usize, m.celdas);
    }
}

#[test]
fn el_bloque_inicial_es_identico_en_las_tres_hojas() {
    // Mismo control, mismo desplazamiento (0, 0): el bloque es el mismo byte a byte.
    let mut codificados = Vec::new();
    for filas in TAMANOS {
        let mut core = libro_ralo(filas);
        let req = core.view().request(MARGEN.0, MARGEN.1);
        let bloque = core.draw_block(req).unwrap();
        codificados.push((bloque.cells.len(), Core::encode(&bloque)));
    }
    assert!(codificados[0].0 > 0);
    assert_eq!(codificados[0], codificados[1]);
    assert_eq!(codificados[1], codificados[2]);
}

#[test]
fn recorrer_la_hoja_de_un_millon_de_filas_nunca_pasa_de_la_cota() {
    let filas = 1_000_000;
    let mut core = libro_ralo(filas);
    let m = core.sheet_metrics();
    let mut max_celdas = 0usize;
    let mut max_filas = 0u32;
    let mut construidos = 0;
    // 400 saltos repartidos por toda la hoja (arrastrar la barra) y el final.
    for i in 0..=400 {
        let y = m.max_scroll.1 * f64::from(i) / 400.0;
        core.execute(Command::ScrollTo { x: 0.0, y }).unwrap();
        let req = core.view().request(MARGEN.0, MARGEN.1);
        let antes = core.data_version();
        let bloque = core.draw_block(req).unwrap();
        assert_eq!(
            antes,
            core.data_version(),
            "desplazarse no cambia los datos"
        );
        comprobar_cotas(&bloque, &req, "recorrido");
        assert_eq!(core.drawn_cells_last() as usize, bloque.cells.len());
        max_celdas = max_celdas.max(bloque.cells.len());
        max_filas = max_filas.max(bloque.n_rows);
        construidos += 1;
    }
    assert_eq!(construidos, 401);
    // Un bloque de 40 filas nunca ve más que un parche (60 x 12 celdas) y algún marcador suelto.
    assert!(
        max_celdas <= (PARCHE.0 * PARCHE.1) as usize + 20,
        "{max_celdas} celdas en un bloque"
    );
    assert!(max_filas <= cota(&core.view().request(MARGEN.0, MARGEN.1)).0);
}

#[test]
fn pedir_el_mismo_area_devuelve_el_bloque_en_cache_sin_reconstruir() {
    let mut core = libro_ralo(100_000);
    let req = core.view().request(MARGEN.0, MARGEN.1);
    let a = core.draw_block(req).unwrap();
    let b = core.draw_block(req).unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    // Un desplazamiento pequeño dentro del margen tampoco reconstruye.
    core.execute(Command::ScrollBy { dx: 0.0, dy: 100.0 })
        .unwrap();
    let req2 = core.view().request(MARGEN.0, MARGEN.1);
    let c = core.draw_block(req2).unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &c));
}

#[test]
fn con_la_caracteristica_metrics_el_contador_de_celdas_sigue_la_misma_regla() {
    // Sin la característica no hay nada que comprobar; con ella, `cells_per_block` registra las
    // celdas de cada bloque construido y no depende del tamaño de la hoja.
    #[cfg(feature = "metrics")]
    {
        use dokumentz_metrics::Recorder;
        use std::sync::Arc;
        let dir = tempfile::tempdir().unwrap();
        let mut maximos = Vec::new();
        for filas in TAMANOS {
            let rec = Arc::new(Recorder::to_path(dir.path().join("m.json")));
            let mut core = libro_ralo(filas);
            core.set_recorder(Arc::clone(&rec));
            medir(&mut core, filas);
            let h = rec.report().unwrap().internal.unwrap().histograms;
            let serie = &h[dokumentz_sheet::metric_names::CELLS_PER_BLOCK];
            assert_eq!(serie.count, 3, "tres bloques construidos");
            maximos.push(serie.max);
        }
        assert_eq!(maximos[0], maximos[1]);
        assert_eq!(maximos[1], maximos[2]);
    }
}

// ---------------------------------------------------------------------------------------------
// Libros densos (solo --release): los datos de verdad están en todas las filas
// ---------------------------------------------------------------------------------------------

/// Abre un libro denso de `filas` x 2 y mide el bloque en el principio, el medio y el final.
fn medir_denso(filas: u32) -> Vec<Medida> {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_grande(dir.path(), filas, 2);
    let mut core = Core::open_xlsx(&ruta, opciones()).unwrap();
    core.set_viewport(CONTROL.0, CONTROL.1, 1.0);
    assert_eq!(core.sheet_metrics().used_rows, filas as i32);
    let n = filas as i32;
    let mut medidas = Vec::new();
    for (posicion, fila) in [("arriba", 1), ("medio", n / 2), ("final", n - 30)] {
        let y = core.geometry(0).unwrap().row_y(fila);
        core.execute(Command::ScrollTo { x: 0.0, y }).unwrap();
        let req = core.view().request(MARGEN.0, MARGEN.1);
        let bloque = core.draw_block(req).unwrap();
        comprobar_cotas(&bloque, &req, posicion);
        // Hoja densa: la lista lleva las dos columnas de todas las filas del bloque que tienen
        // datos (al final de la hoja el margen de abajo cae en filas vacías).
        let con_datos = bloque.last_row().min(n) - bloque.first_row + 1;
        assert_eq!(
            bloque.cells.len(),
            con_datos as usize * 2,
            "{posicion}: dos columnas llenas en {con_datos} filas"
        );
        medidas.push(Medida {
            posicion,
            n_rows: bloque.n_rows,
            n_cols: bloque.n_cols,
            celdas: bloque.cells.len(),
            por_cuadro: core.drawn_cells_last(),
            bytes: Core::encode(&bloque).len(),
        });
    }
    medidas
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "genera libros de hasta 2 M de celdas: solo en --release (los ralos cubren el invariante en debug)"
)]
fn libros_densos_de_mil_cien_mil_y_un_millon_de_filas_dan_el_mismo_bloque() {
    let por_tamano: Vec<(u32, Vec<Medida>)> = [1_000u32, 100_000, 1_000_000]
        .into_iter()
        .map(|f| (f, medir_denso(f)))
        .collect();
    let (referencia, resto) = por_tamano.split_first().unwrap();
    for (filas, medidas) in resto {
        for (a, b) in referencia.1.iter().zip(medidas) {
            assert_eq!(
                (a.n_rows, a.n_cols, a.celdas, a.por_cuadro),
                (b.n_rows, b.n_cols, b.celdas, b.por_cuadro),
                "{} con {filas} filas frente a {}",
                a.posicion,
                referencia.0
            );
            // Los números y las etiquetas de fila tienen más dígitos en la hoja larga.
            assert!(
                b.bytes <= a.bytes + 16 * b.n_rows as usize,
                "{}: {} bytes con {filas} filas frente a {} con {}",
                a.posicion,
                b.bytes,
                a.bytes,
                referencia.0
            );
        }
    }
    for (filas, medidas) in &por_tamano {
        println!("T032 denso {filas} filas: {medidas:?}");
    }
}
