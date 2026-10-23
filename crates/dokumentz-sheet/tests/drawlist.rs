//! T017: construcción de la lista de dibujo desde el motor.
//!
//! Las celdas de `small.xlsx` están descritas en `tests/common/mod.rs`. Donde se puede, el
//! resultado se compara con un oráculo independiente: el mismo archivo abierto directamente con
//! IronCalc y leído con sus propios getters.

mod common;

use std::sync::Arc;

use dokumentz_sheet::drawlist::{MAX_BLOCK_COLS, MAX_BLOCK_ROWS, col_label};
use dokumentz_sheet::{Align, Core, CoreError, DrawList, DrawRequest, Kind, Rgba, VAlign};
use ironcalc_base::UserModel;

use common::{TEXTO_LARGO, abrir, crear_formatos, crear_grande, crear_small, crear_tamanos};

/// Pide un bloque que cubre exactamente `[x0, x0 + w] x [y0, y0 + h]`, sin márgenes.
fn pedir(core: &mut Core, x0: f64, y0: f64, w: f32, h: f32) -> Arc<DrawList> {
    core.draw_block(DrawRequest {
        sheet: 0,
        x0,
        y0,
        w,
        h,
        margin_x: 0.0,
        margin_y: 0.0,
    })
    .expect("draw_block")
}

/// El bloque que cubre A1:E7 de la hoja 0 (5 columnas de 90 px y 7 de 25 px; sin mover).
fn bloque_inicial(core: &mut Core) -> Arc<DrawList> {
    pedir(core, 0.0, 0.0, 5.0 * 90.0 - 0.5, 7.0 * 25.0 - 0.5)
}

fn oraculo(ruta: &std::path::Path) -> UserModel<'static> {
    UserModel::from_model(
        ironcalc::import::load_from_xlsx(ruta.to_str().unwrap(), "en", "UTC", "en").unwrap(),
    )
}

#[test]
fn el_numero_de_celdas_coincide_con_las_celdas_con_contenido() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let mut core = abrir(&ruta);
    let dl = bloque_inicial(&mut core);
    assert_eq!((dl.first_row, dl.first_col), (1, 1));
    assert_eq!((dl.n_rows, dl.n_cols), (7, 5));

    // Valor esperado, contado a mano sobre el fixture: fila 1: 4, fila 2: 4, fila 3: 3 (A, B, D),
    // fila 4: 2 (A con relleno y C solo relleno), fila 5: 1, fila 6: 2 (A y B). Total 16.
    assert_eq!(dl.cells.len(), 16);

    // Y contra el oráculo: celdas con texto o con relleno resuelto en el rectángulo.
    let um = oraculo(&ruta);
    let mut esperadas = 0;
    for f in 1..=7 {
        for c in 1..=5 {
            let texto = um.get_formatted_cell_value(0, f, c).unwrap();
            let estilo = um.get_cell_style(0, f, c).unwrap();
            let con_relleno = !um.resolve_color(&estilo.fill.color).is_empty();
            if !texto.is_empty() || con_relleno {
                esperadas += 1;
            }
        }
    }
    assert_eq!(dl.cells.len(), esperadas);
    assert_eq!(core.drawn_cells_last(), 16);

    // Un área menor: solo las filas 1-2 y las columnas A-D (8 celdas con dato). Con el mismo
    // `Core` saldría el bloque anterior de la caché (más grande y suficiente); se abre otro.
    let mut otro = abrir(&ruta);
    let parte = pedir(&mut otro, 0.0, 0.0, 4.0 * 90.0 - 0.5, 2.0 * 25.0 - 0.5);
    assert_eq!((parte.n_rows, parte.n_cols), (2, 4));
    assert_eq!(parte.cells.len(), 8);
}

#[test]
fn textos_formateados_no_son_numeros_de_serie() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let dl = bloque_inicial(&mut core);
    let texto = |f: i32, c: i32| -> String {
        let cel = dl
            .find_cell(f, c)
            .unwrap_or_else(|| panic!("falta ({f}, {c})"));
        dl.cell_text(cel).to_string()
    };
    assert_eq!(texto(2, 2), "1,234.50", "formato #,##0.00");
    assert_eq!(texto(2, 3), "2024-03-05", "una fecha no sale como 45356");
    assert_eq!(texto(2, 4), "2469");
    assert_eq!(texto(6, 2), "7.12");
    assert_eq!(texto(3, 4), "TRUE");
    assert_eq!(texto(5, 1), TEXTO_LARGO);
    // Los tipos.
    let tipo = |f: i32, c: i32| dl.find_cell(f, c).unwrap().kind;
    assert_eq!(tipo(1, 1), Kind::Texto);
    assert_eq!(tipo(2, 2), Kind::Numero);
    assert_eq!(
        tipo(2, 3),
        Kind::Numero,
        "las fechas son números con formato"
    );
    assert_eq!(tipo(2, 4), Kind::Numero);
    assert_eq!(tipo(3, 4), Kind::Bool);
    assert_eq!(tipo(4, 3), Kind::SoloEstilo);
    // Una celda sin dato ni relleno no está en la lista.
    assert!(dl.find_cell(3, 3).is_none());
    assert!(dl.find_cell(7, 5).is_none());
}

#[test]
fn negrita_cursiva_color_relleno_y_alineacion_llegan_al_estilo() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let dl = bloque_inicial(&mut core);
    let est = |f: i32, c: i32| {
        *dl.styles
            .get(usize::from(dl.find_cell(f, c).unwrap().style))
            .unwrap()
    };

    // A2: sin formato.
    let normal = est(2, 1);
    assert!(!normal.bold && !normal.italic && !normal.underline && !normal.strike && !normal.wrap);
    assert_eq!(normal.fg, Rgba::NEGRO, "color de letra por defecto: negro");
    assert_eq!(normal.fill, None, "sin relleno");
    assert_eq!(
        (normal.h_align, normal.v_align),
        (Align::Izq, VAlign::Abajo)
    );

    // A1 y B1: negrita; B1 además alineado a la derecha de forma explícita.
    assert!(est(1, 1).bold);
    assert_eq!(est(1, 1).h_align, Align::Izq);
    assert!(est(1, 2).bold);
    assert_eq!(est(1, 2).h_align, Align::Der);

    // A3: cursiva, subrayado y letra roja.
    let roja = est(3, 1);
    assert!(roja.italic && roja.underline && !roja.bold);
    assert_eq!(roja.fg, Rgba::rgb(0xFF, 0x00, 0x00));
    assert_eq!(roja.fg.componentes(), (255, 0, 0));

    // A4: relleno amarillo y centrado. C4: solo relleno verde.
    let amarilla = est(4, 1);
    assert_eq!(amarilla.fill, Some(Rgba::rgb(0xFF, 0xFF, 0x00)));
    assert_eq!(amarilla.h_align, Align::Centro);
    let verde = dl.find_cell(4, 3).unwrap();
    assert_eq!(verde.kind, Kind::SoloEstilo);
    assert_eq!(dl.cell_text(verde), "");
    assert_eq!(est(4, 3).fill, Some(Rgba::rgb(0x00, 0xFF, 0x00)));

    // A6: ajuste de línea, tachado y letra de 14 pt (14 x 1,5625 px).
    let ajustada = est(6, 1);
    assert!(ajustada.wrap && ajustada.strike);
    assert!(
        (ajustada.font_px - 21.875).abs() < 1e-4,
        "{}",
        ajustada.font_px
    );
    // La letra por defecto de Excel es de 11 pt.
    assert!(
        (normal.font_px - 11.0 * 1.5625).abs() < 1e-4,
        "{}",
        normal.font_px
    );
}

#[test]
fn los_estilos_se_deduplican() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_grande(dir.path(), 60, 12));
    let dl = pedir(&mut core, 0.0, 0.0, 12.0 * 90.0 - 0.5, 60.0 * 25.0 - 0.5);
    assert_eq!(dl.cells.len(), 60 * 12);
    // Encabezados (texto, a la izquierda) y números (a la derecha): dos estilos, no 720.
    assert_eq!(dl.styles.len(), 2, "{:?}", dl.styles);
    for (i, a) in dl.styles.iter().enumerate() {
        for b in &dl.styles[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert!(
        dl.cells
            .iter()
            .all(|c| usize::from(c.style) < dl.styles.len())
    );
    // Las celdas salen ordenadas por fila y luego por columna.
    assert!(
        dl.cells
            .windows(2)
            .all(|p| (p[0].row_off, p[0].col_off) < (p[1].row_off, p[1].col_off))
    );
}

#[test]
fn la_alineacion_general_se_resuelve_por_tipo() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    // Un error de fórmula en G1 (la columna G está fuera de A1:E7: se agranda el bloque).
    core.set_cell_input(0, 1, 7, "=1/0").unwrap();
    let dl = pedir(&mut core, 0.0, 0.0, 7.0 * 90.0 - 0.5, 7.0 * 25.0 - 0.5);
    let alin = |f: i32, c: i32| {
        let cel = dl.find_cell(f, c).unwrap();
        (cel.kind, dl.styles[usize::from(cel.style)].h_align)
    };
    assert_eq!(
        alin(2, 1),
        (Kind::Texto, Align::Izq),
        "texto a la izquierda"
    );
    assert_eq!(
        alin(3, 2),
        (Kind::Numero, Align::Der),
        "número a la derecha"
    );
    assert_eq!(
        alin(2, 3),
        (Kind::Numero, Align::Der),
        "fecha (número) a la derecha"
    );
    assert_eq!(alin(3, 4), (Kind::Bool, Align::Centro), "lógico al centro");
    assert_eq!(alin(1, 7), (Kind::Error, Align::Centro), "error al centro");
    let error = dl.find_cell(1, 7).unwrap();
    assert_eq!(dl.cell_text(error), "#DIV/0!");
    // Una alineación explícita manda sobre la general: B1 es texto pero está a la derecha.
    assert_eq!(alin(1, 2), (Kind::Texto, Align::Der));
}

#[test]
fn el_texto_izquierdo_desborda_sobre_las_columnas_vacias() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let spill = |dl: &DrawList, f: i32, c: i32| dl.find_cell(f, c).unwrap().spill_cols;
    let dl = bloque_inicial(&mut core);
    assert_eq!(
        spill(&dl, 5, 1),
        8,
        "A5: todo a su derecha está vacío (tope 8)"
    );
    assert_eq!(spill(&dl, 2, 1), 0, "A2: B2 tiene dato");
    assert_eq!(spill(&dl, 3, 1), 0, "A3: B3 tiene dato");
    assert_eq!(spill(&dl, 4, 1), 0, "A4: centrado, no desborda");
    assert_eq!(spill(&dl, 6, 1), 0, "A6: con ajuste de línea, no desborda");
    assert_eq!(spill(&dl, 2, 2), 0, "un número no desborda");
    assert_eq!(spill(&dl, 1, 2), 0, "B1: alineado a la derecha");

    // Un dato en D5 limita el desborde de A5 a B5 y C5; y el bloque nuevo lo refleja.
    let version = core.data_version();
    core.set_cell_input(0, 5, 4, "tope").unwrap();
    assert!(core.data_version() > version);
    let dl = bloque_inicial(&mut core);
    assert_eq!(spill(&dl, 5, 1), 2);
}

#[test]
fn etiquetas_de_cabecera() {
    assert_eq!(col_label(1), "A");
    assert_eq!(col_label(26), "Z");
    assert_eq!(col_label(27), "AA");
    assert_eq!(col_label(52), "AZ");
    assert_eq!(col_label(702), "ZZ");
    assert_eq!(col_label(703), "AAA");
    assert_eq!(col_label(16_384), "XFD");

    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let dl = bloque_inicial(&mut core);
    let cols: Vec<&str> = (0..5).map(|i| dl.col_label(i)).collect();
    let filas: Vec<&str> = (0..7).map(|i| dl.row_label(i)).collect();
    assert_eq!(cols, ["A", "B", "C", "D", "E"]);
    assert_eq!(filas, ["1", "2", "3", "4", "5", "6", "7"]);

    // Un bloque que empieza en la columna AA (27) y en la fila 1 000.
    let x = 26.0 * 90.0;
    let y = 999.0 * 25.0;
    let dl = pedir(&mut core, x, y, 3.0 * 90.0 - 0.5, 2.0 * 25.0 - 0.5);
    assert_eq!((dl.first_row, dl.first_col), (1000, 27));
    assert_eq!(dl.origin, (x, y));
    let cols: Vec<&str> = (0..3).map(|i| dl.col_label(i)).collect();
    assert_eq!(cols, ["AA", "AB", "AC"]);
    assert_eq!(dl.row_label(0), "1000");
    assert_eq!(dl.row_label(1), "1001");
}

#[test]
fn las_aristas_reflejan_tamanos_propios_y_las_celdas_ocultas_no_se_dibujan() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_tamanos(dir.path()));
    let dl = pedir(&mut core, 0.0, 0.0, 800.0, 300.0);
    // Columnas: A 90, B ~186,4, C 90, D oculta (0), E 90...
    let ancho = |i: usize| f64::from(dl.col_edges[i + 1] - dl.col_edges[i]);
    assert!((ancho(0) - 90.0).abs() < 1e-3);
    assert!((ancho(1) - 186.398_437_5).abs() < 1e-3, "{}", ancho(1));
    assert!((ancho(2) - 90.0).abs() < 1e-3);
    assert_eq!(ancho(3), 0.0, "columna D oculta");
    assert!((ancho(4) - 90.0).abs() < 1e-3);
    // Filas: 1: 25; 2: 46,875; 3: 25; 4: oculta (0); 5: 25.
    let alto = |i: usize| f64::from(dl.row_edges[i + 1] - dl.row_edges[i]);
    assert!((alto(0) - 25.0).abs() < 1e-3);
    assert!((alto(1) - 46.875).abs() < 1e-3);
    assert_eq!(alto(3), 0.0, "fila 4 oculta");
    assert_eq!(dl.col_edges.len(), dl.n_cols as usize + 1);
    assert_eq!(dl.row_edges.len(), dl.n_rows as usize + 1);
    // Los datos: A1, B2, C3, E5 y A6 se dibujan; D4 está en una columna y fila ocultas: no.
    for (f, c) in [(1, 1), (2, 2), (3, 3), (5, 5), (6, 1)] {
        assert!(dl.find_cell(f, c).is_some(), "falta ({f}, {c})");
    }
    assert!(dl.find_cell(4, 4).is_none());
    assert_eq!(dl.cells.len(), 5);
    // Las etiquetas siguen contando las ocultas (la cabecera salta de C a E por el ancho 0).
    assert_eq!(dl.col_label(3), "D");
}

#[test]
fn el_formato_de_fila_y_columna_entera_se_hereda() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_formatos(dir.path());
    let mut core = abrir(&ruta);
    // Bloque A1:H5. La fila 3 tiene entrada propia y mide 15 pt x 1,5625 = 23,4375 px (regla del
    // motor para las filas con entrada), así que el alto se toma de la geometría.
    let alto = core.geometry(0).unwrap().row_y(6) - 0.5;
    let dl = pedir(&mut core, 0.0, 0.0, 8.0 * 90.0 - 0.5, alto as f32);
    assert_eq!((dl.n_rows, dl.n_cols), (5, 8));
    let relleno = |f: i32, c: i32| -> Option<Rgba> {
        dl.find_cell(f, c)
            .and_then(|cel| dl.styles[usize::from(cel.style)].fill)
    };
    let azul = Some(Rgba::rgb(0x00, 0x00, 0xFF));
    let gris = Some(Rgba::rgb(0xC0, 0xC0, 0xC0));
    assert_eq!(
        relleno(3, 2),
        azul,
        "B3: sin dato, hereda el azul de la fila 3"
    );
    assert_eq!(relleno(3, 8), azul, "H3");
    assert_eq!(relleno(1, 6), gris, "F1: hereda el gris de la columna F");
    assert_eq!(relleno(5, 6), gris, "F5");
    assert_eq!(
        relleno(3, 6),
        azul,
        "F3: la fila manda sobre la columna (regla del motor)"
    );
    assert_eq!(relleno(2, 2), None, "B2: ni fila ni columna con formato");
    assert_eq!(relleno(3, 1), azul, "A3: el dato conserva su propio estilo");
    assert_eq!(dl.cell_text(dl.find_cell(3, 1).unwrap()), "en fila azul");
    assert_eq!(dl.cell_text(dl.find_cell(1, 1).unwrap()), "dato");

    // Oráculo independiente: dónde hay relleno según el motor.
    let um = oraculo(&ruta);
    let mut con_relleno = 0;
    for f in 1..=5 {
        for c in 1..=8 {
            let est = um.get_cell_style(0, f, c).unwrap();
            let esperado = Rgba::from_hex(&um.resolve_color(&est.fill.color));
            let texto = !um.get_formatted_cell_value(0, f, c).unwrap().is_empty();
            assert_eq!(relleno(f, c), esperado, "relleno en ({f}, {c})");
            assert_eq!(
                dl.find_cell(f, c).is_some(),
                esperado.is_some() || texto,
                "presencia de ({f}, {c})"
            );
            con_relleno += usize::from(esperado.is_some() || texto);
        }
    }
    assert_eq!(dl.cells.len(), con_relleno);
    assert_eq!(
        dl.cells.len(),
        13,
        "8 de la fila 3 + 4 de la columna F + A1"
    );
}

#[test]
fn una_hoja_inexistente_es_error_y_los_extremos_se_acotan() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    let req = DrawRequest {
        sheet: 3,
        x0: 0.0,
        y0: 0.0,
        w: 100.0,
        h: 100.0,
        margin_x: 0.0,
        margin_y: 0.0,
    };
    assert_eq!(core.draw_block(req).err(), Some(CoreError::HojaInvalida(3)));

    // Pasado el borde derecho e inferior de la hoja: se acota a la última fila y columna.
    let dl = pedir(&mut core, 1e12, 1e12, 500.0, 500.0);
    assert_eq!(dl.last_col(), 16_384);
    assert_eq!(dl.last_row(), 1_048_576);
    assert_eq!((dl.n_cols, dl.n_rows), (1, 1));
    assert!(dl.cells.is_empty());
    // Antes del origen: se acota a la esquina.
    let dl = pedir(&mut core, -500.0, -500.0, 100.0, 100.0);
    assert_eq!((dl.first_row, dl.first_col), (1, 1));
    // Una petición de tamaño absurdo se acota a los topes del bloque.
    let dl = pedir(&mut core, 0.0, 0.0, 1e9, 1e9);
    assert_eq!(dl.n_cols as i32, MAX_BLOCK_COLS);
    assert_eq!(dl.n_rows as i32, MAX_BLOCK_ROWS);
}

#[test]
fn los_margenes_ensanchan_el_bloque() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_grande(dir.path(), 200, 30));
    // Área visible: columnas 11-13 y filas 51-53, con margen de una columna y dos filas.
    let (x0, y0) = (10.0 * 90.0, 50.0 * 25.0);
    let dl = core
        .draw_block(DrawRequest {
            sheet: 0,
            x0,
            y0,
            w: 3.0 * 90.0 - 0.5,
            h: 3.0 * 25.0 - 0.5,
            margin_x: 90.0,
            margin_y: 50.0,
        })
        .unwrap();
    assert_eq!(dl.first_col, 10);
    assert_eq!(dl.last_col(), 14);
    assert_eq!(dl.first_row, 49);
    assert_eq!(dl.last_row(), 55);
    assert_eq!(dl.origin, (9.0 * 90.0, 48.0 * 25.0));
    // El contenido de una celda de ese bloque: fila 51, columna 11 = 51 * 1000 + 11.
    let c = dl.find_cell(51, 11).unwrap();
    assert_eq!(dl.cell_text(c), "51011");
}

// ---------------------------------------------------------------------------
// Medición de la construcción de un bloque (no es un requisito de T017; sirve de línea base).
// ---------------------------------------------------------------------------

#[test]
#[ignore = "medición: correr con --release"]
fn medir_construccion_de_un_bloque_de_pantalla() {
    if cfg!(debug_assertions) {
        panic!("no se mide con `debug` (regla 6): use --release");
    }
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_grande(dir.path(), 400, 40));
    // Una pantalla de 1440 x 900 con margen de +-1 alto y +-1/2 ancho (plan §5.4): ~2 800 celdas.
    let (w, h) = (1440.0 - 46.0, 900.0 - 25.0);
    let mut tiempos = Vec::new();
    let mut celdas = 0;
    for i in 0..15 {
        // Subir la versión de datos invalida la caché: cada vuelta construye un bloque nuevo
        // (la geometría de la hoja sí se conserva, como en uso real).
        core.recalculate();
        let t = std::time::Instant::now();
        let dl = core
            .draw_block(DrawRequest {
                sheet: 0,
                x0: 90.0 * f64::from(i),
                y0: 25.0 * f64::from(i) * 7.0,
                w: w as f32,
                h: h as f32,
                margin_x: (w / 2.0) as f32,
                margin_y: h as f32,
            })
            .unwrap();
        tiempos.push(t.elapsed().as_secs_f64() * 1000.0);
        celdas = dl.cells.len();
    }
    tiempos.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mediana = tiempos[tiempos.len() / 2];
    println!("build_block: {celdas} celdas, mediana de 15 = {mediana:.2} ms");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/dev");
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::json!({
        "tarea": "T017",
        "perfil": "release",
        "celdas_en_el_bloque": celdas,
        "build_block_ms_mediana_de_15": (mediana * 100.0).round() / 100.0,
        "maquina": "Apple M1, 16 GB, macOS (ver research.md)",
    });
    std::fs::write(
        dir.join("T017.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();
}
