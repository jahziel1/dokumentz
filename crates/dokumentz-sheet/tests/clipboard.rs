//! E08 (T027): portapapeles de texto tabulado (FR-010, `plan.md` §5.8; US3.2 a US3.4).
//!
//! * `copy_text`: TSV con comillas para los campos con tabulador, salto de línea o comillas;
//!   el texto es el que se ve (con formato de número).
//! * `PasteText`: normaliza (rellena a rectángulo, recorta columnas finales vacías, `\r\n`) y
//!   usa `paste_csv_string`: **una** entrada de historial, aunque pegue un 3×3.

mod common;

use dokumentz_sheet::{COL_HEADER_H, Command, Core, CoreError, Dir, ROW_HEADER_W};
use rust_xlsxwriter::Workbook;

use common::{abrir, crear_small, opciones};

fn nuevo() -> Core {
    let mut core = Core::new_empty(opciones()).expect("libro vacío");
    core.set_viewport(1046.0, 525.0, 1.0);
    core
}

fn centro(core: &mut Core, fila: i32, col: i32) -> (f32, f32) {
    let (sx, sy) = (core.view().scroll_x, core.view().scroll_y);
    let g = core.geometry(core.view().sheet).unwrap();
    let x = g.col_x(col) + g.col_w(col) / 2.0 - sx;
    let y = g.row_y(fila) + g.row_h(fila) / 2.0 - sy;
    (ROW_HEADER_W + x as f32, COL_HEADER_H + y as f32)
}

fn ir_a(core: &mut Core, fila: i32, col: i32) {
    let (x, y) = centro(core, fila, col);
    core.execute(Command::Click {
        x,
        y,
        extend: false,
    })
    .unwrap();
    assert_eq!(core.selection().active, (fila, col));
}

/// Selecciona el rango `(f0, c0)..(f1, c1)` con clic y arrastre.
fn seleccionar(core: &mut Core, f0: i32, c0: i32, f1: i32, c1: i32) {
    ir_a(core, f0, c0);
    let (x, y) = centro(core, f1, c1);
    core.execute(Command::DragTo { x, y }).unwrap();
    assert_eq!(core.selection().range, (f0, c0, f1, c1));
}

fn escribir(core: &mut Core, fila: i32, col: i32, texto: &str) {
    core.set_cell_input(0, fila, col, texto).unwrap();
}

fn contenido(core: &Core, fila: i32, col: i32) -> String {
    core.cell_input(0, fila, col).unwrap()
}

/// Contenido de un rectángulo, fila a fila.
fn rectangulo(core: &Core, f0: i32, c0: i32, f1: i32, c1: i32) -> Vec<Vec<String>> {
    (f0..=f1)
        .map(|f| (c0..=c1).map(|c| contenido(core, f, c)).collect())
        .collect()
}

// ----------------------------------------------------------------------------------------
// Copiar
// ----------------------------------------------------------------------------------------

#[test]
fn copiar_pone_comillas_a_los_campos_con_tabulador_salto_de_linea_y_comillas() {
    let mut core = nuevo();
    escribir(&mut core, 1, 1, "a\tb");
    escribir(&mut core, 1, 2, "dice \"hola\"");
    escribir(&mut core, 2, 1, "linea1\nlinea2");
    escribir(&mut core, 2, 2, "normal");
    escribir(&mut core, 2, 3, "=1+1");
    seleccionar(&mut core, 1, 1, 2, 3);
    assert_eq!(
        core.copy_text().unwrap(),
        "\"a\tb\"\t\"dice \"\"hola\"\"\"\t\n\"linea1\nlinea2\"\tnormal\t2"
    );
}

#[test]
fn copiar_usa_el_texto_que_se_ve_con_su_formato() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    core.set_viewport(1046.0, 525.0, 1.0);
    core.recalculate();
    seleccionar(&mut core, 1, 1, 2, 4);
    assert_eq!(
        core.copy_text().unwrap(),
        "Producto\tPrecio\tFecha\tDoble\nManzana\t1,234.50\t2024-03-05\t2469"
    );
}

#[test]
fn copiar_una_celda_y_una_celda_vacia() {
    let mut core = nuevo();
    escribir(&mut core, 3, 3, "solo");
    ir_a(&mut core, 3, 3);
    assert_eq!(core.copy_text().unwrap(), "solo");
    ir_a(&mut core, 4, 4);
    assert_eq!(core.copy_text().unwrap(), "");
}

#[test]
fn copiar_un_rango_normal_conserva_las_celdas_vacias_del_final() {
    let mut core = nuevo();
    escribir(&mut core, 1, 1, "x");
    seleccionar(&mut core, 1, 1, 3, 3);
    assert_eq!(core.copy_text().unwrap(), "x\t\t\n\t\t\n\t\t");
}

#[test]
fn copiar_columnas_o_filas_enteras_se_limita_al_rango_con_datos() {
    let mut core = nuevo();
    for (i, v) in ["a", "b", "c"].into_iter().enumerate() {
        escribir(&mut core, i as i32 + 1, 1, v);
        escribir(&mut core, i as i32 + 1, 2, &format!("{v}{v}"));
    }
    // Clic en la cabecera de la columna A: columna entera (un millón de filas).
    let x = centro(&mut core, 1, 1).0;
    core.execute(Command::Click {
        x,
        y: COL_HEADER_H / 2.0,
        extend: false,
    })
    .unwrap();
    assert!(core.selection().is_entire_cols());
    assert_eq!(core.copy_text().unwrap(), "a\nb\nc");

    // Clic en la cabecera de la fila 2: fila entera.
    let y = centro(&mut core, 2, 1).1;
    core.execute(Command::Click {
        x: ROW_HEADER_W / 2.0,
        y,
        extend: false,
    })
    .unwrap();
    assert!(core.selection().is_entire_rows());
    assert_eq!(core.copy_text().unwrap(), "b\tbb");

    // La esquina: toda la hoja.
    core.execute(Command::Click {
        x: 1.0,
        y: 1.0,
        extend: false,
    })
    .unwrap();
    assert_eq!(core.copy_text().unwrap(), "a\taa\nb\tbb\nc\tcc");
}

// ----------------------------------------------------------------------------------------
// Pegar
// ----------------------------------------------------------------------------------------

#[test]
fn pegar_un_3x3_en_medio_de_la_hoja_y_deshacerlo_en_un_solo_paso() {
    let mut core = nuevo();
    escribir(&mut core, 1, 1, "antes");
    escribir(&mut core, 6, 6, "se pisa");
    ir_a(&mut core, 5, 5);
    let version = core.data_version();
    core.execute(Command::PasteText("1\t2\t3\n4\t5\t6\n7\t8\t9".into()))
        .unwrap();
    assert!(core.data_version() > version);
    assert!(core.is_dirty());
    assert_eq!(
        rectangulo(&core, 5, 5, 7, 7),
        [["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"]]
    );
    // Lo pegado queda seleccionado, con la celda activa en su esquina superior izquierda.
    assert_eq!(core.selection().range, (5, 5, 7, 7));
    assert_eq!(core.selection().active, (5, 5));

    // Un solo Undo quita las nueve celdas y devuelve lo que había debajo.
    core.execute(Command::Undo).unwrap();
    assert_eq!(rectangulo(&core, 5, 5, 7, 7)[0], ["", "", ""]);
    assert_eq!(contenido(&core, 6, 6), "se pisa", "lo pisado vuelve");
    assert_eq!(contenido(&core, 1, 1), "antes");
    // Los dos `escribir` previos siguen en el historial: el pegado fue UNA entrada.
    core.undo().unwrap();
    assert_eq!(
        contenido(&core, 6, 6),
        "",
        "el siguiente undo es la escritura previa"
    );
    core.redo().unwrap();
    core.redo().unwrap();
    assert_eq!(
        contenido(&core, 7, 7),
        "9",
        "y Redo lo vuelve a pegar entero"
    );
}

#[test]
fn pegar_celdas_con_tabulador_comillas_y_salto_de_linea() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    core.paste_text("a\t\"b\tc\"\t\"linea1\nlinea2\"\t\"dice \"\"hola\"\"\"\r\n1\t2\t3\t=1+1\r\n")
        .unwrap();
    assert_eq!(contenido(&core, 2, 2), "a");
    assert_eq!(contenido(&core, 2, 3), "b\tc");
    assert_eq!(contenido(&core, 2, 4), "linea1\nlinea2");
    assert_eq!(contenido(&core, 2, 5), "dice \"hola\"");
    assert_eq!(contenido(&core, 3, 4), "3");
    assert_eq!(
        core.cell_text(0, 3, 5).unwrap(),
        "2",
        "=1+1 se evalúa como al teclear"
    );
    assert_eq!(core.selection().range, (2, 2, 3, 5));
    // El pegado no ajusta la altura de las filas con saltos de línea (teclear sí lo hace).
    let g = core.geometry(0).unwrap();
    assert_eq!(g.row_h(2), g.row_h(3));
}

#[test]
fn pegar_rellena_las_filas_cortas_hasta_un_rectangulo() {
    // IronCalc 0.9.1 descarta en silencio las filas más cortas que la primera.
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    core.paste_text("a\tb\tc\n1\t2\nx\ty\tz\n").unwrap();
    assert_eq!(
        rectangulo(&core, 1, 1, 3, 3),
        [["a", "b", "c"], ["1", "2", ""], ["x", "y", "z"]]
    );
    assert_eq!(core.selection().range, (1, 1, 3, 3));
}

#[test]
fn pegar_recorta_las_columnas_finales_vacias_y_no_borra_lo_de_la_derecha() {
    // Los campos vacíos finales borrarían las celdas de la derecha.
    let mut core = nuevo();
    escribir(&mut core, 1, 4, "conservar");
    escribir(&mut core, 2, 4, "conservar 2");
    ir_a(&mut core, 1, 1);
    core.paste_text("a\tb\t\t\n1\t2\t\t\n").unwrap();
    assert_eq!(contenido(&core, 1, 4), "conservar");
    assert_eq!(contenido(&core, 2, 4), "conservar 2");
    assert_eq!(core.selection().range, (1, 1, 2, 2));
    // Pero una columna que tiene algo en alguna fila no se recorta: ahí el vacío es de verdad.
    core.paste_text("a\tb\t\t\n1\t2\t\tz\n").unwrap();
    assert_eq!(
        contenido(&core, 1, 4),
        "",
        "la celda vacía de la columna 4 sí pisa"
    );
    assert_eq!(contenido(&core, 2, 4), "z");
}

#[test]
fn pegar_acepta_fin_de_linea_de_excel_y_filas_vacias_intermedias() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    // Excel/Windows: CRLF y un salto final.
    core.paste_text("x\ty\r\nz\tw\r\n").unwrap();
    assert_eq!(rectangulo(&core, 1, 1, 2, 2), [["x", "y"], ["z", "w"]]);
    assert!(
        contenido(&core, 3, 1).is_empty(),
        "el salto final no crea una fila"
    );
    // Una línea vacía dentro del texto es una fila vacía (el lector de IronCalc la saltaría).
    ir_a(&mut core, 5, 1);
    core.paste_text("a\n\nb\n\n\n").unwrap();
    assert_eq!(contenido(&core, 5, 1), "a");
    assert_eq!(contenido(&core, 6, 1), "");
    assert_eq!(contenido(&core, 7, 1), "b");
    assert_eq!(
        contenido(&core, 8, 1),
        "",
        "las filas vacías del final no cuentan"
    );
    assert_eq!(core.selection().range, (5, 1, 7, 1));
}

#[test]
fn pegar_empieza_en_la_esquina_superior_izquierda_de_la_seleccion() {
    let mut core = nuevo();
    // Selección de 2×2 con la celda activa abajo a la derecha (arrastre hacia arriba).
    ir_a(&mut core, 4, 4);
    let (x, y) = centro(&mut core, 3, 3);
    core.execute(Command::DragTo { x, y }).unwrap();
    assert_eq!(core.selection().range, (3, 3, 4, 4));
    core.paste_text("p\tq").unwrap();
    assert_eq!(contenido(&core, 3, 3), "p");
    assert_eq!(contenido(&core, 3, 4), "q");
    assert_eq!(contenido(&core, 4, 4), "");
}

#[test]
fn pegar_texto_sin_contenido_no_hace_nada() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    let version = core.data_version();
    for vacio in ["", "\n", "\t\t\n\n"] {
        core.paste_text(vacio).unwrap();
    }
    assert_eq!(core.data_version(), version);
    assert!(!core.is_dirty());
    assert!(!core.can_undo());
}

#[test]
fn pegar_donde_no_cabe_es_un_error_y_no_pega_nada() {
    let mut core = nuevo();
    // Última fila de la hoja: un bloque de dos filas no cabe.
    core.execute(Command::Home { ctrl: true }).unwrap();
    core.execute(Command::Edge {
        dir: Dir::Down,
        extend: false,
    })
    .unwrap();
    assert_eq!(core.selection().active.0, 1_048_576);
    let err = core.paste_text("a\nb").unwrap_err();
    assert!(matches!(err, CoreError::Motor(_)), "{err:?}");
    assert!(err.to_string().contains("no cabe"), "{err}");
    assert!(!core.is_dirty());
    assert!(!core.can_undo());
    assert_eq!(contenido(&core, 1_048_576, 1), "");
    // Una sola fila sí cabe.
    core.paste_text("ultima").unwrap();
    assert_eq!(contenido(&core, 1_048_576, 1), "ultima");
}

#[test]
fn pegar_lejos_del_rango_usado_amplia_la_extension_de_la_hoja() {
    let mut core = nuevo();
    escribir(&mut core, 1, 1, "x");
    assert_eq!(core.sheet_metrics().used_rows, 1);
    core.execute(Command::Move {
        dir: Dir::Down,
        extend: false,
    })
    .unwrap();
    for _ in 0..40 {
        core.execute(Command::Move {
            dir: Dir::Down,
            extend: false,
        })
        .unwrap();
    }
    let fila = core.selection().active.0;
    core.paste_text("1\t2\t3\n4\t5\t6").unwrap();
    let m = core.sheet_metrics();
    assert_eq!(m.used_rows, fila + 1);
    assert_eq!(m.used_cols, 3);
}

#[test]
fn copiar_y_pegar_ida_y_vuelta_conserva_los_textos_raros() {
    let mut core = nuevo();
    let raros = [
        "a\tb",
        "dice \"hola\"",
        "linea1\nlinea2",
        "\"entrecomillado\"",
        "con ñ y acentos áéíóú",
        "  espacios  ",
        "",
        "ultimo",
    ];
    for (i, texto) in raros.iter().enumerate() {
        escribir(&mut core, 1 + (i as i32 / 4), 1 + (i as i32 % 4), texto);
    }
    seleccionar(&mut core, 1, 1, 2, 4);
    let tsv = core.copy_text().unwrap();
    ir_a(&mut core, 5, 1);
    core.paste_text(&tsv).unwrap();
    assert_eq!(rectangulo(&core, 5, 1, 6, 4), rectangulo(&core, 1, 1, 2, 4));
    // Y el TSV copiado de lo pegado es el mismo.
    assert_eq!(core.selection().range, (5, 1, 6, 4));
    assert_eq!(core.copy_text().unwrap(), tsv);
}

#[test]
fn pegar_en_otra_hoja_va_a_la_hoja_visible() {
    let mut wb = Workbook::new();
    wb.add_worksheet().set_name("Una").unwrap();
    wb.add_worksheet().set_name("Otra").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("dos.xlsx");
    wb.save(&ruta).unwrap();
    let mut core = abrir(&ruta);
    core.select_sheet(1).unwrap();
    core.paste_text("x\ty").unwrap();
    assert_eq!(core.cell_text(1, 1, 2).unwrap(), "y");
    assert_eq!(core.cell_text(0, 1, 2).unwrap(), "");
    assert_eq!(core.selection().sheet, 1);
}
