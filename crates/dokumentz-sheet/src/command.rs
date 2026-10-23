//! Comandos de la interfaz sobre el núcleo y estado que se les devuelve (`plan.md` §5.5).
//!
//! # La fachada
//!
//! Una interfaz habla con el núcleo con tres llamadas, todas sobre un `Mutex<Core>`
//! (`plan.md` §5.11):
//!
//! * [`Core::apply`]: ejecuta un [`Command`] y devuelve el [`UiState`] resultante. Es lo que
//!   hace el comando `exec` de Tauri y el manejador de entrada de GPUI.
//! * [`Core::ui_state`]: el mismo estado sin ejecutar nada (al abrir un archivo, o tras un
//!   error).
//! * [`Core::draw_block`] / [`Core::encode_block`]: el bloque de la cuadrícula para pintar.
//!
//! [`Core::execute`] ejecuta un comando sin construir el estado: lo usan las pruebas y los
//! ejecutores por lotes, que miran el estado una vez al final. `apply` es `execute` más
//! `ui_state`, sin lógica propia.
//!
//! # JSON estable
//!
//! `Command` y `UiState` implementan `serde`. El formato es el de `serde_json` con las
//! etiquetas por defecto y **no cambia sin aviso**:
//!
//! * `Command` es una enumeración con etiqueta externa: una variante sin datos es una cadena
//!   (`"Undo"`), una de un solo dato lleva ese dato (`{"SelectSheet":2}`,
//!   `{"PasteText":"a\tb"}`) y una con campos lleva un objeto
//!   (`{"Click":{"x":120.5,"y":60.0,"extend":false}}`). Los nombres son los de Rust.
//! * `UiState` es un objeto cuyos campos salen **siempre en el orden de su declaración**
//!   (`data_version` primero); las tuplas son listas (`"active":[3,2]`). Los campos nuevos solo
//!   se añaden al final.
//! * Los números no finitos no existen en JSON: un `Command` con `NaN` o infinito no se puede
//!   serializar. El núcleo ya trata esos valores como «ignorar», así que la interfaz no debe
//!   enviarlos.
//!
//! E07 definió las variantes de **vista, selección y navegación**; E08 las de **edición,
//! historial, hojas, portapapeles, archivo e idioma**.
//!
//! Copiar no es un comando (devuelve texto): la interfaz llama a [`Core::copy_text`].
//!
//! Para añadir un comando: una variante nueva aquí, su método en `Core` (en el módulo que le
//! corresponda), una rama en [`Core::execute`] y su caso en `tests/facade.rs`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::{Core, Lang, SheetTab};
use crate::edit::{Advance, EditMode, EditState};
use crate::error::CoreError;
use crate::nav::Dir;

/// Una orden de la interfaz al núcleo. Las coordenadas `x`, `y` son píxeles lógicos del control
/// de la cuadrícula con origen en su esquina superior izquierda, cabeceras incluidas.
///
/// Se serializa a JSON como se explica en el módulo. `Save` lleva la ruta como cadena; una ruta
/// que no sea UTF-8 no se puede serializar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // --- vista ---
    /// Tamaño del control (cabeceras incluidas) y escala de la pantalla.
    SetViewport {
        /// Ancho.
        w: f32,
        /// Alto.
        h: f32,
        /// Píxeles físicos por lógico.
        scale: f32,
    },
    /// Desplazamiento absoluto en píxeles de hoja.
    ScrollTo {
        /// Coordenada x de hoja en el borde izquierdo del área de celdas.
        x: f64,
        /// Coordenada y de hoja en el borde superior del área de celdas.
        y: f64,
    },
    /// Desplazamiento relativo en píxeles de hoja.
    ScrollBy {
        /// Incremento horizontal.
        dx: f64,
        /// Incremento vertical.
        dy: f64,
    },
    // --- selección y navegación (FR-004) ---
    /// Clic; con `extend` (Mayús), extiende el rango.
    Click {
        /// Posición x.
        x: f32,
        /// Posición y.
        y: f32,
        /// Mayús pulsada.
        extend: bool,
    },
    /// Arrastre del puntero (botón pulsado) hasta `(x, y)`.
    DragTo {
        /// Posición x.
        x: f32,
        /// Posición y.
        y: f32,
    },
    /// Flechas; Mayús = `extend`.
    Move {
        /// Dirección.
        dir: Dir,
        /// Mayús pulsada.
        extend: bool,
    },
    /// Ctrl/Cmd+flecha: ir al borde del bloque de datos.
    Edge {
        /// Dirección.
        dir: Dir,
        /// Mayús pulsada.
        extend: bool,
    },
    /// RePág (`down = false`) / AvPág (`down = true`).
    Page {
        /// Hacia abajo.
        down: bool,
        /// Mayús pulsada.
        extend: bool,
    },
    /// Inicio (`ctrl = false`) / Ctrl+Inicio.
    Home {
        /// Ctrl (o Cmd) pulsada.
        ctrl: bool,
    },
    /// Cambia la hoja visible (índice base 0).
    SelectSheet(u32),
    /// Añade una hoja al final y la selecciona (opcional en la PoC, FR-009).
    NewSheet,
    /// Renombra una hoja.
    RenameSheet {
        /// Índice de la hoja.
        sheet: u32,
        /// Nombre nuevo.
        name: String,
    },
    /// Borra una hoja (no la única).
    DeleteSheet(u32),
    // --- edición (FR-005) ---
    /// Abre la edición de la celda activa. `seed` es lo ya tecleado (modo `Replace`) o lo que
    /// se añade al contenido actual (`Append`, `FormulaBar`).
    BeginEdit {
        /// Cómo se abre.
        mode: EditMode,
        /// Texto inicial tecleado, si lo hay.
        seed: Option<String>,
    },
    /// Actualiza el texto de la edición abierta (para que la celda y la barra de fórmulas
    /// muestren lo mismo). No toca el libro.
    SetEditText(String),
    /// Confirma la edición con `text` y mueve la celda activa según `advance`.
    CommitEdit {
        /// Texto final.
        text: String,
        /// Adónde va la celda activa (`Advance::enter`, `Advance::tab`).
        advance: Advance,
    },
    /// Descarta la edición (Esc).
    CancelEdit,
    // --- historial (FR-008) ---
    /// Deshace.
    Undo,
    /// Rehace.
    Redo,
    // --- portapapeles (FR-010), archivo (FR-011) e idioma (FR-006) ---
    /// Pega texto tabulado en la selección.
    PasteText(String),
    /// Guarda; con `path` es «Guardar como», sin él sobrescribe el archivo abierto.
    Save {
        /// Destino, o `None` para el archivo abierto.
        path: Option<PathBuf>,
    },
    /// Cambia el idioma de los nombres de función de las fórmulas.
    SetLang(Lang),
}

/// Lo que la interfaz necesita para repintar tras un comando (`plan.md` §5.5). Es pequeño y
/// viaja como JSON; los bloques de la cuadrícula **no** van aquí (los pide con
/// [`Core::draw_block`]). La interfaz decide qué pedir de nuevo comparando `data_version` con el
/// de su bloque en caché.
///
/// Filas y columnas son base 1 (A1 = `(1, 1)`), las hojas base 0. El orden de los campos es el
/// del JSON y es estable (ver el módulo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    /// Versión de los datos: sube con todo cambio de datos, de tamaños o de hoja activa. Un
    /// bloque con otra versión está viejo.
    pub data_version: u64,
    /// Hoja visible (base 0).
    pub sheet: u32,
    /// Celda activa `(fila, columna)`: el ancla de la selección, donde escribe el teclado.
    pub active: (i32, i32),
    /// Selección `(fila_ini, col_ini, fila_fin, col_fin)`, normalizada y con ambos extremos
    /// incluidos.
    pub range: (i32, i32, i32, i32),
    /// Desplazamiento `(x, y)` en px de hoja del borde superior izquierdo del área de celdas.
    pub scroll: (f64, f64),
    /// Extensión desplazable `(ancho, alto)` en px de hoja (para las barras).
    pub extent: (f64, f64),
    /// Edición abierta, si la hay (celda, modo, contenido original y texto).
    pub edit: Option<EditState>,
    /// Texto de la barra de fórmulas: el de la edición abierta o el contenido de la celda
    /// activa.
    pub formula_bar: String,
    /// Pestañas de las hojas visibles, en orden.
    pub sheets: Vec<SheetTab>,
    /// Idioma de los nombres de función de las fórmulas.
    pub lang: Lang,
    /// Ruta completa del archivo abierto o guardado (con conversión a UTF-8 si hiciera falta);
    /// `None` en un libro nuevo.
    pub path: Option<String>,
    /// ¿Se puede deshacer?
    pub can_undo: bool,
    /// ¿Se puede rehacer?
    pub can_redo: bool,
    /// ¿Hay cambios sin guardar?
    pub dirty: bool,
    /// Operación larga en curso. **Reservado:** el núcleo es síncrono, no hay estado
    /// intermedio que devolver y siempre es `None`. Lo rellena la interfaz (por ejemplo
    /// «guardando») mientras una operación larga tiene el candado de `Mutex<Core>`
    /// (`plan.md` §5.11); existe en el contrato para que ese día no cambie el JSON.
    pub busy: Option<String>,
    /// Aviso para el usuario. **Reservado:** el núcleo informa los fallos como [`CoreError`] y
    /// nunca lo rellena; lo usan las interfaces.
    pub message: Option<String>,
}

impl Core {
    /// Ejecuta un comando y devuelve el estado resultante para la interfaz.
    ///
    /// Es [`Core::execute`] seguido de [`Core::ui_state`]; no tiene lógica propia. Si el comando
    /// falla devuelve el error y el estado queda como lo dejó el método que falló (una edición
    /// que no se pudo confirmar sigue abierta, por ejemplo); la interfaz puede releerlo con
    /// [`Core::ui_state`].
    ///
    /// ```
    /// use std::sync::Mutex;
    /// use dokumentz_sheet::{Advance, Command, Core, EditMode, OpenOptions};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let core = Mutex::new(Core::new_empty(OpenOptions::default())?);
    /// let estado = {
    ///     let mut core = core.lock().unwrap();
    ///     core.apply(Command::BeginEdit { mode: EditMode::Replace, seed: Some("42".into()) })?;
    ///     core.apply(Command::CommitEdit { text: "42".into(), advance: Advance::Down })?
    /// };
    /// assert_eq!(estado.active, (2, 1)); // Intro baja a A2
    /// assert!(estado.dirty && estado.can_undo);
    /// # Ok(()) }
    /// ```
    pub fn apply(&mut self, cmd: Command) -> Result<UiState, CoreError> {
        self.execute(cmd)?;
        Ok(self.ui_state())
    }

    /// Estado actual para la interfaz (ver [`UiState`]).
    ///
    /// Recibe `&mut self` (el plan decía `&self`) porque la extensión desplazable usa la
    /// geometría de la hoja, que se construye la primera vez que se pide y se guarda en el
    /// núcleo. Es barato: no recorre celdas, y llamarlo dos veces seguidas da el mismo estado.
    pub fn ui_state(&mut self) -> UiState {
        self.sync_view_sheet();
        let seleccion = self.selection();
        let medidas = self.sheet_metrics();
        UiState {
            data_version: self.data_version(),
            sheet: seleccion.sheet,
            active: seleccion.active,
            range: seleccion.range,
            scroll: (self.view().scroll_x, self.view().scroll_y),
            extent: medidas.extent,
            edit: self.edit_state().cloned(),
            formula_bar: self.formula_bar(),
            sheets: self.sheet_tabs(),
            lang: self.lang(),
            path: self.path().map(|p| p.to_string_lossy().into_owned()),
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            dirty: self.is_dirty(),
            busy: None,
            message: None,
        }
    }

    /// Ejecuta un comando. Los valores no válidos de vista y navegación (un tamaño no finito, un
    /// punto fuera de la cuadrícula) no son error: se ignoran o se acotan como dicen los
    /// métodos que los ejecutan. Edición, hojas, pegado y guardado devuelven su error tipado.
    pub fn execute(&mut self, cmd: Command) -> Result<(), CoreError> {
        match cmd {
            Command::SetViewport { w, h, scale } => self.set_viewport(w, h, scale),
            Command::ScrollTo { x, y } => {
                self.scroll_to(x, y);
            }
            Command::ScrollBy { dx, dy } => {
                self.scroll_by(dx, dy);
            }
            Command::Click { x, y, extend } => {
                self.click(x, y, extend)?;
            }
            Command::DragTo { x, y } => self.drag_to(x, y)?,
            Command::Move { dir, extend } => self.move_active(dir, extend)?,
            Command::Edge { dir, extend } => self.go_to_edge(dir, extend)?,
            Command::Page { down, extend } => self.page(down, extend)?,
            Command::Home { ctrl } => self.home(ctrl)?,
            Command::SelectSheet(hoja) => self.select_sheet(hoja)?,
            Command::NewSheet => {
                self.new_sheet()?;
            }
            Command::RenameSheet { sheet, name } => self.rename_sheet(sheet, &name)?,
            Command::DeleteSheet(hoja) => self.delete_sheet(hoja)?,
            Command::BeginEdit { mode, seed } => self.begin_edit(mode, seed)?,
            Command::SetEditText(texto) => self.set_edit_text(texto),
            Command::CommitEdit { text, advance } => self.commit_edit(text, advance)?,
            Command::CancelEdit => self.cancel_edit(),
            Command::Undo => self.undo()?,
            Command::Redo => self.redo()?,
            Command::PasteText(texto) => self.paste_text(&texto)?,
            Command::Save { path } => self.save(path.as_deref())?,
            Command::SetLang(lang) => self.set_lang(lang)?,
        }
        Ok(())
    }
}
