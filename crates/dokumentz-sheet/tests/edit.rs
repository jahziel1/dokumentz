//! E08 (T023): edición de celdas (`BeginEdit`, `SetEditText`, `CommitEdit`, `CancelEdit`) y
//! regla de avance de Office. Cubre los escenarios 1 a 3 de US2 (`spec.md`):
//!
//! 1. Escribir y confirmar guarda el valor; Intro baja a la celda siguiente y Tab pasa a la de la
//!    derecha (Mayús invierte).
//! 2. Editar una celda con contenido abre la edición con ese contenido; Esc descarta y conserva
//!    el valor anterior.
//! 3. La celda y la barra de fórmulas muestran el mismo texto mientras se escribe.
//!
//! Más celdas con texto, fórmula, error y texto largo, el avance dentro de un rango, y la
//! invalidación de la geometría cuando la edición amplía el rango usado o la altura de una fila.

mod common;

use dokumentz_sheet::{
    Advance, COL_HEADER_H, Command, Core, CoreError, DrawRequest, EditMode, ROW_HEADER_W,
};

use common::opciones;

fn nuevo() -> Core {
    Core::new_empty(opciones()).expect("libro vacío")
}

fn activa(core: &Core) -> (i32, i32) {
    core.selection().active
}

/// Punto del control en el centro de la celda `(fila, col)`; la celda debe estar a la vista.
fn centro(core: &mut Core, fila: i32, col: i32) -> (f32, f32) {
    let (sx, sy) = (core.view().scroll_x, core.view().scroll_y);
    let g = core.geometry(core.view().sheet).unwrap();
    let x = g.col_x(col) + g.col_w(col) / 2.0 - sx;
    let y = g.row_y(fila) + g.row_h(fila) / 2.0 - sy;
    (ROW_HEADER_W + x as f32, COL_HEADER_H + y as f32)
}

/// Deja la celda activa en `(fila, col)` con un clic.
fn ir_a(core: &mut Core, fila: i32, col: i32) {
    let (x, y) = centro(core, fila, col);
    core.execute(Command::Click {
        x,
        y,
        extend: false,
    })
    .unwrap();
    assert_eq!(activa(core), (fila, col), "el clic debía llegar a la celda");
}

/// Escribe `texto` en la celda activa como lo haría la interfaz: empezar a teclear y confirmar.
fn escribir(core: &mut Core, texto: &str, avance: Advance) {
    core.execute(Command::BeginEdit {
        mode: EditMode::Replace,
        seed: None,
    })
    .unwrap();
    core.execute(Command::SetEditText(texto.to_string()))
        .unwrap();
    core.execute(Command::CommitEdit {
        text: texto.to_string(),
        advance: avance,
    })
    .unwrap();
}

fn valor(core: &Core, fila: i32, col: i32) -> String {
    core.cell_text(0, fila, col).unwrap()
}

fn contenido(core: &Core, fila: i32, col: i32) -> String {
    core.cell_input(0, fila, col).unwrap()
}

// ----------------------------------------------------------------------------------------
// US2.1 — valor guardado y avance
// ----------------------------------------------------------------------------------------

#[test]
fn escenario_1_intro_guarda_el_valor_y_la_seleccion_baja() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    escribir(&mut core, "hola", Advance::enter(false));
    assert_eq!(valor(&core, 2, 2), "hola");
    assert_eq!(activa(&core), (3, 2));
    assert!(
        core.edit_state().is_none(),
        "la sesión se cierra al confirmar"
    );
    assert!(core.is_dirty());
}

#[test]
fn escenario_1_tab_pasa_a_la_celda_de_la_derecha() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    escribir(&mut core, "42", Advance::tab(false));
    assert_eq!(valor(&core, 2, 2), "42");
    assert_eq!(activa(&core), (2, 3));
}

#[test]
fn mayus_invierte_el_sentido_del_avance() {
    assert_eq!(Advance::enter(false), Advance::Down);
    assert_eq!(Advance::enter(true), Advance::Up);
    assert_eq!(Advance::tab(false), Advance::Right);
    assert_eq!(Advance::tab(true), Advance::Left);

    let mut core = nuevo();
    ir_a(&mut core, 5, 5);
    escribir(&mut core, "a", Advance::enter(true));
    assert_eq!(activa(&core), (4, 5), "Mayús+Intro sube");
    escribir(&mut core, "b", Advance::tab(true));
    assert_eq!(activa(&core), (4, 4), "Mayús+Tab va a la izquierda");
    assert_eq!(valor(&core, 5, 5), "a");
    assert_eq!(valor(&core, 4, 5), "b");
}

#[test]
fn sin_avance_la_celda_activa_no_se_mueve() {
    let mut core = nuevo();
    ir_a(&mut core, 3, 3);
    escribir(&mut core, "x", Advance::Stay);
    assert_eq!(activa(&core), (3, 3));
    assert_eq!(valor(&core, 3, 3), "x");
}

#[test]
fn el_avance_en_el_borde_de_la_hoja_deja_la_celda_activa_donde_esta() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "x", Advance::enter(true));
    assert_eq!(activa(&core), (1, 1), "no hay fila 0");
    escribir(&mut core, "y", Advance::tab(true));
    assert_eq!(activa(&core), (1, 1), "no hay columna 0");
    assert_eq!(valor(&core, 1, 1), "y");
}

#[test]
fn confirmar_sin_edicion_abierta_escribe_en_la_celda_activa() {
    let mut core = nuevo();
    ir_a(&mut core, 4, 2);
    core.execute(Command::CommitEdit {
        text: "directo".into(),
        advance: Advance::Down,
    })
    .unwrap();
    assert_eq!(valor(&core, 4, 2), "directo");
    assert_eq!(activa(&core), (5, 2));
}

#[test]
fn confirmar_un_texto_vacio_borra_la_celda() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 1);
    escribir(&mut core, "algo", Advance::Stay);
    escribir(&mut core, "", Advance::Stay);
    assert_eq!(valor(&core, 2, 1), "");
    assert_eq!(contenido(&core, 2, 1), "");
}

#[test]
fn confirmar_avanza_la_version_de_datos_y_la_vista_sigue_a_la_celda_activa() {
    let mut core = nuevo();
    core.set_viewport(500.0, 300.0, 1.0);
    let antes = core.data_version();
    // Una columna a la derecha del borde visible: el avance con Tab debe traerla a la vista.
    ir_a(&mut core, 1, 4);
    escribir(&mut core, "x", Advance::tab(false));
    assert!(core.data_version() > antes);
    let (fila, col) = activa(&core);
    assert_eq!((fila, col), (1, 5));
    let g = core.geometry(0).unwrap();
    let v = *core.view();
    assert!(
        g.col_x(col + 1) <= v.scroll_x + v.cells_w() + 1e-6,
        "la celda activa debe verse entera tras el avance: {v:?}"
    );
}

// ----------------------------------------------------------------------------------------
// US2.2 — editar una celda con contenido; Esc descarta
// ----------------------------------------------------------------------------------------

#[test]
fn escenario_2_f2_abre_con_el_contenido_actual_y_esc_conserva_el_valor() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    escribir(&mut core, "original", Advance::Stay);
    let version = core.data_version();

    core.execute(Command::BeginEdit {
        mode: EditMode::Append,
        seed: None,
    })
    .unwrap();
    let e = core.edit_state().expect("hay edición abierta").clone();
    assert_eq!((e.sheet, e.row, e.col), (0, 2, 2));
    assert_eq!(e.original, "original");
    assert_eq!(e.text, "original", "F2 abre con el contenido actual");
    assert_eq!(e.mode, EditMode::Append);

    core.execute(Command::SetEditText("otro texto".into()))
        .unwrap();
    // Mientras se edita el libro no cambia.
    assert_eq!(valor(&core, 2, 2), "original");
    core.execute(Command::CancelEdit).unwrap();
    assert!(core.edit_state().is_none());
    assert_eq!(
        valor(&core, 2, 2),
        "original",
        "Esc conserva el valor anterior"
    );
    assert_eq!(core.data_version(), version, "cancelar no cambia los datos");
    assert_eq!(activa(&core), (2, 2));
}

#[test]
fn clic_en_la_barra_de_formulas_abre_con_el_contenido_actual() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "=1+2", Advance::Stay);
    core.execute(Command::BeginEdit {
        mode: EditMode::FormulaBar,
        seed: None,
    })
    .unwrap();
    assert_eq!(core.edit_state().unwrap().text, "=1+2");
    assert_eq!(core.edit_state().unwrap().mode, EditMode::FormulaBar);
}

#[test]
fn escribir_sobre_la_celda_reemplaza_y_el_texto_inicial_es_lo_tecleado() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "viejo", Advance::Stay);
    core.execute(Command::BeginEdit {
        mode: EditMode::Replace,
        seed: Some("n".into()),
    })
    .unwrap();
    let e = core.edit_state().unwrap();
    assert_eq!(e.text, "n", "Reemplazar empieza solo con lo tecleado");
    assert_eq!(
        e.original, "viejo",
        "pero Esc debe poder volver al original"
    );
    core.execute(Command::CancelEdit).unwrap();
    assert_eq!(valor(&core, 1, 1), "viejo");
}

#[test]
fn anexar_con_texto_inicial_lo_agrega_al_contenido() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "abc", Advance::Stay);
    core.execute(Command::BeginEdit {
        mode: EditMode::Append,
        seed: Some("d".into()),
    })
    .unwrap();
    assert_eq!(core.edit_state().unwrap().text, "abcd");
}

#[test]
fn f2_durante_la_edicion_cambia_el_modo_sin_perder_lo_escrito() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    core.execute(Command::BeginEdit {
        mode: EditMode::Replace,
        seed: Some("hola".into()),
    })
    .unwrap();
    core.execute(Command::SetEditText("hola mundo".into()))
        .unwrap();
    core.execute(Command::BeginEdit {
        mode: EditMode::Append,
        seed: None,
    })
    .unwrap();
    let e = core.edit_state().unwrap();
    assert_eq!(e.mode, EditMode::Append);
    assert_eq!(e.text, "hola mundo");
}

#[test]
fn cancelar_o_cambiar_el_texto_sin_edicion_no_hace_nada() {
    let mut core = nuevo();
    core.execute(Command::CancelEdit).unwrap();
    core.execute(Command::SetEditText("x".into())).unwrap();
    assert!(core.edit_state().is_none());
    assert_eq!(valor(&core, 1, 1), "");
}

// ----------------------------------------------------------------------------------------
// US2.3 — celda y barra de fórmulas muestran lo mismo
// ----------------------------------------------------------------------------------------

#[test]
fn escenario_3_la_barra_de_formulas_muestra_el_texto_que_se_escribe() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    assert_eq!(core.formula_bar(), "", "celda vacía");
    core.execute(Command::BeginEdit {
        mode: EditMode::Replace,
        seed: Some("=SU".into()),
    })
    .unwrap();
    assert_eq!(core.formula_bar(), "=SU");
    for texto in ["=SUM", "=SUM(", "=SUM(1,2)"] {
        core.execute(Command::SetEditText(texto.into())).unwrap();
        assert_eq!(core.formula_bar(), texto);
        assert_eq!(core.edit_state().unwrap().text, texto);
    }
    core.execute(Command::CommitEdit {
        text: "=SUM(1,2)".into(),
        advance: Advance::Stay,
    })
    .unwrap();
    assert_eq!(
        core.formula_bar(),
        "=SUM(1,2)",
        "ya sin edición, el contenido de la celda"
    );
    assert_eq!(valor(&core, 2, 2), "3");
    // Al moverse a otra celda la barra sigue a la celda activa.
    ir_a(&mut core, 3, 3);
    assert_eq!(core.formula_bar(), "");
}

// ----------------------------------------------------------------------------------------
// Contenidos: texto, número, fórmula, error y texto largo
// ----------------------------------------------------------------------------------------

#[test]
fn celdas_con_texto_numero_formula_y_error() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "texto", Advance::Down);
    escribir(&mut core, "12.5", Advance::Down);
    escribir(&mut core, "=A1+A2", Advance::Down);
    escribir(&mut core, "=1/0", Advance::Down);
    escribir(&mut core, "=A1", Advance::Down);
    assert_eq!(valor(&core, 1, 1), "texto");
    assert_eq!(valor(&core, 2, 1), "12.5");
    assert_eq!(contenido(&core, 2, 1), "12.5");
    assert_eq!(valor(&core, 3, 1), "#VALUE!", "texto + número no es número");
    assert_eq!(valor(&core, 4, 1), "#DIV/0!");
    assert_eq!(
        contenido(&core, 4, 1),
        "=1/0",
        "el texto de la fórmula con error se conserva"
    );
    assert_eq!(valor(&core, 5, 1), "texto");

    // El error no bloquea: otra celda se escribe y se calcula con normalidad.
    ir_a(&mut core, 1, 3);
    escribir(&mut core, "=IFERROR(A4,\"capturado\")", Advance::Stay);
    assert_eq!(valor(&core, 1, 3), "capturado");
}

#[test]
fn editar_una_formula_muestra_su_texto_y_se_puede_corregir() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "=2+3", Advance::Stay);
    core.execute(Command::BeginEdit {
        mode: EditMode::Append,
        seed: None,
    })
    .unwrap();
    assert_eq!(core.edit_state().unwrap().text, "=2+3");
    core.execute(Command::CommitEdit {
        text: "=2+30".into(),
        advance: Advance::Stay,
    })
    .unwrap();
    assert_eq!(valor(&core, 1, 1), "32");
}

#[test]
fn una_formula_mal_escrita_deja_error_y_conserva_el_texto() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "=2+*3", Advance::Stay);
    assert!(
        valor(&core, 1, 1).starts_with('#'),
        "{}",
        valor(&core, 1, 1)
    );
    assert_eq!(contenido(&core, 1, 1), "=2+*3");
    // Se puede corregir.
    escribir(&mut core, "=2+3", Advance::Stay);
    assert_eq!(valor(&core, 1, 1), "5");
}

#[test]
fn texto_largo_con_saltos_de_linea_no_se_pierde_y_crece_la_fila() {
    let mut core = nuevo();
    core.set_viewport(800.0, 400.0, 1.0);
    ir_a(&mut core, 2, 1);
    let altura_antes = core.geometry(0).unwrap().row_h(2);

    let largo = format!("{}\nsegunda línea\ttercera", "palabra ".repeat(1_500));
    assert!(largo.chars().count() > 10_000);
    escribir(&mut core, &largo, Advance::Stay);
    assert_eq!(contenido(&core, 2, 1), largo, "no se pierde ni un carácter");
    assert_eq!(valor(&core, 2, 1), largo);
    assert_eq!(core.formula_bar(), largo);

    // Con un salto de línea la fila crece y la geometría lo sabe sin que nadie la invalide.
    let altura_despues = core.geometry(0).unwrap().row_h(2);
    assert!(
        altura_despues > altura_antes,
        "la fila debe crecer: {altura_antes} -> {altura_despues}"
    );
    // Y pintar sigue funcionando.
    let req = core.view().request(0.0, 0.0);
    let bloque = core.draw_block(req).unwrap();
    assert!(bloque.find_cell(2, 1).is_some());
}

#[test]
fn texto_con_apostrofo_inicial_es_texto_aunque_parezca_formula() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    escribir(&mut core, "'=1+1", Advance::Stay);
    assert_eq!(valor(&core, 1, 1), "=1+1");
}

// ----------------------------------------------------------------------------------------
// Rango usado y geometría
// ----------------------------------------------------------------------------------------

#[test]
fn escribir_fuera_del_rango_usado_amplia_la_extension_sin_reconstruir_en_cada_tecla() {
    let mut core = nuevo();
    core.set_viewport(500.0, 300.0, 1.0);
    ir_a(&mut core, 2, 2);
    escribir(&mut core, "x", Advance::Stay);
    assert_eq!(core.sheet_metrics().used_rows, 2);
    assert_eq!(core.sheet_metrics().used_cols, 2);

    // Una celda lejos del rango usado: la geometría se pone al día.
    core.execute(Command::Edge {
        dir: dokumentz_sheet::Dir::Down,
        extend: false,
    })
    .unwrap();
    let (fila, _) = activa(&core);
    escribir(&mut core, "lejos", Advance::Stay);
    assert_eq!(core.sheet_metrics().used_rows, fila);

    // Escribir dentro del rango usado no reconstruye la geometría: el `Arc` es el mismo.
    core.execute(Command::Home { ctrl: true }).unwrap();
    ir_a(&mut core, 1, 1);
    let antes = core.geometry(0).unwrap();
    escribir(&mut core, "dentro", Advance::Stay);
    let despues = core.geometry(0).unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&antes, &despues),
        "una edición dentro del rango no debe reconstruir la geometría"
    );
}

#[test]
fn el_bloque_de_dibujo_refleja_lo_confirmado() {
    let mut core = nuevo();
    ir_a(&mut core, 2, 2);
    let req: DrawRequest = core.view().request(0.0, 0.0);
    let antes = core.draw_block(req).unwrap();
    assert!(antes.find_cell(2, 2).is_none());
    escribir(&mut core, "visible", Advance::Stay);
    let req = core.view().request(0.0, 0.0);
    let despues = core.draw_block(req).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&antes, &despues));
    let celda = despues.find_cell(2, 2).expect("la celda está en el bloque");
    assert_eq!(despues.cell_text(celda), "visible");
}

// ----------------------------------------------------------------------------------------
// Avance dentro de un rango
// ----------------------------------------------------------------------------------------

/// Selecciona `(f0, c0)..(f1, c1)` con clic y arrastre (la celda activa es `(f0, c0)`).
fn seleccionar(core: &mut Core, f0: i32, c0: i32, f1: i32, c1: i32) {
    ir_a(core, f0, c0);
    let (x, y) = centro(core, f1, c1);
    core.execute(Command::DragTo { x, y }).unwrap();
    assert_eq!(core.selection().range, (f0, c0, f1, c1));
}

#[test]
fn con_un_rango_seleccionado_intro_recorre_el_rango_y_da_la_vuelta() {
    let mut core = nuevo();
    seleccionar(&mut core, 2, 2, 3, 3);
    let mut visitadas = Vec::new();
    for _ in 0..5 {
        visitadas.push(activa(&core));
        escribir(&mut core, "x", Advance::enter(false));
        assert_eq!(core.selection().range, (2, 2, 3, 3), "el rango se conserva");
    }
    // Baja por la columna, pasa a la siguiente, y da la vuelta al llegar al final.
    assert_eq!(visitadas, [(2, 2), (3, 2), (2, 3), (3, 3), (2, 2)]);
}

#[test]
fn con_un_rango_seleccionado_tab_recorre_por_filas_y_mayus_invierte() {
    let mut core = nuevo();
    seleccionar(&mut core, 2, 2, 3, 3);
    let mut visitadas = Vec::new();
    for _ in 0..4 {
        visitadas.push(activa(&core));
        escribir(&mut core, "x", Advance::tab(false));
    }
    assert_eq!(visitadas, [(2, 2), (2, 3), (3, 2), (3, 3)]);
    assert_eq!(activa(&core), (2, 2), "da la vuelta");
    escribir(&mut core, "y", Advance::tab(true));
    assert_eq!(
        activa(&core),
        (3, 3),
        "Mayús+Tab desde el principio va al final"
    );
}

// ----------------------------------------------------------------------------------------
// Historial, errores y casos raros
// ----------------------------------------------------------------------------------------

#[test]
fn cada_confirmacion_es_una_entrada_de_historial() {
    let mut core = nuevo();
    ir_a(&mut core, 1, 1);
    assert!(!core.can_undo());
    escribir(&mut core, "uno", Advance::Down);
    escribir(&mut core, "dos", Advance::Down);
    core.execute(Command::Undo).unwrap();
    assert_eq!(valor(&core, 2, 1), "");
    assert_eq!(valor(&core, 1, 1), "uno");
    core.execute(Command::Undo).unwrap();
    assert_eq!(valor(&core, 1, 1), "");
    assert!(!core.can_undo());
    core.execute(Command::Redo).unwrap();
    assert_eq!(valor(&core, 1, 1), "uno");
}

#[test]
fn una_celda_combinada_no_se_edita_por_dentro_y_su_ancla_si() {
    // La celda cubierta de un rango combinado no se puede escribir (error del motor); la ancla
    // sí, y el avance con Intro no da error aunque no pueda moverse por dentro de la combinada.
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("combinadas.xlsx");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.merge_range(0, 0, 1, 1, "combinada", &rust_xlsxwriter::Format::new())
        .unwrap();
    wb.save(&ruta).unwrap();
    let mut core = common::abrir(&ruta);
    let err = core.set_cell_input(0, 2, 2, "no").unwrap_err();
    assert!(matches!(err, CoreError::Motor(_)), "{err:?}");
    assert!(
        !core.is_dirty(),
        "una escritura rechazada no cambia el documento"
    );
    assert!(!core.can_undo());

    ir_a(&mut core, 1, 1);
    assert_eq!(
        core.selection().range,
        (1, 1, 2, 2),
        "la combinada se selecciona entera"
    );
    escribir(&mut core, "nuevo", Advance::enter(false));
    assert_eq!(valor(&core, 1, 1), "nuevo");
    assert_eq!(
        core.selection().active,
        (1, 1),
        "no hay por dónde avanzar dentro de la combinada"
    );
}

#[test]
fn una_hoja_inexistente_es_un_error_tipado() {
    let mut core = nuevo();
    assert_eq!(
        core.set_cell_input(7, 1, 1, "x").unwrap_err(),
        CoreError::HojaInvalida(7)
    );
    assert!(!core.is_dirty());
}
