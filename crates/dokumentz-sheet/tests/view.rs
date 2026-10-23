//! T019: vista, desplazamiento y caché de bloque de `Core`.

mod common;

use std::sync::Arc;

use dokumentz_sheet::{Core, DrawRequest, ROW_HEADER_W};
use rust_xlsxwriter::Workbook;

use common::{abrir, crear_empty, crear_grande, opciones};

/// Libro de 200 filas x 30 columnas con números (`fila * 1000 + col`) y encabezados.
fn grande() -> (tempfile::TempDir, Core) {
    let dir = tempfile::tempdir().unwrap();
    let core = abrir(&crear_grande(dir.path(), 200, 30));
    (dir, core)
}

/// Petición de `cells_w x cells_h` px desde `(x0, y0)` con márgenes de una pantalla vertical y
/// media horizontal.
fn peticion(sheet: u32, x0: f64, y0: f64) -> DrawRequest {
    DrawRequest {
        sheet,
        x0,
        y0,
        w: 900.0,
        h: 575.0,
        margin_x: 450.0,
        margin_y: 575.0,
    }
}

#[test]
fn dentro_del_bloque_cacheado_es_el_mismo_arc() {
    let (_dir, mut core) = grande();
    let a = core.draw_block(peticion(0, 900.0, 2500.0)).unwrap();
    // Desplazarse un poco, dentro de los márgenes: mismo bloque, no uno equivalente.
    for (dx, dy) in [(0.0, 0.0), (90.0, 25.0), (-300.0, -500.0), (400.0, 560.0)] {
        let b = core
            .draw_block(peticion(0, 900.0 + dx, 2500.0 + dy))
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b), "desplazamiento ({dx}, {dy})");
    }
    assert_eq!(core.drawn_cells_last() as usize, a.cells.len());
    // Un área que se sale del bloque por abajo da uno distinto, que cubre la petición.
    let c = core
        .draw_block(peticion(0, 900.0, 2500.0 + 1_200.0))
        .unwrap();
    assert!(!Arc::ptr_eq(&a, &c));
    assert!(c.first_row > a.first_row);
    assert!(c.covers(&peticion(0, 900.0, 3700.0)));
    // Y el siguiente pedido dentro del nuevo bloque vuelve a ser el mismo.
    let d = core.draw_block(peticion(0, 900.0, 3710.0)).unwrap();
    assert!(Arc::ptr_eq(&c, &d));
}

#[test]
fn tras_un_cambio_de_datos_devuelve_un_bloque_nuevo() {
    let (_dir, mut core) = grande();
    let req = peticion(0, 0.0, 0.0);
    let a = core.draw_block(req).unwrap();
    assert_eq!(a.data_version, core.data_version());
    let celda = |dl: &dokumentz_sheet::DrawList| {
        let c = dl.find_cell(2, 2).expect("B2");
        dl.cell_text(c).to_string()
    };
    assert_eq!(celda(&a), "2002");

    core.set_cell_input(0, 2, 2, "hola").unwrap();
    let b = core.draw_block(req).unwrap();
    assert!(
        !Arc::ptr_eq(&a, &b),
        "el bloque viejo no se debe reutilizar"
    );
    assert_eq!(celda(&b), "hola");
    assert_eq!(b.data_version, core.data_version());
    assert!(b.data_version > a.data_version);
    // El bloque viejo sigue siendo válido para quien lo conserve (es un Arc inmutable).
    assert_eq!(celda(&a), "2002");
    // Sin cambios, vuelve a ser el mismo.
    let c = core.draw_block(req).unwrap();
    assert!(Arc::ptr_eq(&b, &c));

    // Recalcular también invalida (los valores pueden haber cambiado).
    core.recalculate();
    let d = core.draw_block(req).unwrap();
    assert!(!Arc::ptr_eq(&c, &d));
}

#[test]
fn el_cache_distingue_hojas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("dos-hojas.xlsx");
    let mut wb = Workbook::new();
    wb.add_worksheet().write_string(0, 0, "uno").unwrap();
    wb.add_worksheet().write_string(0, 0, "dos").unwrap();
    wb.save(&ruta).unwrap();
    let mut core = abrir(&ruta);
    let h0 = core.draw_block(peticion(0, 0.0, 0.0)).unwrap();
    let h1 = core.draw_block(peticion(1, 0.0, 0.0)).unwrap();
    assert_eq!((h0.sheet, h1.sheet), (0, 1));
    assert!(!Arc::ptr_eq(&h0, &h1));
    assert_eq!(h0.cell_text(h0.find_cell(1, 1).unwrap()), "uno");
    assert_eq!(h1.cell_text(h1.find_cell(1, 1).unwrap()), "dos");
}

#[test]
fn el_desplazamiento_se_acota_a_la_extension() {
    let (_dir, mut core) = grande();
    core.set_viewport(1000.0, 600.0, 2.0);
    let v = *core.view();
    assert_eq!((v.viewport_w, v.viewport_h, v.scale), (1000.0, 600.0, 2.0));
    assert_eq!(v.cells_w(), 1000.0 - f64::from(ROW_HEADER_W));
    assert_eq!(v.cells_h(), 575.0);

    // Rango usado: 200 filas x 30 columnas = 2 700 x 5 000 px; más una pantalla de holgura.
    let m = core.sheet_metrics();
    assert_eq!((m.used_rows, m.used_cols), (200, 30));
    assert_eq!(m.extent, (2700.0 + 954.0, 5000.0 + 575.0));
    assert_eq!(m.max_scroll, (2700.0, 5000.0));

    assert_eq!(core.scroll_to(500.0, 800.0), (500.0, 800.0));
    assert_eq!(core.scroll_by(100.0, -300.0), (600.0, 500.0));
    assert_eq!(core.scroll_to(1e9, 1e9), (2700.0, 5000.0), "tope");
    assert_eq!(core.scroll_to(-40.0, -1.0), (0.0, 0.0), "no baja de cero");
    // Un valor no finito deja ese eje como estaba.
    core.scroll_to(300.0, 400.0);
    assert_eq!(core.scroll_to(f64::NAN, 900.0), (300.0, 900.0));
    assert_eq!(core.scroll_to(100.0, f64::INFINITY), (100.0, 900.0));
    assert_eq!(core.view().scroll_x, 100.0);
    // `scroll_by` desde el tope no pasa del tope.
    core.scroll_to(1e9, 1e9);
    assert_eq!(core.scroll_by(500.0, 500.0), (2700.0, 5000.0));
}

#[test]
fn una_hoja_vacia_solo_se_desplaza_dentro_de_su_pantalla() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_empty(dir.path()));
    core.set_viewport(800.0, 600.0, 1.0);
    let m = core.sheet_metrics();
    assert_eq!(m.extent, (754.0, 575.0));
    assert_eq!(m.max_scroll, (0.0, 0.0));
    assert_eq!(core.scroll_to(300.0, 300.0), (0.0, 0.0));
}

#[test]
fn tamanos_de_ventana_no_validos_no_rompen_nada() {
    let (_dir, mut core) = grande();
    core.set_viewport(f32::NAN, -5.0, f32::INFINITY);
    let v = *core.view();
    assert_eq!((v.viewport_w, v.viewport_h, v.scale), (0.0, 0.0, 1.0));
    assert_eq!((v.cells_w(), v.cells_h()), (0.0, 0.0));
    // Con un área de celdas de tamaño 0 el bloque sigue teniendo al menos una celda de ancho.
    let dl = core.draw_block(v.request(0.0, 0.0)).unwrap();
    assert!(dl.n_rows >= 1 && dl.n_cols >= 1);
    // Un control más pequeño que las cabeceras tampoco da área negativa.
    core.set_viewport(20.0, 10.0, 1.0);
    assert_eq!((core.view().cells_w(), core.view().cells_h()), (0.0, 0.0));
}

#[test]
fn la_vista_arma_la_peticion_de_lo_visible() {
    let (_dir, mut core) = grande();
    core.set_viewport(1000.0, 600.0, 1.0);
    core.scroll_to(180.0, 250.0);
    let req = core.view().request(90.0, 100.0);
    assert_eq!(req.sheet, 0);
    assert_eq!((req.x0, req.y0), (180.0, 250.0));
    assert_eq!((req.w, req.h), (954.0, 575.0));
    assert_eq!((req.margin_x, req.margin_y), (90.0, 100.0));
    let dl = core.draw_block(req).unwrap();
    // Con los márgenes el bloque va de x = 90 a 1 224 y de y = 150 a 925.
    assert_eq!((dl.first_col, dl.last_col()), (2, 14));
    assert_eq!((dl.first_row, dl.last_row()), (7, 38));
    assert_eq!(dl.origin, (90.0, 150.0));
    // La fila 11 (y = 250) está dentro, con su contenido.
    let c = dl.find_cell(11, 3).unwrap();
    assert_eq!(dl.cell_text(c), "11003");
}

#[test]
fn desplazarse_sin_salir_del_margen_no_reconstruye() {
    // 100 pasos de una fila (25 px) con un margen vertical de una pantalla (575 px): el
    // bloque se rehace solo cuando el área visible se sale del precargado.
    let (_dir, mut core) = grande();
    core.set_viewport(1000.0, 600.0, 1.0);
    let mut bloques: Vec<Arc<dokumentz_sheet::DrawList>> = Vec::new();
    for paso in 0..100 {
        core.scroll_to(0.0, f64::from(paso) * 25.0);
        let dl = core.draw_block(core.view().request(0.0, 575.0)).unwrap();
        if !bloques
            .last()
            .is_some_and(|ultimo| Arc::ptr_eq(ultimo, &dl))
        {
            bloques.push(dl);
        }
    }
    println!(
        "100 pasos de una fila: {} bloques construidos",
        bloques.len()
    );
    assert!(
        (2..=6).contains(&bloques.len()),
        "se esperaban pocos bloques y hubo {}",
        bloques.len()
    );
}

#[test]
fn covers_usa_el_area_visible_y_no_los_margenes() {
    let (_dir, mut core) = grande();
    let dl = core.draw_block(peticion(0, 900.0, 2500.0)).unwrap();
    // La petición que lo construyó está cubierta; una desplazada fuera del bloque, no.
    assert!(dl.covers(&peticion(0, 900.0, 2500.0)));
    assert!(!dl.covers(&peticion(0, 900.0, 40_000.0)));
    assert!(!dl.covers(&peticion(0, 100_000.0, 2500.0)));
    // Otra hoja nunca está cubierta.
    assert!(!dl.covers(&peticion(1, 900.0, 2500.0)));
    // Un bloque que arranca en la esquina cubre también lo que queda antes del origen.
    let esquina = core.draw_block(peticion(0, 0.0, 0.0)).unwrap();
    assert!(esquina.covers(&peticion(0, -50.0, -50.0)));
}

#[test]
fn libro_nuevo_tiene_vista_inicial_razonable() {
    let core = Core::new_empty(opciones()).unwrap();
    let v = core.view();
    assert_eq!((v.sheet, v.scroll_x, v.scroll_y), (0, 0.0, 0.0));
    assert!(v.cells_w() > 0.0 && v.cells_h() > 0.0);
}
