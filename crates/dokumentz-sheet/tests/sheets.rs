//! E08 (T026): hojas (FR-009, US3.1). `SheetTab` para `UiState.sheets`, `SelectSheet`, y crear,
//! renombrar y borrar (opcional en la PoC, pero cuesta poco y entra en el historial).
//!
//! El fixture es un libro de 5 hojas de 10 000 filas (el A3 de `spec.md`), generado con
//! `rust_xlsxwriter` en un directorio temporal: cada hoja `k` lleva «Hoja k» en A1, `k * 100` en
//! B2 y números en las filas 3 a 10 000, para saber qué hoja se está viendo.

mod common;

use dokumentz_sheet::{Command, Core, CoreError, Dir, SheetTab};
use rust_xlsxwriter::{Color, Workbook};

use common::abrir;

const NOMBRES: [&str; 5] = ["Enero", "Febrero", "Marzo", "Abril", "Mayo"];
const FILAS: u32 = 10_000;

fn crear_a3(dir: &std::path::Path) -> std::path::PathBuf {
    let ruta = dir.join("a3.xlsx");
    let mut wb = Workbook::new();
    for (i, nombre) in NOMBRES.iter().enumerate() {
        let k = i as u32 + 1;
        let ws = wb.add_worksheet();
        ws.set_name(*nombre).unwrap();
        ws.write_string(0, 0, format!("Hoja {k}")).unwrap();
        ws.write_number(1, 1, f64::from(k * 100)).unwrap();
        for r in 2..FILAS {
            ws.write_number(r, 0, f64::from(r * k)).unwrap();
        }
        if k == 2 {
            // Una hoja con pestaña de color; la hoja 3 cita a la 1.
            ws.set_tab_color(Color::RGB(0xFF0000));
        }
        if k == 3 {
            ws.write_formula(1, 2, "=Enero!B2+1").unwrap();
        }
    }
    wb.save(&ruta).expect("guardar a3.xlsx");
    ruta
}

fn a3() -> (tempfile::TempDir, Core) {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_a3(dir.path()));
    core.recalculate();
    core.set_viewport(800.0, 400.0, 1.0);
    (dir, core)
}

/// Texto de la celda `(fila, col)` de la lista de dibujo de la hoja visible.
fn pintado(core: &mut Core, fila: i32, col: i32) -> (u32, String) {
    let req = core.view().request(0.0, 0.0);
    let bloque = core.draw_block(req).unwrap();
    let texto = bloque
        .find_cell(fila, col)
        .map(|c| bloque.cell_text(c).to_string())
        .unwrap_or_default();
    (bloque.sheet, texto)
}

fn activas(core: &Core) -> Vec<u32> {
    core.sheet_tabs()
        .iter()
        .filter(|t| t.active)
        .map(|t| t.index)
        .collect()
}

#[test]
fn las_pestanas_listan_las_cinco_hojas_con_la_activa_marcada() {
    let (_dir, core) = a3();
    let tabs = core.sheet_tabs();
    assert_eq!(tabs.len(), 5);
    assert_eq!(
        tabs.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        NOMBRES
    );
    assert_eq!(
        tabs.iter().map(|t| t.index).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(activas(&core), [0]);
    assert_eq!(core.sheet_names(), NOMBRES);
    // Colores: solo la pestaña de «Febrero» tiene.
    assert_eq!(tabs[1].color.as_deref(), Some("#FF0000"));
    assert_eq!(tabs[0].color, None);
    assert_eq!(
        tabs[0],
        SheetTab {
            index: 0,
            name: "Enero".into(),
            color: None,
            active: true
        }
    );
}

#[test]
fn escenario_1_cada_pestana_muestra_el_contenido_de_su_hoja() {
    let (_dir, mut core) = a3();
    for k in 0..5u32 {
        core.execute(Command::SelectSheet(k)).unwrap();
        assert_eq!(core.selected_sheet(), k);
        assert_eq!(core.view().sheet, k);
        assert_eq!(activas(&core), [k], "solo la pestaña activa queda marcada");
        // draw_block devuelve el contenido de esa hoja: A1 «Hoja k» y B2 el número de la hoja.
        assert_eq!(pintado(&mut core, 1, 1), (k, format!("Hoja {}", k + 1)));
        assert_eq!(pintado(&mut core, 2, 2).1, ((k + 1) * 100).to_string());
        // Y una celda de datos (fila 5, columna A = fila - 1 por la hoja: r * k con r base 0).
        assert_eq!(pintado(&mut core, 5, 1).1, (4 * (k + 1)).to_string());
    }
    // Volver a la primera.
    core.execute(Command::SelectSheet(0)).unwrap();
    assert_eq!(pintado(&mut core, 1, 1), (0, "Hoja 1".to_string()));
}

#[test]
fn cambiar_de_hoja_sube_la_version_y_no_marca_el_documento_como_modificado() {
    let (_dir, mut core) = a3();
    let v0 = core.data_version();
    assert!(!core.is_dirty());
    core.select_sheet(0).unwrap();
    assert_eq!(
        core.data_version(),
        v0,
        "seleccionar la hoja que ya se ve no hace nada"
    );
    core.select_sheet(3).unwrap();
    assert!(core.data_version() > v0);
    assert!(!core.is_dirty(), "cambiar de hoja no es una edición");
    assert!(!core.can_undo());
}

#[test]
fn una_hoja_inexistente_es_un_error_tipado_y_no_cambia_nada() {
    let (_dir, mut core) = a3();
    core.select_sheet(2).unwrap();
    assert_eq!(core.select_sheet(5), Err(CoreError::HojaInvalida(5)));
    assert_eq!(
        core.execute(Command::SelectSheet(99)),
        Err(CoreError::HojaInvalida(99))
    );
    assert_eq!(core.selected_sheet(), 2);
    assert_eq!(activas(&core), [2]);
}

#[test]
fn cada_hoja_recuerda_su_seleccion_y_su_desplazamiento() {
    let (_dir, mut core) = a3();
    // En «Enero»: mover la celda activa y desplazarse hacia abajo.
    for _ in 0..3 {
        core.move_active(Dir::Down, false).unwrap();
    }
    core.move_active(Dir::Right, false).unwrap();
    core.execute(Command::ScrollTo { x: 0.0, y: 2_000.0 })
        .unwrap();
    // Se mueve la celda activa de nuevo para que quede dentro de lo que se ve.
    core.execute(Command::Page {
        down: true,
        extend: false,
    })
    .unwrap();
    let sel0 = core.selection();
    let scroll0 = (core.view().scroll_x, core.view().scroll_y);
    assert!(scroll0.1 > 0.0);

    core.select_sheet(1).unwrap();
    assert_eq!(core.selection().active, (1, 1), "«Febrero» empieza en A1");
    assert_eq!(core.view().scroll_y, 0.0);
    core.move_active(Dir::Right, false).unwrap();
    core.move_active(Dir::Right, false).unwrap();

    core.select_sheet(0).unwrap();
    assert_eq!(core.selection(), sel0, "«Enero» recuerda su selección");
    assert_eq!((core.view().scroll_x, core.view().scroll_y), scroll0);
    core.select_sheet(1).unwrap();
    assert_eq!(core.selection().active, (1, 3));
}

#[test]
fn editar_escribe_en_la_hoja_visible_y_no_en_las_demas() {
    let (_dir, mut core) = a3();
    core.select_sheet(3).unwrap();
    core.execute(Command::CommitEdit {
        text: "solo aquí".into(),
        advance: dokumentz_sheet::Advance::Stay,
    })
    .unwrap();
    assert_eq!(core.cell_text(3, 1, 1).unwrap(), "solo aquí");
    assert_eq!(core.cell_text(0, 1, 1).unwrap(), "Hoja 1");
    assert_eq!(pintado(&mut core, 1, 1), (3, "solo aquí".to_string()));
    core.select_sheet(0).unwrap();
    assert_eq!(pintado(&mut core, 1, 1), (0, "Hoja 1".to_string()));
}

#[test]
fn una_edicion_abierta_sigue_apuntando_a_su_celda_aunque_se_cambie_de_hoja() {
    let (_dir, mut core) = a3();
    core.select_sheet(1).unwrap();
    core.execute(Command::BeginEdit {
        mode: dokumentz_sheet::EditMode::Replace,
        seed: Some("hacia febrero".into()),
    })
    .unwrap();
    core.select_sheet(2).unwrap();
    core.execute(Command::CommitEdit {
        text: "hacia febrero".into(),
        advance: dokumentz_sheet::Advance::Stay,
    })
    .unwrap();
    assert_eq!(core.cell_text(1, 1, 1).unwrap(), "hacia febrero");
    assert_eq!(core.cell_text(2, 1, 1).unwrap(), "Hoja 3");
}

#[test]
fn las_hojas_ocultas_no_tienen_pestana_pero_existen() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("oculta.xlsx");
    let mut wb = Workbook::new();
    wb.add_worksheet().set_name("Visible").unwrap();
    let oculta = wb.add_worksheet();
    oculta.set_name("Secreta").unwrap();
    oculta.write_string(0, 0, "dentro").unwrap();
    oculta.set_hidden(true);
    wb.add_worksheet().set_name("Otra").unwrap();
    wb.save(&ruta).unwrap();
    let core = abrir(&ruta);
    let tabs = core.sheet_tabs();
    assert_eq!(
        tabs.iter()
            .map(|t| (t.index, t.name.as_str()))
            .collect::<Vec<_>>(),
        [(0, "Visible"), (2, "Otra")]
    );
    assert_eq!(core.sheet_names().len(), 3);
    assert_eq!(core.cell_text(1, 1, 1).unwrap(), "dentro");
}

// ----------------------------------------------------------------------------------------
// Crear, renombrar y borrar
// ----------------------------------------------------------------------------------------

#[test]
fn crear_una_hoja_la_selecciona_y_se_puede_deshacer() {
    let (_dir, mut core) = a3();
    core.select_sheet(2).unwrap();
    core.execute(Command::NewSheet).unwrap();
    assert_eq!(core.sheet_tabs().len(), 6);
    assert_eq!(
        core.selected_sheet(),
        5,
        "la hoja nueva queda al final y seleccionada"
    );
    assert_eq!(activas(&core), [5]);
    assert_eq!(pintado(&mut core, 1, 1), (5, String::new()));
    assert!(core.is_dirty());

    core.undo().unwrap();
    assert_eq!(core.sheet_tabs().len(), 5);
    assert!(
        core.selected_sheet() < 5,
        "la vista no se queda en una hoja que ya no existe"
    );
    assert_eq!(core.view().sheet, core.selected_sheet());
    let (hoja, _) = pintado(&mut core, 1, 1);
    assert_eq!(hoja, core.selected_sheet());

    core.redo().unwrap();
    assert_eq!(core.sheet_tabs().len(), 6);
}

#[test]
fn renombrar_actualiza_las_formulas_que_citan_la_hoja() {
    let (_dir, mut core) = a3();
    assert_eq!(core.cell_input(2, 2, 3).unwrap(), "=Enero!B2+1");
    assert_eq!(core.cell_text(2, 2, 3).unwrap(), "101");
    core.execute(Command::RenameSheet {
        sheet: 0,
        name: "Ene".into(),
    })
    .unwrap();
    assert_eq!(core.sheet_tabs()[0].name, "Ene");
    assert_eq!(core.cell_input(2, 2, 3).unwrap(), "=Ene!B2+1");
    assert_eq!(core.cell_text(2, 2, 3).unwrap(), "101");
    // Nombres inválidos o repetidos son errores del motor y no cambian nada.
    assert!(matches!(
        core.rename_sheet(1, "Marzo"),
        Err(CoreError::Motor(_))
    ));
    assert!(matches!(
        core.rename_sheet(1, "mal/nombre"),
        Err(CoreError::Motor(_))
    ));
    assert_eq!(core.rename_sheet(9, "X"), Err(CoreError::HojaInvalida(9)));
    core.undo().unwrap();
    assert_eq!(core.sheet_tabs()[0].name, "Enero");
    assert_eq!(core.cell_input(2, 2, 3).unwrap(), "=Enero!B2+1");
}

#[test]
fn borrar_una_hoja_y_deshacer_la_devuelve_con_su_contenido() {
    let (_dir, mut core) = a3();
    core.select_sheet(4).unwrap();
    core.execute(Command::DeleteSheet(4)).unwrap();
    assert_eq!(core.sheet_tabs().len(), 4);
    assert_eq!(core.selected_sheet(), 3, "se pasa a la hoja anterior");
    assert_eq!(core.view().sheet, 3);
    assert_eq!(pintado(&mut core, 1, 1), (3, "Hoja 4".to_string()));

    core.undo().unwrap();
    assert_eq!(core.sheet_tabs().len(), 5);
    assert_eq!(core.cell_text(4, 1, 1).unwrap(), "Hoja 5");
    assert_eq!(core.cell_text(4, 2, 2).unwrap(), "500");

    // Borrar una hoja del medio desplaza las siguientes, y la vista sigue siendo coherente.
    core.select_sheet(3).unwrap();
    core.delete_sheet(1).unwrap();
    assert_eq!(core.sheet_names(), ["Enero", "Marzo", "Abril", "Mayo"]);
    let activa = core.selected_sheet();
    assert_eq!(core.view().sheet, activa);
    let (hoja, _) = pintado(&mut core, 1, 1);
    assert_eq!(hoja, activa);
}

#[test]
fn no_se_puede_borrar_la_unica_hoja() {
    let mut core = Core::new_empty(common::opciones()).unwrap();
    assert!(matches!(core.delete_sheet(0), Err(CoreError::Motor(_))));
    assert_eq!(core.sheet_tabs().len(), 1);
    assert_eq!(core.delete_sheet(3), Err(CoreError::HojaInvalida(3)));
}

#[test]
fn guardar_y_reabrir_conserva_las_cinco_hojas_en_orden() {
    let (dir, mut core) = a3();
    core.select_sheet(2).unwrap();
    let destino = dir.path().join("guardado.xlsx");
    core.save(Some(&destino)).unwrap();
    let mut otra = abrir(&destino);
    otra.recalculate();
    assert_eq!(otra.sheet_names(), NOMBRES);
    for k in 0..5u32 {
        assert_eq!(
            otra.cell_text(k, 1, 1).unwrap(),
            format!("Hoja {}", k + 1),
            "hoja {k}"
        );
    }
}
