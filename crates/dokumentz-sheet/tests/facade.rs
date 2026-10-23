//! E09 (T029): la fachada `Core::apply` / `Core::ui_state`, y el JSON de `Command` y `UiState`.
//!
//! * Cada `Command` produce el `UiState` esperado (agrupados por área: vista, navegación,
//!   edición, historial, hojas, portapapeles, archivo e idioma).
//! * `apply` no tiene lógica propia: es `execute` más `ui_state`.
//! * Ida y vuelta por JSON de **cada** variante de `Command` (la prueba falla al compilar si se
//!   añade una variante sin darle nombre aquí, y falla al correr si le falta un ejemplo) y del
//!   `UiState`; el formato exacto queda fijado con cadenas literales (orden de campos estable).
//! * Con la característica `metrics`: los tramos del núcleo llegan al `Recorder`.

mod common;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use dokumentz_sheet::{
    Advance, COL_HEADER_H, Command, Core, CoreError, Dir, EditMode, EditState, Lang, ROW_HEADER_W,
    SheetTab, UiState,
};

use common::{abrir, crear_small, opciones};

/// Libro `small.xlsx` (hoja «Datos», A1:D6) con un control de 1046 x 525 px.
fn pequeno() -> (tempfile::TempDir, Core) {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_small(dir.path()));
    core.set_viewport(1046.0, 525.0, 1.0);
    (dir, core)
}

/// Centro de la celda `(fila, col)` en coordenadas del control (con el desplazamiento actual).
fn centro(core: &mut Core, fila: i32, col: i32) -> (f32, f32) {
    let g = core.geometry(core.view().sheet).unwrap();
    let v = *core.view();
    let x = g.col_x(col) + g.col_w(col) / 2.0 - v.scroll_x;
    let y = g.row_y(fila) + g.row_h(fila) / 2.0 - v.scroll_y;
    (ROW_HEADER_W + x as f32, COL_HEADER_H + y as f32)
}

fn texto(core: &Core, fila: i32, col: i32) -> String {
    core.cell_text(core.selected_sheet(), fila, col).unwrap()
}

fn reemplazar(texto: &str) -> Command {
    Command::BeginEdit {
        mode: EditMode::Replace,
        seed: Some(texto.to_string()),
    }
}

fn confirmar(texto: &str, advance: Advance) -> Command {
    Command::CommitEdit {
        text: texto.to_string(),
        advance,
    }
}

// ------------------------------------------------------------------------------------------
// Cada comando produce el estado esperado
// ------------------------------------------------------------------------------------------

#[test]
fn vista_set_viewport_scroll_to_y_scroll_by() {
    let (_dir, mut core) = pequeno();
    let inicial = core.ui_state();
    assert_eq!(inicial.scroll, (0.0, 0.0));

    // SetViewport: la extensión desplazable es el rango usado más «una pantalla» (el área de
    // celdas), así que crece con el control; el desplazamiento no cambia.
    let grande = core
        .apply(Command::SetViewport {
            w: 1246.0,
            h: 725.0,
            scale: 2.0,
        })
        .unwrap();
    assert_eq!(grande.scroll, (0.0, 0.0));
    assert_eq!(grande.extent.0 - inicial.extent.0, 200.0);
    assert_eq!(grande.extent.1 - inicial.extent.1, 200.0);
    assert_eq!(core.view().scale, 2.0);

    // ScrollTo: absoluto, acotado a [0, extensión - área de celdas].
    let e = core.apply(Command::ScrollTo { x: 40.0, y: 60.0 }).unwrap();
    assert_eq!(e.scroll, (40.0, 60.0));
    let tope = core.apply(Command::ScrollTo { x: 1e9, y: 1e9 }).unwrap();
    assert_eq!(
        tope.scroll,
        (
            tope.extent.0 - core.view().cells_w(),
            tope.extent.1 - core.view().cells_h()
        )
    );
    // ScrollBy: relativo.
    core.apply(Command::ScrollTo { x: 10.0, y: 10.0 }).unwrap();
    let e = core.apply(Command::ScrollBy { dx: 5.0, dy: 30.0 }).unwrap();
    assert_eq!(e.scroll, (15.0, 40.0));
    // Un valor no finito se ignora: el estado no cambia y no es error.
    let e = core
        .apply(Command::ScrollBy {
            dx: f64::NAN,
            dy: -10.0,
        })
        .unwrap();
    assert_eq!(e.scroll, (15.0, 30.0));
}

#[test]
fn seleccion_click_drag_y_extend() {
    let (_dir, mut core) = pequeno();
    let inicial = core.ui_state();
    assert_eq!((inicial.active, inicial.range), ((1, 1), (1, 1, 1, 1)));

    let (x, y) = centro(&mut core, 2, 2);
    let e = core
        .apply(Command::Click {
            x,
            y,
            extend: false,
        })
        .unwrap();
    assert_eq!((e.active, e.range), ((2, 2), (2, 2, 2, 2)));
    assert_eq!(e.formula_bar, "1234.5"); // B2

    let (x, y) = centro(&mut core, 4, 4);
    let e = core.apply(Command::Click { x, y, extend: true }).unwrap();
    assert_eq!((e.active, e.range), ((2, 2), (2, 2, 4, 4)));

    // DragTo: tras un clic, arrastrar extiende desde el ancla.
    let (x, y) = centro(&mut core, 3, 2);
    core.apply(Command::Click {
        x,
        y,
        extend: false,
    })
    .unwrap();
    let (x, y) = centro(&mut core, 5, 3);
    let e = core.apply(Command::DragTo { x, y }).unwrap();
    assert_eq!((e.active, e.range), ((3, 2), (3, 2, 5, 3)));
}

#[test]
fn navegacion_move_edge_page_y_home() {
    let (_dir, mut core) = pequeno();
    let e = core
        .apply(Command::Move {
            dir: Dir::Down,
            extend: false,
        })
        .unwrap();
    assert_eq!((e.active, e.range), ((2, 1), (2, 1, 2, 1)));
    let e = core
        .apply(Command::Move {
            dir: Dir::Right,
            extend: true,
        })
        .unwrap();
    assert_eq!((e.active, e.range), ((2, 1), (2, 1, 2, 2)));

    // Edge: Ctrl+flecha va al borde del bloque de datos (A1:A6 tiene datos sin huecos).
    core.apply(Command::Home { ctrl: true }).unwrap();
    let e = core
        .apply(Command::Edge {
            dir: Dir::Down,
            extend: false,
        })
        .unwrap();
    assert_eq!(e.active, (6, 1));

    // Page: 525 - 25 = 500 px de celdas, 20 filas de 25 px (la fila 6 es más alta, da igual).
    let e = core
        .apply(Command::Page {
            down: true,
            extend: false,
        })
        .unwrap();
    assert!(e.active.0 > 6, "AvPág baja: {:?}", e.active);
    let e = core
        .apply(Command::Page {
            down: false,
            extend: false,
        })
        .unwrap();
    assert!(e.active.0 <= 6);

    // Home: columna A de la fila actual; Ctrl+Inicio: A1.
    core.apply(Command::Move {
        dir: Dir::Right,
        extend: false,
    })
    .unwrap();
    core.apply(Command::Move {
        dir: Dir::Right,
        extend: false,
    })
    .unwrap();
    let fila = core.ui_state().active.0;
    let e = core.apply(Command::Home { ctrl: false }).unwrap();
    assert_eq!(e.active, (fila, 1));
    let e = core.apply(Command::Home { ctrl: true }).unwrap();
    assert_eq!((e.active, e.scroll), ((1, 1), (0.0, 0.0)));
}

#[test]
fn edicion_begin_set_text_commit_y_cancel() {
    let (_dir, mut core) = pequeno();
    core.apply(Command::Move {
        dir: Dir::Down,
        extend: false,
    })
    .unwrap(); // A2 «Manzana»

    // BeginEdit en modo Replace: solo lo tecleado.
    let e = core.apply(reemplazar("kiwi")).unwrap();
    assert_eq!(
        e.edit,
        Some(EditState {
            sheet: 0,
            row: 2,
            col: 1,
            mode: EditMode::Replace,
            original: "Manzana".into(),
            text: "kiwi".into(),
        })
    );
    assert_eq!(e.formula_bar, "kiwi");
    assert!(!e.dirty, "editar no toca el libro hasta confirmar");

    // SetEditText: la celda y la barra muestran lo mismo.
    let e = core.apply(Command::SetEditText("kiwis".into())).unwrap();
    assert_eq!(e.edit.as_ref().unwrap().text, "kiwis");
    assert_eq!(e.formula_bar, "kiwis");

    // CancelEdit: cierra la sesión y conserva el valor anterior.
    let e = core.apply(Command::CancelEdit).unwrap();
    assert!(e.edit.is_none() && !e.dirty && !e.can_undo);
    assert_eq!(e.formula_bar, "Manzana");
    assert_eq!(texto(&core, 2, 1), "Manzana");

    // BeginEdit en modo Append: el contenido actual más lo tecleado.
    let e = core
        .apply(Command::BeginEdit {
            mode: EditMode::Append,
            seed: Some("!".into()),
        })
        .unwrap();
    assert_eq!(e.edit.as_ref().unwrap().text, "Manzana!");
    // FormulaBar sin semilla: el contenido actual.
    core.apply(Command::CancelEdit).unwrap();
    let e = core
        .apply(Command::BeginEdit {
            mode: EditMode::FormulaBar,
            seed: None,
        })
        .unwrap();
    assert_eq!(e.edit.as_ref().unwrap().text, "Manzana");
    assert_eq!(e.edit.as_ref().unwrap().mode, EditMode::FormulaBar);

    // CommitEdit: escribe, cierra, avanza (Intro baja) y deja el libro modificado.
    let e = core.apply(confirmar("kiwi", Advance::Down)).unwrap();
    assert!(e.edit.is_none());
    assert_eq!(e.active, (3, 1));
    assert!(e.dirty && e.can_undo && !e.can_redo);
    assert_eq!(texto(&core, 2, 1), "kiwi");
    assert!(e.data_version > 0);

    // Las cinco reglas de avance, partiendo siempre de B3.
    let (x, y) = centro(&mut core, 3, 2);
    for avance in [
        Advance::Up,
        Advance::Right,
        Advance::Left,
        Advance::Down,
        Advance::Stay,
    ] {
        core.apply(Command::Click {
            x,
            y,
            extend: false,
        })
        .unwrap();
        assert_eq!(core.ui_state().active, (3, 2));
        let e = core.apply(confirmar("9", avance)).unwrap();
        assert_eq!(e.active, destino_de((3, 2), avance), "{avance:?}");
    }
}

/// Destino del avance desde `desde` (sin saltos de filas ocultas: el fixture no las tiene).
fn destino_de(desde: (i32, i32), avance: Advance) -> (i32, i32) {
    match avance {
        Advance::Down => (desde.0 + 1, desde.1),
        Advance::Up => (desde.0 - 1, desde.1),
        Advance::Right => (desde.0, desde.1 + 1),
        Advance::Left => (desde.0, desde.1 - 1),
        Advance::Stay => desde,
    }
}

#[test]
fn un_comando_que_falla_devuelve_el_error_tipado_y_la_edicion_sigue_abierta() {
    let (_dir, mut core) = pequeno();
    core.apply(reemplazar("x")).unwrap();
    // Una hoja que no existe: el error es tipado (no es un texto del motor).
    let err = core.apply(Command::SelectSheet(99)).unwrap_err();
    assert!(matches!(err, CoreError::HojaInvalida(99)), "{err:?}");
    // El estado se puede releer: la edición sigue abierta y nada cambió.
    let e = core.ui_state();
    assert!(e.edit.is_some() && !e.dirty);
    assert_eq!(e.sheet, 0);
}

#[test]
fn historial_undo_y_redo() {
    let (_dir, mut core) = pequeno();
    let base = core.ui_state();
    assert!(!base.can_undo && !base.can_redo);

    core.apply(reemplazar("77")).unwrap();
    core.apply(confirmar("77", Advance::Stay)).unwrap();
    assert_eq!(texto(&core, 1, 1), "77");

    let e = core.apply(Command::Undo).unwrap();
    assert!(!e.can_undo && e.can_redo);
    assert_eq!(texto(&core, 1, 1), "Producto");
    assert_eq!(e.formula_bar, "Producto");
    assert!(e.data_version > base.data_version);

    let e = core.apply(Command::Redo).unwrap();
    assert!(e.can_undo && !e.can_redo);
    assert_eq!(texto(&core, 1, 1), "77");
    assert_eq!(e.formula_bar, "77");
    // Deshacer sin historial no es error.
    core.apply(Command::Undo).unwrap();
    core.apply(Command::Undo).unwrap();
    assert!(!core.ui_state().can_undo);
}

#[test]
fn hojas_nueva_selecciona_renombra_y_borra() {
    let (_dir, mut core) = pequeno();
    let e = core.ui_state();
    assert_eq!(
        e.sheets,
        vec![SheetTab {
            index: 0,
            name: "Datos".into(),
            color: None,
            active: true
        }]
    );

    // NewSheet: se añade al final y se selecciona.
    let e = core.apply(Command::NewSheet).unwrap();
    assert_eq!(e.sheets.len(), 2);
    assert_eq!(
        (e.sheet, e.sheets[1].active, e.sheets[0].active),
        (1, true, false)
    );
    assert!(e.dirty);

    // El contenido que se pinta es el de la hoja activa.
    let req = core.view().request(0.0, 0.0);
    assert_eq!(core.draw_block(req).unwrap().sheet, 1);

    // RenameSheet.
    let e = core
        .apply(Command::RenameSheet {
            sheet: 1,
            name: "Resumen".into(),
        })
        .unwrap();
    assert_eq!(e.sheets[1].name, "Resumen");

    // SelectSheet: vuelve a la primera.
    let e = core.apply(Command::SelectSheet(0)).unwrap();
    assert_eq!((e.sheet, e.sheets[0].active), (0, true));
    assert_eq!(e.formula_bar, "Producto");

    // DeleteSheet.
    let e = core.apply(Command::DeleteSheet(1)).unwrap();
    assert_eq!(e.sheets.len(), 1);
    // No se puede borrar la única hoja.
    assert!(core.apply(Command::DeleteSheet(0)).is_err());
}

#[test]
fn portapapeles_paste_text() {
    let (_dir, mut core) = pequeno();
    // Pegar 2 x 2 en F2 y comprobar que entra todo en un solo paso del historial.
    let (x, y) = centro(&mut core, 2, 6);
    core.apply(Command::Click {
        x,
        y,
        extend: false,
    })
    .unwrap();
    let e = core
        .apply(Command::PasteText("1\t2\r\n3\t4".into()))
        .unwrap();
    assert!(e.dirty && e.can_undo);
    assert_eq!(
        [
            texto(&core, 2, 6),
            texto(&core, 2, 7),
            texto(&core, 3, 6),
            texto(&core, 3, 7)
        ],
        ["1", "2", "3", "4"]
    );
    let e = core.apply(Command::Undo).unwrap();
    assert!(!e.can_undo);
    assert_eq!(texto(&core, 3, 7), "");
}

#[test]
fn archivo_save_con_ruta_sin_ruta_y_error() {
    let (dir, mut core) = pequeno();
    // Un libro nuevo no tiene ruta: «Guardar» sin destino es un error claro y el estado no cambia.
    let mut nuevo = Core::new_empty(opciones()).unwrap();
    let antes = nuevo.ui_state();
    assert!(matches!(
        nuevo.apply(Command::Save { path: None }),
        Err(CoreError::Io(_))
    ));
    assert_eq!(nuevo.ui_state(), antes);
    assert_eq!(antes.path, None);
    // El libro abierto desde un archivo sí tiene ruta.
    assert!(core.ui_state().path.is_some());

    // Guardar como: la ruta cambia y el documento queda limpio.
    core.apply(reemplazar("guardado")).unwrap();
    let e = core.apply(confirmar("guardado", Advance::Stay)).unwrap();
    assert!(e.dirty);
    let destino = dir.path().join("copia.xlsx");
    let e = core
        .apply(Command::Save {
            path: Some(destino.clone()),
        })
        .unwrap();
    assert!(!e.dirty);
    assert_eq!(e.path.as_deref(), destino.to_str());
    assert!(destino.exists());

    // Guardar sobre el archivo abierto (ahora, la copia).
    core.apply(reemplazar("otra")).unwrap();
    core.apply(confirmar("otra", Advance::Stay)).unwrap();
    let e = core.apply(Command::Save { path: None }).unwrap();
    assert!(!e.dirty);
    let reabierto = abrir(&destino);
    assert_eq!(reabierto.cell_text(0, 1, 1).unwrap(), "otra");
}

#[test]
fn idioma_set_lang_cambia_como_se_ve_la_formula() {
    let (_dir, mut core) = pequeno();
    core.apply(reemplazar("=SUM(1,2)")).unwrap();
    let e = core.apply(confirmar("=SUM(1,2)", Advance::Stay)).unwrap();
    assert_eq!(e.lang, Lang::En);
    assert_eq!(e.formula_bar, "=SUM(1,2)");
    let antes = e.data_version;

    let e = core.apply(Command::SetLang(Lang::Es)).unwrap();
    assert_eq!(e.lang, Lang::Es);
    assert_eq!(e.formula_bar, "=SUMA(1,2)");
    assert!(
        e.data_version > antes,
        "los errores se rotulan en el idioma activo"
    );
    assert!(
        e.dirty,
        "ya lo estaba por la edición; cambiar de idioma no lo limpia"
    );
}

#[test]
fn set_lang_no_ensucia_un_documento_limpio() {
    let (_dir, mut core) = pequeno();
    let e = core.apply(Command::SetLang(Lang::Es)).unwrap();
    assert!(!e.dirty && !e.can_undo);
}

// ------------------------------------------------------------------------------------------
// La fachada no tiene lógica propia
// ------------------------------------------------------------------------------------------

#[test]
fn apply_es_execute_mas_ui_state() {
    let guion = |click: (f32, f32)| {
        vec![
            Command::SetViewport {
                w: 900.0,
                h: 500.0,
                scale: 1.0,
            },
            Command::Click {
                x: click.0,
                y: click.1,
                extend: false,
            },
            reemplazar("10"),
            confirmar("10", Advance::Right),
            Command::Move {
                dir: Dir::Down,
                extend: true,
            },
            Command::Undo,
            Command::Redo,
            Command::SetLang(Lang::Es),
        ]
    };
    let (_d1, mut con_apply) = pequeno();
    let (_d2, mut con_execute) = pequeno();
    let click = centro(&mut con_apply, 3, 2);
    let mut ultimo = None;
    for cmd in guion(click) {
        ultimo = Some(con_apply.apply(cmd).unwrap());
    }
    for cmd in guion(click) {
        con_execute.execute(cmd).unwrap();
    }
    // Cada libro vive en su propio directorio temporal: solo la ruta puede diferir.
    let (mut a, mut b) = (ultimo.unwrap(), con_execute.ui_state());
    assert_ne!(a.path, b.path);
    (a.path, b.path) = (None, None);
    assert_eq!(a, b);
}

#[test]
fn ui_state_es_idempotente_y_pequeno() {
    let (_dir, mut core) = pequeno();
    let a = core.ui_state();
    let b = core.ui_state();
    assert_eq!(a, b, "leer el estado no lo cambia");
    assert_eq!((a.busy.clone(), a.message.clone()), (None, None));
    // Viaja como JSON en cada tecla: tiene que ser pequeño.
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.len() < 600, "{} bytes: {json}", json.len());
    // Mover la selección no cambia los datos: el bloque en caché sigue valiendo.
    let e = core
        .apply(Command::Move {
            dir: Dir::Down,
            extend: false,
        })
        .unwrap();
    assert_eq!(e.data_version, a.data_version);
}

#[test]
fn la_fachada_cruza_hilos_con_un_mutex() {
    let (_dir, core) = pequeno();
    let core = Arc::new(Mutex::new(core));
    let hilos: Vec<_> = (0..4)
        .map(|i| {
            let core = Arc::clone(&core);
            std::thread::spawn(move || {
                for _ in 0..10 {
                    let mut c = core.lock().unwrap();
                    c.apply(Command::Move {
                        dir: if i % 2 == 0 { Dir::Down } else { Dir::Right },
                        extend: false,
                    })
                    .unwrap();
                }
            })
        })
        .collect();
    for h in hilos {
        h.join().unwrap();
    }
    let e = core.lock().unwrap().ui_state();
    // 20 pasos hacia abajo y 20 a la derecha, en cualquier orden.
    assert_eq!(e.active, (21, 21));
}

// ------------------------------------------------------------------------------------------
// JSON: ida y vuelta de cada Command y del UiState; formato fijado
// ------------------------------------------------------------------------------------------

/// Nombre de la variante. **Sin comodín:** si se añade un `Command`, esta función no compila
/// hasta que se le dé nombre, y `VARIANTES` y `ejemplos` hasta que se le dé un ejemplo.
fn nombre(c: &Command) -> &'static str {
    match c {
        Command::SetViewport { .. } => "SetViewport",
        Command::ScrollTo { .. } => "ScrollTo",
        Command::ScrollBy { .. } => "ScrollBy",
        Command::Click { .. } => "Click",
        Command::DragTo { .. } => "DragTo",
        Command::Move { .. } => "Move",
        Command::Edge { .. } => "Edge",
        Command::Page { .. } => "Page",
        Command::Home { .. } => "Home",
        Command::SelectSheet(_) => "SelectSheet",
        Command::NewSheet => "NewSheet",
        Command::RenameSheet { .. } => "RenameSheet",
        Command::DeleteSheet(_) => "DeleteSheet",
        Command::BeginEdit { .. } => "BeginEdit",
        Command::SetEditText(_) => "SetEditText",
        Command::CommitEdit { .. } => "CommitEdit",
        Command::CancelEdit => "CancelEdit",
        Command::Undo => "Undo",
        Command::Redo => "Redo",
        Command::PasteText(_) => "PasteText",
        Command::Save { .. } => "Save",
        Command::SetLang(_) => "SetLang",
    }
}

const VARIANTES: [&str; 22] = [
    "SetViewport",
    "ScrollTo",
    "ScrollBy",
    "Click",
    "DragTo",
    "Move",
    "Edge",
    "Page",
    "Home",
    "SelectSheet",
    "NewSheet",
    "RenameSheet",
    "DeleteSheet",
    "BeginEdit",
    "SetEditText",
    "CommitEdit",
    "CancelEdit",
    "Undo",
    "Redo",
    "PasteText",
    "Save",
    "SetLang",
];

/// Al menos un ejemplo de cada variante, y los valores de cada tipo auxiliar (`Dir`, `Advance`,
/// `EditMode`, `Lang`, `Option`).
fn ejemplos() -> Vec<Command> {
    let mut v = vec![
        Command::SetViewport {
            w: 1280.5,
            h: 800.0,
            scale: 2.0,
        },
        Command::ScrollTo { x: 12.25, y: 1e6 },
        Command::ScrollBy {
            dx: -3.5,
            dy: 120.0,
        },
        Command::Click {
            x: 120.5,
            y: 60.0,
            extend: false,
        },
        Command::Click {
            x: 0.0,
            y: 0.0,
            extend: true,
        },
        Command::DragTo {
            x: 300.0,
            y: 200.75,
        },
        Command::Page {
            down: true,
            extend: false,
        },
        Command::Page {
            down: false,
            extend: true,
        },
        Command::Home { ctrl: true },
        Command::Home { ctrl: false },
        Command::SelectSheet(2),
        Command::NewSheet,
        Command::RenameSheet {
            sheet: 1,
            name: "Año «2026» \"fin\"\n".into(),
        },
        Command::DeleteSheet(3),
        Command::BeginEdit {
            mode: EditMode::Replace,
            seed: None,
        },
        Command::SetEditText("=SUMA(A1:A3)".into()),
        Command::CancelEdit,
        Command::Undo,
        Command::Redo,
        Command::PasteText("a\tb\r\n\"c\td\"\ttab".into()),
        Command::Save { path: None },
        Command::Save {
            path: Some("/tmp/libro con espacios.xlsx".into()),
        },
    ];
    for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
        v.push(Command::Move { dir, extend: false });
        v.push(Command::Edge { dir, extend: true });
    }
    for mode in [EditMode::Replace, EditMode::Append, EditMode::FormulaBar] {
        v.push(Command::BeginEdit {
            mode,
            seed: Some("ñ😀".into()),
        });
    }
    for advance in [
        Advance::Down,
        Advance::Up,
        Advance::Right,
        Advance::Left,
        Advance::Stay,
    ] {
        v.push(Command::CommitEdit {
            text: "texto".into(),
            advance,
        });
    }
    for lang in [Lang::Es, Lang::En] {
        v.push(Command::SetLang(lang));
    }
    v
}

#[test]
fn cada_variante_de_command_tiene_ejemplo_y_vuelve_igual_por_json() {
    let ejemplos = ejemplos();
    let cubiertas: BTreeSet<&str> = ejemplos.iter().map(nombre).collect();
    let todas: BTreeSet<&str> = VARIANTES.into_iter().collect();
    assert_eq!(
        cubiertas, todas,
        "falta un ejemplo o un nombre en VARIANTES"
    );
    for cmd in &ejemplos {
        let json = serde_json::to_string(cmd).unwrap();
        let de_vuelta: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(&de_vuelta, cmd, "ida y vuelta de {json}");
        // La etiqueta externa es siempre el nombre de la variante.
        let valor: serde_json::Value = serde_json::from_str(&json).unwrap();
        let etiqueta = match &valor {
            serde_json::Value::String(s) => s.as_str(),
            serde_json::Value::Object(o) => o.keys().next().unwrap().as_str(),
            otro => panic!("forma inesperada: {otro}"),
        };
        assert_eq!(etiqueta, nombre(cmd), "{json}");
        // Y el JSON re-serializado es idéntico (estable).
        assert_eq!(serde_json::to_string(&de_vuelta).unwrap(), json);
    }
}

#[test]
fn el_json_de_command_es_el_documentado() {
    let casos: [(Command, &str); 12] = [
        (Command::Undo, r#""Undo""#),
        (Command::NewSheet, r#""NewSheet""#),
        (Command::SelectSheet(2), r#"{"SelectSheet":2}"#),
        (Command::PasteText("a\tb".into()), r#"{"PasteText":"a\tb"}"#),
        (Command::SetLang(Lang::Es), r#"{"SetLang":"Es"}"#),
        (
            Command::SetViewport {
                w: 1280.0,
                h: 800.0,
                scale: 2.0,
            },
            r#"{"SetViewport":{"w":1280.0,"h":800.0,"scale":2.0}}"#,
        ),
        (
            Command::Click {
                x: 120.5,
                y: 60.0,
                extend: false,
            },
            r#"{"Click":{"x":120.5,"y":60.0,"extend":false}}"#,
        ),
        (
            Command::Move {
                dir: Dir::Down,
                extend: true,
            },
            r#"{"Move":{"dir":"Down","extend":true}}"#,
        ),
        (
            Command::BeginEdit {
                mode: EditMode::FormulaBar,
                seed: None,
            },
            r#"{"BeginEdit":{"mode":"FormulaBar","seed":null}}"#,
        ),
        (
            Command::CommitEdit {
                text: "=A1".into(),
                advance: Advance::Right,
            },
            r#"{"CommitEdit":{"text":"=A1","advance":"Right"}}"#,
        ),
        (Command::Save { path: None }, r#"{"Save":{"path":null}}"#),
        (
            Command::Save {
                path: Some("/a/b.xlsx".into()),
            },
            r#"{"Save":{"path":"/a/b.xlsx"}}"#,
        ),
    ];
    for (cmd, json) in casos {
        assert_eq!(serde_json::to_string(&cmd).unwrap(), json);
        assert_eq!(serde_json::from_str::<Command>(json).unwrap(), cmd);
    }
    // Los campos opcionales pueden faltar al leer (la interfaz web no siempre los manda).
    let c: Command = serde_json::from_str(r#"{"Save":{}}"#).unwrap();
    assert_eq!(c, Command::Save { path: None });
    let c: Command = serde_json::from_str(r#"{"BeginEdit":{"mode":"Replace"}}"#).unwrap();
    assert_eq!(
        c,
        Command::BeginEdit {
            mode: EditMode::Replace,
            seed: None
        }
    );
}

#[test]
fn un_command_mal_formado_es_error_y_no_panico() {
    for malo in [
        r#""Inexistente""#,
        r#"{"Click":{"x":1.0}}"#,
        r#"{"Move":{"dir":"Arriba","extend":false}}"#,
        r#"{"SelectSheet":-1}"#,
        r#"{"SetViewport":{"w":"ancho","h":1.0,"scale":1.0}}"#,
        r#"[]"#,
        r#"null"#,
        "",
    ] {
        assert!(
            serde_json::from_str::<Command>(malo).is_err(),
            "debía fallar: {malo}"
        );
    }
}

fn estado_completo() -> UiState {
    UiState {
        data_version: 7,
        sheet: 1,
        active: (3, 2),
        range: (3, 2, 5, 4),
        scroll: (10.5, 250.0),
        extent: (2000.0, 3000.25),
        edit: Some(EditState {
            sheet: 1,
            row: 3,
            col: 2,
            mode: EditMode::Append,
            original: "=A1".into(),
            text: "=A1+1".into(),
        }),
        formula_bar: "=A1+1".into(),
        sheets: vec![
            SheetTab {
                index: 0,
                name: "Enero".into(),
                color: None,
                active: false,
            },
            SheetTab {
                index: 1,
                name: "Febrero".into(),
                color: Some("#FF0000".into()),
                active: true,
            },
        ],
        lang: Lang::Es,
        path: Some("/tmp/a.xlsx".into()),
        can_undo: true,
        can_redo: false,
        dirty: true,
        busy: Some("guardando".into()),
        message: None,
    }
}

#[test]
fn el_json_del_ui_state_tiene_los_campos_en_orden_fijo() {
    const ESPERADO: &str = concat!(
        r#"{"data_version":7,"sheet":1,"active":[3,2],"range":[3,2,5,4],"#,
        r#""scroll":[10.5,250.0],"extent":[2000.0,3000.25],"#,
        r#""edit":{"sheet":1,"row":3,"col":2,"mode":"Append","original":"=A1","text":"=A1+1"},"#,
        r#""formula_bar":"=A1+1","#,
        r##""sheets":[{"index":0,"name":"Enero","color":null,"active":false},"##,
        r##"{"index":1,"name":"Febrero","color":"#FF0000","active":true}],"##,
        r#""lang":"Es","path":"/tmp/a.xlsx","can_undo":true,"can_redo":false,"dirty":true,"#,
        r#""busy":"guardando","message":null}"#,
    );
    let estado = estado_completo();
    assert_eq!(serde_json::to_string(&estado).unwrap(), ESPERADO);
    assert_eq!(serde_json::from_str::<UiState>(ESPERADO).unwrap(), estado);
}

#[test]
fn el_json_de_un_libro_nuevo_es_estable() {
    // Libro vacío con el control por defecto (1280 x 800): sin datos, sin edición, sin ruta.
    const ESPERADO: &str = concat!(
        r#"{"data_version":0,"sheet":0,"active":[1,1],"range":[1,1,1,1],"#,
        r#""scroll":[0.0,0.0],"extent":[1234.0,775.0],"edit":null,"formula_bar":"","#,
        r#""sheets":[{"index":0,"name":"Sheet1","color":null,"active":true}],"#,
        r#""lang":"En","path":null,"can_undo":false,"can_redo":false,"dirty":false,"#,
        r#""busy":null,"message":null}"#,
    );
    let mut core = Core::new_empty(opciones()).unwrap();
    assert_eq!(serde_json::to_string(&core.ui_state()).unwrap(), ESPERADO);
}

#[test]
fn el_ui_state_de_cada_paso_vuelve_igual_por_json() {
    let (_dir, mut core) = pequeno();
    let (x, y) = centro(&mut core, 2, 2);
    let guion = [
        Command::Click {
            x,
            y,
            extend: false,
        },
        reemplazar("=SUM(1,2)"),
        confirmar("=SUM(1,2)", Advance::Down),
        Command::SetLang(Lang::Es),
        Command::NewSheet,
        Command::Undo,
    ];
    for cmd in guion {
        let estado = match core.apply(cmd.clone()) {
            Ok(e) => e,
            Err(_) => core.ui_state(),
        };
        let json = serde_json::to_string(&estado).unwrap();
        assert_eq!(
            serde_json::from_str::<UiState>(&json).unwrap(),
            estado,
            "{cmd:?}"
        );
    }
    // Un estado abierto en edición también viaja bien.
    core.apply(reemplazar("x")).unwrap();
    let estado = core.ui_state();
    assert!(estado.edit.is_some());
    let json = serde_json::to_string(&estado).unwrap();
    assert_eq!(serde_json::from_str::<UiState>(&json).unwrap(), estado);
}

#[test]
fn una_ruta_con_utf8_viaja_en_el_ui_state_y_en_save() {
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("ñandú — 😀.xlsx");
    let mut core = Core::new_empty(opciones()).unwrap();
    let e = core
        .apply(Command::Save {
            path: Some(destino.clone()),
        })
        .unwrap();
    assert_eq!(e.path.as_deref(), destino.to_str());
    let cmd = Command::Save {
        path: Some(destino),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert_eq!(serde_json::from_str::<Command>(&json).unwrap(), cmd);
}

// ------------------------------------------------------------------------------------------
// Pausa de la evaluación (R-04, opción a)
// ------------------------------------------------------------------------------------------

#[test]
fn pausar_la_evaluacion_difiere_el_calculo_hasta_recalcular() {
    let (_dir, mut core) = pequeno();
    // D2 = B2 * 2 = 2469 (valor guardado); cambiar B2 con la evaluación pausada no recalcula.
    assert_eq!(texto(&core, 2, 4), "2469");
    assert!(!core.is_evaluation_paused());
    core.pause_evaluation();
    assert!(core.is_evaluation_paused());
    core.set_cell_input(0, 2, 2, "10").unwrap();
    assert_eq!(
        texto(&core, 2, 4),
        "2469",
        "pausada: sigue el valor anterior"
    );
    // Reanudar no evalúa por sí solo; recalcular sí.
    core.resume_evaluation();
    assert!(!core.is_evaluation_paused());
    assert_eq!(texto(&core, 2, 4), "2469");
    core.recalculate();
    assert_eq!(texto(&core, 2, 4), "20");
    // Sin pausa, una escritura evalúa en el acto.
    core.set_cell_input(0, 2, 2, "11").unwrap();
    assert_eq!(texto(&core, 2, 4), "22");
}

// ------------------------------------------------------------------------------------------
// Característica `metrics`
// ------------------------------------------------------------------------------------------

#[cfg(feature = "metrics")]
mod con_metricas {
    use super::*;
    use dokumentz_metrics::Recorder;
    use dokumentz_sheet::metric_names as n;

    fn muestras(r: &Recorder, serie: &str) -> u64 {
        r.report()
            .unwrap()
            .internal
            .unwrap()
            .histograms
            .get(serie)
            .map_or(0, |h| h.count)
    }

    fn contador(r: &Recorder, nombre: &str) -> u64 {
        r.report()
            .unwrap()
            .internal
            .unwrap()
            .counters
            .get(nombre)
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn los_tramos_del_nucleo_llegan_al_recorder() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = crear_small(dir.path());
        // El JSON se pediría en `dir`, pero el núcleo nunca lo escribe: lo escribe quien lo creó.
        let salida = dir.path().join("metricas.json");
        let rec = Arc::new(Recorder::to_path(&salida));

        // Abrir: `load_ms`.
        let mut core = Core::open_xlsx_with_recorder(&ruta, opciones(), Arc::clone(&rec)).unwrap();
        assert_eq!(muestras(&rec, n::LOAD_MS), 1);
        assert!(core.recorder().is_some());
        // Un fallo al abrir no deja muestra.
        assert!(
            Core::open_xlsx_with_recorder(
                &dir.path().join("no_existe.xlsx"),
                opciones(),
                Arc::clone(&rec)
            )
            .is_err()
        );
        assert_eq!(muestras(&rec, n::LOAD_MS), 1);

        // draw_block: una construcción y una respuesta de la caché.
        core.set_viewport(1046.0, 525.0, 1.0);
        let req = core.view().request(0.0, 200.0);
        let bloque = core.draw_block(req).unwrap();
        core.draw_block(req).unwrap();
        assert_eq!(
            muestras(&rec, n::DRAW_BLOCK_MS),
            1,
            "solo cuenta construcciones"
        );
        assert_eq!(contador(&rec, n::DRAW_BLOCK_BUILDS), 1);
        assert_eq!(contador(&rec, n::DRAW_BLOCK_HITS), 1);
        assert_eq!(
            contador(&rec, n::CELLS_IN_LIST),
            u64::from(core.drawn_cells_last())
        );
        assert_eq!(muestras(&rec, n::CELLS_PER_BLOCK), 1);

        // Codificar: `encode_ms`, y el resultado es el mismo que el de `Core::encode`.
        let bytes = core.encode_block(&bloque);
        assert_eq!(bytes, Core::encode(&bloque));
        assert_eq!(muestras(&rec, n::ENCODE_MS), 1);

        // CommitEdit y evaluar.
        core.apply(reemplazar("5")).unwrap();
        core.apply(confirmar("5", Advance::Stay)).unwrap();
        assert_eq!(muestras(&rec, n::COMMIT_EDIT_MS), 1);
        core.recalculate();
        assert_eq!(muestras(&rec, n::EVALUATE_MS), 1);

        // Guardar.
        core.apply(Command::Save {
            path: Some(dir.path().join("copia.xlsx")),
        })
        .unwrap();
        assert_eq!(muestras(&rec, n::SAVE_MS), 1);

        // El núcleo no escribió nada: el JSON solo aparece cuando quien creó el Recorder lo pide.
        assert!(!salida.exists());
        rec.write_json().unwrap();
        assert!(salida.exists());
    }

    #[test]
    fn un_recorder_apagado_se_descarta_y_no_deja_rastro() {
        let (_dir, mut core) = pequeno();
        let apagado = Arc::new(Recorder::off());
        core.set_recorder(Arc::clone(&apagado));
        assert!(
            core.recorder().is_none(),
            "un recorder apagado no se guarda"
        );
        let req = core.view().request(0.0, 0.0);
        core.draw_block(req).unwrap();
        core.apply(reemplazar("1")).unwrap();
        core.apply(confirmar("1", Advance::Stay)).unwrap();
        assert!(apagado.report().is_none());
        assert_eq!(apagado.write_json().unwrap(), None);
    }

    #[test]
    fn sin_recorder_el_nucleo_funciona_igual() {
        let (_dir, mut core) = pequeno();
        assert!(core.recorder().is_none());
        let req = core.view().request(0.0, 0.0);
        let a = core.draw_block(req).unwrap();
        assert!(!Core::encode(&a).is_empty());
    }
}
