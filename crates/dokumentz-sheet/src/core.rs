//! `Core`: el estado del documento y su única puerta de entrada (`plan.md` §5.2).
//!
//! E06 cubre abrir, la geometría, la vista y la lista de dibujo; E07 (`nav.rs`, `command.rs`)
//! añade la selección y la navegación; E08 la edición (`edit.rs`), el portapapeles
//! (`clipboard.rs`), el guardado (`save.rs`), las hojas y el idioma. E09 añade la fachada
//! (`Core::apply` y `Core::ui_state`, en `command.rs`) y la instrumentación (característica
//! `metrics`). Reglas para quien añada más:
//!
//! * Todo cambio de datos, de tamaños de fila/columna o de hoja activa debe llamar a
//!   `Core::mark_changed` (sube `data_version` e invalida la caché del bloque y marca el
//!   documento como modificado). Si cambian tamaños, además `invalidate_geometry`.
//! * `Core` no es `Sync` por sí mismo: la interfaz lo guarda en un `Mutex<Core>` (§5.11).
//! * Un tramo que valga la pena medir se abre con `tramo!(self, nombres::X)` (ver más abajo);
//!   sin la característica `metrics` no expande a nada.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ironcalc::error::XlsxError;
use ironcalc::export::save_xlsx_to_writer;
use ironcalc::import::{load_from_xlsx, load_from_xlsx_bytes};
use ironcalc_base::{Model, UserModel};
use serde::{Deserialize, Serialize};

#[cfg(feature = "metrics")]
use dokumentz_metrics::Recorder;

use crate::codec;
use crate::drawlist::{DrawList, DrawRequest, build_block};
use crate::edit::EditState;
use crate::error::CoreError;
use crate::geometry::SheetGeometry;
use crate::nav::DragMode;
use crate::view::{SheetMetrics, View};

/// Nombres de las series y contadores que escribe el núcleo en el `Recorder` (característica
/// `metrics`). Los comunes a las dos interfaces (`LOAD_MS`, `DRAW_BLOCK_MS`, `ENCODE_MS`,
/// `CELLS_IN_LIST`…) son los de `dokumentz_metrics::names`; los demás son del núcleo.
#[cfg(feature = "metrics")]
pub mod nombres {
    pub use dokumentz_metrics::names::*;

    /// Serie: guardar un `.xlsx` (serializar, escribir el temporal y renombrar).
    pub const SAVE_MS: &str = "save_ms";
    /// Serie: una evaluación completa del libro pedida con `Core::recalculate`.
    pub const EVALUATE_MS: &str = "evaluate_ms";
    /// Serie: `CommitEdit` completo (escribir, evaluar si la evaluación no está pausada, y
    /// avanzar la celda activa).
    pub const COMMIT_EDIT_MS: &str = "commit_edit_ms";
    /// Serie: celdas de cada bloque construido (la mediana y el p95 sirven para FR-003; el
    /// total acumulado está en el contador `CELLS_IN_LIST`).
    pub const CELLS_PER_BLOCK: &str = "cells_per_block";
    /// Contador: bloques construidos por `draw_block` (fallos de la caché de un bloque).
    pub const DRAW_BLOCK_BUILDS: &str = "draw_block_builds";
    /// Contador: llamadas a `draw_block` que se resolvieron con el bloque en caché.
    pub const DRAW_BLOCK_HITS: &str = "draw_block_hits";
}

/// Abre un tramo cronometrado del `Recorder` del núcleo hasta el final del bloque en que se
/// escribe. `tramo!(self, nombres::DRAW_BLOCK_MS)`.
///
/// Sin la característica `metrics`, o sin `Recorder` encendido, no hace nada (un `Recorder`
/// apagado ni siquiera se guarda en el núcleo, ver `Core::set_recorder`). La guardia
/// (`Span`) no toma prestado el núcleo: lleva su propia copia del `Arc`.
macro_rules! tramo {
    ($core:expr, $nombre:expr) => {
        #[cfg(feature = "metrics")]
        let __recorder = $core.recorder.clone();
        #[cfg(feature = "metrics")]
        let __tramo = __recorder.as_deref().map(|r| r.span($nombre));
    };
}
pub(crate) use tramo;

/// Idioma de los nombres de función de las fórmulas (`plan.md` §5.7). No decide los
/// separadores ni los decimales: eso lo decide el `locale` de [`OpenOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    /// Español (`SUMA`, `SI`…).
    Es,
    /// Inglés (`SUM`, `IF`…).
    En,
}

impl Lang {
    /// Identificador del idioma en IronCalc (`language_id`).
    pub fn id(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
        }
    }

    /// El otro idioma (la política de teclear de `plan.md` §5.7 prueba con él).
    pub fn other(self) -> Lang {
        match self {
            Lang::Es => Lang::En,
            Lang::En => Lang::Es,
        }
    }
}

/// Opciones para crear o abrir un documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenOptions {
    /// Idioma de los nombres de función.
    pub lang: Lang,
    /// `locale` de IronCalc (`"en"`, `"es"`…): separadores de argumentos y decimal al teclear
    /// (§5.7). Sigue a la región del sistema; en las pruebas es un valor fijo.
    pub locale: &'static str,
    /// Zona horaria (`"UTC"`).
    pub tz: &'static str,
}

impl Default for OpenOptions {
    /// Inglés, `locale` `en`, `UTC`: lo que usan las herramientas y las pruebas.
    fn default() -> Self {
        OpenOptions {
            lang: Lang::En,
            locale: "en",
            tz: "UTC",
        }
    }
}

impl OpenOptions {
    /// Opciones con el `locale` que corresponde a la región del sistema
    /// ([`locale_for_region`]). La interfaz lee la región (en macOS, `AppleLocale`) y la pasa
    /// aquí; el núcleo no la lee.
    pub fn from_region(lang: Lang, region: &str) -> OpenOptions {
        OpenOptions {
            lang,
            locale: locale_for_region(region),
            tz: "UTC",
        }
    }
}

/// Territorios hispanohablantes que usan la coma como separador decimal (CLDR). Los demás
/// (México, Estados Unidos, Perú, Centroamérica…) usan el punto.
const ES_DECIMAL_COMA: &[&str] = &["ES", "AR", "BO", "CL", "CO", "CR", "EC", "PY", "UY", "VE"];

/// Idiomas con coma decimal sin `locale` propio en IronCalc, que se asignan al más cercano.
const OTROS_COMA_ES: &[&str] = &["ca", "gl", "eu"];
const OTROS_COMA_DE: &[&str] = &[
    "pt", "nl", "ru", "pl", "cs", "sk", "sv", "da", "nb", "nn", "no", "fi", "tr", "id", "el", "hu",
    "ro", "bg", "uk", "vi", "hr", "sl", "sr", "lt",
];

/// `locale` de IronCalc (`"en"`, `"en-GB"`, `"es"`, `"de"`, `"fr"`, `"it"`) que corresponde a
/// una región del sistema (`plan.md` §5.7, decisión de CP-0: el `locale` sigue a la región,
/// como en Excel). Decide el separador de argumentos y el decimal al teclear, no los nombres de
/// función (eso es [`Lang`]).
///
/// Es una función pura: acepta `es_MX`, `es-MX`, `es_MX.UTF-8`, `de_DE@euro`, `zh-Hans-CN`…
/// Si la región usa coma decimal (`es_ES`, `es_AR`, `de_DE`, `fr_FR`, `it_IT`…) da `es`, `de`,
/// `fr` o `it`; si usa punto (`es_MX`, `es_US`, `en_US`, `de_CH`…) da `en` (`en-GB` para
/// `en_GB`). Una región vacía o desconocida da `en`.
///
/// Se decide por el idioma y, para el español y los idiomas con variantes suizas, por el
/// territorio; un `en_DE` (inglés con formatos alemanes) da `en`.
pub fn locale_for_region(region: &str) -> &'static str {
    let base = region.split(['.', '@']).next().unwrap_or("").trim();
    let mut partes = base.split(['_', '-']).filter(|p| !p.is_empty());
    let idioma = partes.next().unwrap_or("").to_ascii_lowercase();
    // El territorio es la última parte de dos letras o tres cifras (`419`, América Latina); así
    // se salta la escritura de `zh-Hans-CN`.
    let territorio = partes
        .rfind(|p| {
            (p.len() == 2 && p.chars().all(|c| c.is_ascii_alphabetic()))
                || (p.len() == 3 && p.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();
    match idioma.as_str() {
        "es" if territorio.is_empty() || ES_DECIMAL_COMA.contains(&territorio.as_str()) => "es",
        "es" => "en",
        "de" if matches!(territorio.as_str(), "CH" | "LI") => "en",
        "de" => "de",
        "fr" => "fr",
        "it" if territorio == "CH" => "en",
        "it" => "it",
        "en" if territorio == "GB" => "en-GB",
        otro if OTROS_COMA_ES.contains(&otro) => "es",
        otro if OTROS_COMA_DE.contains(&otro) => "de",
        _ => "en",
    }
}

/// Una pestaña de hoja para la interfaz (`UiState.sheets`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetTab {
    /// Índice de la hoja (base 0), el que recibe `SelectSheet`.
    pub index: u32,
    /// Nombre de la hoja.
    pub name: String,
    /// Color de la pestaña como `#RRGGBB`; `None` si no tiene.
    pub color: Option<String>,
    /// ¿Es la hoja visible?
    pub active: bool,
}

/// Estado de un documento abierto.
pub struct Core {
    pub(crate) um: UserModel<'static>,
    lang: Lang,
    /// `locale` y zona horaria con que se abrió (el modelo de borrador de `edit.rs` los copia).
    pub(crate) locale: &'static str,
    pub(crate) tz: &'static str,
    /// Edición en curso (`edit.rs`).
    pub(crate) edit: Option<EditState>,
    path: Option<PathBuf>,
    dirty: bool,
    /// Sube con todo cambio de datos, de tamaños o de hoja activa.
    data_version: u64,
    /// Geometría por hoja, perezosa; se descarta al cambiar tamaños o la estructura de hojas.
    geometry: Vec<Option<Arc<SheetGeometry>>>,
    pub(crate) view: View,
    /// Qué arrastra el puntero tras el último clic (celdas, filas o columnas; `nav.rs`).
    pub(crate) drag: DragMode,
    /// Último bloque construido (la caché de `draw_block`).
    block: Option<Arc<DrawList>>,
    /// Celdas del último bloque devuelto por `draw_block` (invariante FR-003).
    drawn_cells_last: u32,
    /// ¿Está pausada la evaluación automática de cada escritura? (`pause_evaluation`).
    evaluation_paused: bool,
    /// Registro de mediciones internas; `None` si no hay o está apagado.
    #[cfg(feature = "metrics")]
    pub(crate) recorder: Option<Arc<Recorder>>,
}

impl Core {
    fn desde_usermodel(um: UserModel<'static>, opts: OpenOptions, path: Option<PathBuf>) -> Core {
        let mut core = Core {
            um,
            lang: opts.lang,
            locale: opts.locale,
            tz: opts.tz,
            edit: None,
            path,
            dirty: false,
            data_version: 0,
            geometry: Vec::new(),
            view: View::default(),
            drag: DragMode::default(),
            block: None,
            drawn_cells_last: 0,
            evaluation_paused: false,
            #[cfg(feature = "metrics")]
            recorder: None,
        };
        let (w, h) = (core.view.cells_w(), core.view.cells_h());
        core.um.set_window_width(w);
        core.um.set_window_height(h);
        // La hoja de la vista es siempre la seleccionada en el motor.
        core.view.sheet = core.um.get_selected_sheet();
        core
    }

    /// Libro nuevo con una hoja vacía.
    pub fn new_empty(opts: OpenOptions) -> Result<Core, CoreError> {
        let um = UserModel::new_empty("Libro", opts.locale, opts.tz, opts.lang.id())
            .map_err(CoreError::Motor)?;
        Ok(Core::desde_usermodel(um, opts, None))
    }

    /// Abre un `.xlsx` del disco.
    ///
    /// Los errores de ruta son tipados y no pasan por el motor: [`CoreError::NoExiste`],
    /// [`CoreError::SinPermiso`] y [`CoreError::NoEsXlsx`] (carpeta, archivo corrupto o
    /// truncado). Un fallo interno del motor con un archivo malformado se captura y se
    /// devuelve como `NoEsXlsx`; nunca llega a la interfaz como pánico.
    ///
    /// **No evalúa**: tras abrir se ven los valores que el archivo trae guardados (hallazgo de
    /// T006). Así abrir cuesta solo la lectura, y los valores que calculó Excel no se
    /// sustituyen por errores de funciones que IronCalc no tenga. Quien necesite el cálculo
    /// completo (por ejemplo `compat-check`, o un archivo sin valores guardados) llama a
    /// [`Core::recalculate`].
    pub fn open_xlsx(path: &Path, opts: OpenOptions) -> Result<Core, CoreError> {
        comprobar_lectura(path)?;
        let modelo = capturar_panico(|| match path.to_str() {
            // Camino normal: el motor lee el archivo en flujo (no lo copia entero a memoria).
            Some(ruta) => {
                load_from_xlsx(ruta, opts.locale, opts.tz, opts.lang.id()).map_err(error_de_motor)
            }
            // Ruta que no es UTF-8: el motor no puede recibirla como texto; se leen los bytes.
            None => {
                let bytes = std::fs::read(path).map_err(|e| error_de_io(path, &e))?;
                let nombre = path
                    .file_stem()
                    .map_or_else(|| "Libro".into(), |n| n.to_string_lossy().into_owned());
                modelo_desde_bytes(&bytes, &nombre, opts)
            }
        })?;
        Ok(Core::desde_usermodel(
            UserModel::from_model(modelo),
            opts,
            Some(path.to_path_buf()),
        ))
    }

    /// Abre un `.xlsx` que ya está en memoria (`load_from_xlsx_bytes`). `nombre` es el nombre
    /// del libro (sin extensión). Mismos errores y misma política de evaluación que
    /// [`Core::open_xlsx`]; el documento queda sin ruta.
    pub fn open_xlsx_bytes(
        bytes: &[u8],
        nombre: &str,
        opts: OpenOptions,
    ) -> Result<Core, CoreError> {
        let modelo = capturar_panico(|| modelo_desde_bytes(bytes, nombre, opts))?;
        Ok(Core::desde_usermodel(
            UserModel::from_model(modelo),
            opts,
            None,
        ))
    }

    /// Conecta el núcleo con un `Recorder` (característica `metrics`, FR-012): a partir de aquí
    /// el núcleo anota en él los tramos de `draw_block` (`draw_block_ms`), codificar
    /// ([`Core::encode_block`], `encode_ms`), evaluar (`evaluate_ms`), guardar (`save_ms`) y
    /// `CommitEdit` (`commit_edit_ms`), más los contadores `cells_in_list`,
    /// `draw_block_builds` y `draw_block_hits`.
    ///
    /// Un `Recorder` **apagado** ([`Recorder::is_on`] es `false`) se descarta: el núcleo no
    /// guarda nada, no lee el reloj y no reserva memoria. El núcleo nunca escribe el JSON: lo
    /// escribe quien creó el `Recorder` (`Recorder::write_json`) al salir o al terminar el guion.
    #[cfg(feature = "metrics")]
    pub fn set_recorder(&mut self, recorder: Arc<Recorder>) {
        self.recorder = recorder.is_on().then_some(recorder);
    }

    /// El `Recorder` conectado con `Core::set_recorder`, si hay uno encendido.
    #[cfg(feature = "metrics")]
    pub fn recorder(&self) -> Option<&Arc<Recorder>> {
        self.recorder.as_ref()
    }

    /// [`Core::open_xlsx`] midiendo la carga en la serie `load_ms` de `recorder`, que además
    /// queda conectado al núcleo devuelto (`Core::set_recorder`). Un fallo al abrir no deja
    /// muestra. Característica `metrics`.
    #[cfg(feature = "metrics")]
    pub fn open_xlsx_with_recorder(
        path: &Path,
        opts: OpenOptions,
        recorder: Arc<Recorder>,
    ) -> Result<Core, CoreError> {
        let tramo = recorder.span(nombres::LOAD_MS);
        match Core::open_xlsx(path, opts) {
            Ok(mut core) => {
                drop(tramo);
                core.set_recorder(recorder);
                Ok(core)
            }
            Err(e) => {
                tramo.cancel();
                Err(e)
            }
        }
    }

    /// [`Core::open_xlsx_bytes`] midiendo la carga en `load_ms` (ver
    /// [`Core::open_xlsx_with_recorder`]). Característica `metrics`.
    #[cfg(feature = "metrics")]
    pub fn open_xlsx_bytes_with_recorder(
        bytes: &[u8],
        nombre: &str,
        opts: OpenOptions,
        recorder: Arc<Recorder>,
    ) -> Result<Core, CoreError> {
        let tramo = recorder.span(nombres::LOAD_MS);
        match Core::open_xlsx_bytes(bytes, nombre, opts) {
            Ok(mut core) => {
                drop(tramo);
                core.set_recorder(recorder);
                Ok(core)
            }
            Err(e) => {
                tramo.cancel();
                Err(e)
            }
        }
    }

    /// Serializa el libro a un `.xlsx` en memoria (`save_xlsx_to_writer` sobre un búfer).
    /// Es la primitiva sobre la que E08 construye el guardado atómico (archivo temporal más
    /// `rename`). No cambia el estado del documento.
    pub fn to_xlsx_bytes(&self) -> Result<Vec<u8>, CoreError> {
        let cursor = std::io::Cursor::new(Vec::new());
        let cursor = save_xlsx_to_writer(self.um.get_model(), cursor).map_err(error_de_motor)?;
        Ok(cursor.into_inner())
    }

    /// Idioma de los nombres de función.
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Ruta del archivo abierto, si lo hay.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// ¿Hay cambios sin guardar?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Versión de datos actual. Cambia con todo cambio de datos, de tamaños o de hoja.
    pub fn data_version(&self) -> u64 {
        self.data_version
    }

    /// Nombres de las hojas, en orden.
    pub fn sheet_names(&self) -> Vec<String> {
        self.um
            .get_model()
            .workbook
            .worksheets
            .iter()
            .map(|h| h.name.clone())
            .collect()
    }

    /// Texto de una celda tal como se ve (con su formato de número). Vacío si no tiene dato.
    pub fn cell_text(&self, sheet: u32, row: i32, col: i32) -> Result<String, CoreError> {
        self.um
            .get_formatted_cell_value(sheet, row, col)
            .map_err(CoreError::Motor)
    }

    /// Recalcula todo el libro (`evaluate`). No marca el documento como modificado.
    pub fn recalculate(&mut self) {
        tramo!(self, nombres::EVALUATE_MS);
        self.um.evaluate();
        self.data_version += 1;
    }

    /// Pausa la evaluación automática: mientras dure, escribir una celda (`set_cell_input`,
    /// `CommitEdit`, `PasteText`, `Undo`, `Redo`…) **no** recalcula el libro y las fórmulas
    /// conservan el valor de la última evaluación. Sirve para agrupar varias escrituras con una
    /// sola evaluación (`pause_evaluation` → escrituras → `resume_evaluation` →
    /// [`Core::recalculate`]) y para construir libros grandes de pruebas sin pagar un recorrido
    /// de todas las celdas por escritura (R-04 de `plan.md`, opción a).
    ///
    /// Con la pausa activa los valores que ve la interfaz pueden estar atrasados: quien pausa
    /// debe reanudar y recalcular. No se guarda en el historial ni en el archivo.
    pub fn pause_evaluation(&mut self) {
        self.um.pause_evaluation();
        self.evaluation_paused = true;
    }

    /// Reanuda la evaluación automática. **No evalúa**: las escrituras hechas durante la pausa
    /// siguen sin reflejarse en las fórmulas hasta la siguiente evaluación; llame a
    /// [`Core::recalculate`] después.
    pub fn resume_evaluation(&mut self) {
        self.um.resume_evaluation();
        self.evaluation_paused = false;
    }

    /// ¿Está pausada la evaluación automática? (ver [`Core::pause_evaluation`]).
    pub fn is_evaluation_paused(&self) -> bool {
        self.evaluation_paused
    }

    /// Escribe `text` en una celda como si el usuario lo tecleara y evalúa el libro (una vez).
    /// Es la primitiva de escritura de `CommitEdit` (`edit.rs`): aplica la política de idioma
    /// de `plan.md` §5.7 (una fórmula tecleada con los nombres del otro idioma se acepta),
    /// marca el documento como modificado y descarta la geometría si la escritura amplía el
    /// rango usado o cambia la altura de la fila. No toca la selección ni la vista.
    pub fn set_cell_input(
        &mut self,
        sheet: u32,
        row: i32,
        col: i32,
        text: &str,
    ) -> Result<(), CoreError> {
        self.write_input(sheet, row, col, text)
    }

    /// Sube `data_version` (invalida el bloque en caché) y marca el documento como modificado.
    pub(crate) fn mark_changed(&mut self) {
        self.data_version += 1;
        self.dirty = true;
    }

    /// Sube `data_version` sin marcar el documento como modificado (cambios de presentación,
    /// como el idioma de las fórmulas).
    pub(crate) fn bump_version(&mut self) {
        self.data_version += 1;
    }

    /// Fija el idioma que ve el núcleo (el motor ya lo cambió).
    pub(crate) fn set_lang_state(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// Tras guardar: el documento queda limpio y apunta a `path`.
    pub(crate) fn mark_saved(&mut self, path: PathBuf) {
        self.path = Some(path);
        self.dirty = false;
    }

    /// Avisa de que se escribió en `(sheet, row, col)`: si la celda queda fuera del rango usado
    /// que conoce la geometría ya construida, o si la fila cambió de altura respecto a
    /// `alto_antes`, se descarta la geometría (se reconstruye perezosamente; con 2 M de celdas
    /// cuesta ~9 ms, por eso no se hace en cada tecla). Si aún no hay geometría de esa hoja no
    /// hay nada que descartar.
    pub(crate) fn note_cell_write(
        &mut self,
        sheet: u32,
        row: i32,
        col: i32,
        alto_antes: Option<f64>,
    ) {
        let cambio_alto = alto_antes.is_some_and(|antes| {
            self.um
                .get_row_height(sheet, row)
                .is_ok_and(|ahora| (ahora - antes).abs() > f64::EPSILON)
        });
        let fuera_del_rango = self
            .geometry
            .get(sheet as usize)
            .and_then(Option::as_ref)
            .is_some_and(|g| row > g.used_rows() || col > g.used_cols());
        if cambio_alto || fuera_del_rango {
            self.invalidate_geometry();
        }
    }

    /// Descarta la geometría calculada de todas las hojas (tras cambiar tamaños o la
    /// estructura de hojas). Se reconstruye perezosamente.
    pub fn invalidate_geometry(&mut self) {
        self.geometry.clear();
        self.block = None;
    }

    /// Geometría de una hoja, construida la primera vez que se pide.
    pub fn geometry(&mut self, sheet: u32) -> Result<Arc<SheetGeometry>, CoreError> {
        let hojas = &self.um.get_model().workbook.worksheets;
        let ws = hojas
            .get(sheet as usize)
            .ok_or(CoreError::HojaInvalida(sheet))?;
        if self.geometry.len() != hojas.len() {
            self.geometry = vec![None; hojas.len()];
        }
        let hueco = &mut self.geometry[sheet as usize];
        Ok(hueco
            .get_or_insert_with(|| Arc::new(SheetGeometry::from_worksheet(ws)))
            .clone())
    }

    /// Hoja seleccionada en el motor (base 0); es también `view().sheet`.
    pub fn selected_sheet(&self) -> u32 {
        self.um.get_selected_sheet()
    }

    /// Pestañas de las hojas visibles, en orden, con la activa marcada (`UiState.sheets`).
    /// Las hojas ocultas del archivo no aparecen (como en Office); siguen existiendo y se
    /// pueden seleccionar por su índice.
    pub fn sheet_tabs(&self) -> Vec<SheetTab> {
        let activa = self.um.get_selected_sheet();
        self.um
            .get_worksheets_properties()
            .into_iter()
            .enumerate()
            .filter(|(_, p)| p.state == "visible")
            .map(|(i, p)| {
                let color = self.um.resolve_color(&p.color);
                SheetTab {
                    index: i as u32,
                    name: p.name,
                    color: (!color.is_empty()).then_some(color),
                    active: i as u32 == activa,
                }
            })
            .collect()
    }

    /// Cambia la hoja visible (`SelectSheet`). Cada hoja recuerda su selección y su celda
    /// superior izquierda en el motor, así que al volver a ella se recupera lo que se veía.
    /// Una edición abierta sigue apuntando a su propia celda. Falla con
    /// [`CoreError::HojaInvalida`] si el índice no existe.
    pub fn select_sheet(&mut self, sheet: u32) -> Result<(), CoreError> {
        self.sync_view_sheet();
        if sheet as usize >= self.um.get_model().workbook.worksheets.len() {
            return Err(CoreError::HojaInvalida(sheet));
        }
        self.um
            .set_selected_sheet(sheet)
            .map_err(CoreError::Motor)?;
        self.sync_view_sheet();
        Ok(())
    }

    /// Añade una hoja vacía al final y la selecciona. Devuelve su índice. Entra en el historial.
    pub fn new_sheet(&mut self) -> Result<u32, CoreError> {
        self.um.new_sheet().map_err(CoreError::Motor)?;
        self.after_sheet_structure_change();
        Ok(self.um.get_selected_sheet())
    }

    /// Renombra una hoja (las fórmulas que la citan se actualizan). Entra en el historial.
    pub fn rename_sheet(&mut self, sheet: u32, name: &str) -> Result<(), CoreError> {
        if sheet as usize >= self.um.get_model().workbook.worksheets.len() {
            return Err(CoreError::HojaInvalida(sheet));
        }
        self.um
            .rename_sheet(sheet, name)
            .map_err(CoreError::Motor)?;
        self.mark_changed();
        Ok(())
    }

    /// Borra una hoja. Falla (error del motor) si es la única. Entra en el historial.
    pub fn delete_sheet(&mut self, sheet: u32) -> Result<(), CoreError> {
        if sheet as usize >= self.um.get_model().workbook.worksheets.len() {
            return Err(CoreError::HojaInvalida(sheet));
        }
        self.um.delete_sheet(sheet).map_err(CoreError::Motor)?;
        self.after_sheet_structure_change();
        Ok(())
    }

    /// Tras añadir, borrar o deshacer hojas: las geometrías por índice ya no valen, el documento
    /// cambió y la hoja visible puede ser otra con el mismo índice (se fuerza la resincronización
    /// de la vista).
    pub(crate) fn after_sheet_structure_change(&mut self) {
        self.invalidate_geometry();
        self.mark_changed();
        self.view.sheet = u32::MAX;
        self.sync_view_sheet();
    }

    /// Hace que `view.sheet` sea la hoja seleccionada en el motor. Si cambió, sube
    /// `data_version`, descarta el bloque y recupera el desplazamiento de esa hoja a partir de
    /// su celda superior izquierda (el motor la recuerda por hoja). Lo llama toda operación de
    /// `nav.rs` antes de actuar, y debe llamarlo quien cambie la hoja seleccionada (E08).
    pub(crate) fn sync_view_sheet(&mut self) {
        let hoja = self.um.get_selected_sheet();
        if hoja == self.view.sheet {
            return;
        }
        self.view.sheet = hoja;
        self.data_version += 1;
        self.block = None;
        let vista = self.um.get_selected_view();
        let (arriba, izquierda) = (vista.top_row, vista.left_column);
        if let Ok(g) = self.geometry(hoja) {
            self.set_scroll_nav(Some(g.col_x(izquierda)), Some(g.row_y(arriba)));
        }
    }

    /// Deja la celda superior izquierda de `UserModel` donde dice el desplazamiento en píxeles:
    /// la fila y la columna que contienen el borde superior izquierdo del área de celdas.
    pub(crate) fn sync_um_scroll(&mut self) {
        self.sync_view_sheet();
        let Ok(g) = self.geometry(self.view.sheet) else {
            return;
        };
        let (arriba, izquierda) = (g.row_at(self.view.scroll_y), g.col_at(self.view.scroll_x));
        // Solo falla si la hoja no existe, y `geometry` acaba de comprobar que existe.
        let _ = self.um.set_top_left_visible_cell(arriba, izquierda);
    }

    /// Fija el desplazamiento de un eje (o de los dos) acotado solo al tamaño de la hoja, sin
    /// el tope de `sheet_metrics` (que no deja pasar del rango usado más una pantalla): la
    /// navegación por teclado puede llegar a cualquier celda y la extensión de
    /// `sheet_metrics` se amplía para cubrir lo que se ve. Mantiene `UserModel` coherente.
    pub(crate) fn set_scroll_nav(&mut self, x: Option<f64>, y: Option<f64>) {
        let Ok(g) = self.geometry(self.view.sheet) else {
            return;
        };
        if let Some(x) = x.filter(|x| x.is_finite()) {
            let max = (g.total_w() - self.view.cells_w()).max(0.0);
            self.view.scroll_x = x.clamp(0.0, max);
        }
        if let Some(y) = y.filter(|y| y.is_finite()) {
            let max = (g.total_h() - self.view.cells_h()).max(0.0);
            self.view.scroll_y = y.clamp(0.0, max);
        }
        self.sync_um_scroll();
    }

    /// Vista actual (hoja, desplazamiento y tamaño del control).
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Fija el tamaño del control de la cuadrícula (cabeceras incluidas, px lógicos) y la
    /// escala de la pantalla. Valores no finitos o negativos se toman como 0 (la escala, como 1).
    /// Acota el desplazamiento al nuevo tamaño y avisa al motor (lo usa para RePág/AvPág).
    pub fn set_viewport(&mut self, w: f32, h: f32, scale: f32) {
        let ok = |v: f32| if v.is_finite() && v > 0.0 { v } else { 0.0 };
        self.view.viewport_w = ok(w);
        self.view.viewport_h = ok(h);
        self.view.scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        self.um.set_window_width(self.view.cells_w());
        self.um.set_window_height(self.view.cells_h());
        self.scroll_to(self.view.scroll_x, self.view.scroll_y);
    }

    /// Desplaza a `(x, y)` px de hoja, acotado a `[0, extensión - área de celdas]`. Devuelve el
    /// desplazamiento resultante. Un valor no finito deja ese eje como estaba.
    pub fn scroll_to(&mut self, x: f64, y: f64) -> (f64, f64) {
        let max = self.sheet_metrics().max_scroll;
        if x.is_finite() {
            self.view.scroll_x = x.clamp(0.0, max.0);
        }
        if y.is_finite() {
            self.view.scroll_y = y.clamp(0.0, max.1);
        }
        self.sync_um_scroll();
        (self.view.scroll_x, self.view.scroll_y)
    }

    /// Desplaza `(dx, dy)` px respecto al desplazamiento actual (mismo acotado).
    pub fn scroll_by(&mut self, dx: f64, dy: f64) -> (f64, f64) {
        self.scroll_to(self.view.scroll_x + dx, self.view.scroll_y + dy)
    }

    /// Medidas de la hoja visible: extensión desplazable y rango usado. La extensión incluye
    /// siempre lo que se está viendo y la selección actual, para que la celda activa sea
    /// alcanzable aunque esté fuera del rango con datos y para que la barra de desplazamiento
    /// represente adonde llegó la navegación por teclado. Una selección de filas (o columnas)
    /// enteras no ensancha (ni alarga) la extensión en el eje que abarca por completo.
    pub fn sheet_metrics(&mut self) -> SheetMetrics {
        let geom = self
            .geometry(self.view.sheet)
            .unwrap_or_else(|_| Arc::new(SheetGeometry::default()));
        let (mut ext_w, mut ext_h) = geom.extent(&self.view);
        let sel = self.selection();
        if sel.sheet == self.view.sheet {
            let (_, _, fila_fin, col_fin) = sel.range;
            if !sel.is_entire_rows() {
                ext_w = ext_w.max(geom.col_x(col_fin + 1));
            }
            if !sel.is_entire_cols() {
                ext_h = ext_h.max(geom.row_y(fila_fin + 1));
            }
        }
        ext_w = ext_w
            .max(self.view.scroll_x + self.view.cells_w())
            .min(geom.total_w());
        ext_h = ext_h
            .max(self.view.scroll_y + self.view.cells_h())
            .min(geom.total_h());
        SheetMetrics {
            extent: (ext_w, ext_h),
            max_scroll: (
                (ext_w - self.view.cells_w()).max(0.0),
                (ext_h - self.view.cells_h()).max(0.0),
            ),
            used_rows: geom.used_rows(),
            used_cols: geom.used_cols(),
        }
    }

    /// Bloque de la cuadrícula para pintar el área `req`.
    ///
    /// Devuelve el bloque anterior (el mismo `Arc`) si el área visible pedida cabe en él, es
    /// de la misma hoja y `data_version` no cambió; si no, construye uno nuevo, que cubre el
    /// área más los márgenes de `req`. Falla solo si la hoja no existe.
    ///
    /// Con la característica `metrics` y un `Recorder` encendido (`Core::set_recorder`), la
    /// serie `draw_block_ms` recoge **solo la construcción** de bloques (incluida la geometría
    /// si había que construirla): una respuesta de la caché cuesta nanosegundos y diluiría los
    /// percentiles; se cuenta aparte (`draw_block_hits`).
    pub fn draw_block(&mut self, req: DrawRequest) -> Result<Arc<DrawList>, CoreError> {
        if let Some(bloque) = &self.block
            && bloque.data_version == self.data_version
            && bloque.covers(&req)
        {
            self.drawn_cells_last = bloque.cells.len() as u32;
            #[cfg(feature = "metrics")]
            if let Some(r) = &self.recorder {
                r.count(nombres::DRAW_BLOCK_HITS, 1);
            }
            return Ok(Arc::clone(bloque));
        }
        tramo!(self, nombres::DRAW_BLOCK_MS);
        let geom = self.geometry(req.sheet)?;
        let nuevo = Arc::new(build_block(&self.um, &geom, &req, self.data_version)?);
        self.drawn_cells_last = nuevo.cells.len() as u32;
        #[cfg(feature = "metrics")]
        if let Some(r) = &self.recorder {
            r.count(nombres::DRAW_BLOCK_BUILDS, 1);
            r.count(nombres::CELLS_IN_LIST, u64::from(self.drawn_cells_last));
            r.sample(nombres::CELLS_PER_BLOCK, f64::from(self.drawn_cells_last));
        }
        self.block = Some(Arc::clone(&nuevo));
        Ok(nuevo)
    }

    /// Celdas del último bloque que devolvió [`Core::draw_block`] (instrumentación y FR-003).
    pub fn drawn_cells_last(&self) -> u32 {
        self.drawn_cells_last
    }

    /// Codifica una lista de dibujo al formato binario v1 (`plan.md` §5.4). No mide: quien
    /// quiera el tramo en el `Recorder` usa [`Core::encode_block`].
    pub fn encode(dl: &DrawList) -> Vec<u8> {
        codec::encode(dl)
    }

    /// Igual que [`Core::encode`] pero, con la característica `metrics` y un `Recorder`
    /// encendido, anota la duración en la serie `encode_ms`. Sin la característica es
    /// exactamente `encode`. Es lo que llama la interfaz de Tauri antes de enviar el bloque.
    pub fn encode_block(&self, dl: &DrawList) -> Vec<u8> {
        tramo!(self, nombres::ENCODE_MS);
        codec::encode(dl)
    }
}

/// `Model` desde los bytes de un `.xlsx`.
fn modelo_desde_bytes(
    bytes: &[u8],
    nombre: &str,
    opts: OpenOptions,
) -> Result<Model<'static>, CoreError> {
    let libro =
        load_from_xlsx_bytes(bytes, nombre, opts.locale, opts.tz).map_err(error_de_motor)?;
    Model::from_workbook(libro, opts.lang.id()).map_err(CoreError::Motor)
}

/// Ejecuta una carga y convierte un pánico del motor en `NoEsXlsx`. Solo atrapa si el perfil
/// desenrolla (el valor por defecto de Cargo): con `panic = "abort"` no hay nada que capturar.
pub(crate) fn capturar_panico<T>(f: impl FnOnce() -> Result<T, CoreError>) -> Result<T, CoreError> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| {
        Err(CoreError::NoEsXlsx(
            "el motor falló al leer el contenido del archivo".to_string(),
        ))
    })
}

/// Traduce un error del motor: de lectura/escritura → `Io`; archivo mal formado → `NoEsXlsx`.
fn error_de_motor(e: XlsxError) -> CoreError {
    match e {
        XlsxError::IO(s) => CoreError::Io(s),
        XlsxError::Zip(s) | XlsxError::Xml(s) => CoreError::NoEsXlsx(s),
        otro => CoreError::Motor(otro.to_string()),
    }
}

pub(crate) fn error_de_io(path: &Path, e: &std::io::Error) -> CoreError {
    match e.kind() {
        std::io::ErrorKind::NotFound => CoreError::NoExiste(path.to_path_buf()),
        std::io::ErrorKind::PermissionDenied => CoreError::SinPermiso(path.to_path_buf()),
        _ => CoreError::Io(format!("{}: {e}", path.display())),
    }
}

/// Comprueba antes de llamar al motor que la ruta existe, es un archivo y se puede leer.
fn comprobar_lectura(path: &Path) -> Result<(), CoreError> {
    let meta = std::fs::metadata(path).map_err(|e| error_de_io(path, &e))?;
    if meta.is_dir() {
        return Err(CoreError::NoEsXlsx("la ruta es una carpeta".to_string()));
    }
    std::fs::File::open(path).map_err(|e| error_de_io(path, &e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_extension_incluye_la_seleccion_aunque_este_fuera_de_los_datos() {
        let mut core = Core::new_empty(OpenOptions::default()).unwrap();
        core.set_viewport(1000.0, 600.0, 1.0);
        // Hoja vacía: solo una pantalla.
        assert_eq!(core.sheet_metrics().max_scroll, (0.0, 0.0));
        // El usuario deja la celda activa en la fila 500, columna 40 (por ahora solo el motor
        // puede moverla; E07 añade la navegación).
        core.um.set_selected_cell(500, 40).unwrap();
        let m = core.sheet_metrics();
        let g = core.geometry(0).unwrap();
        assert!(
            m.extent.0 >= g.col_x(41) && m.extent.1 >= g.row_y(501),
            "{m:?}"
        );
        // El desplazamiento puede llegar hasta que esa celda se vea abajo y a la derecha.
        assert!(m.max_scroll.1 >= g.row_y(501) - core.view().cells_h());
        assert!(m.max_scroll.0 >= g.col_x(41) - core.view().cells_w());
        assert_eq!(core.scroll_to(1e9, 1e9), m.max_scroll);
    }

    #[test]
    fn invalidar_la_geometria_descarta_tambien_el_bloque() {
        let mut core = Core::new_empty(OpenOptions::default()).unwrap();
        let req = core.view().request(0.0, 0.0);
        let a = core.draw_block(req).unwrap();
        core.invalidate_geometry();
        let b = core.draw_block(req).unwrap();
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn core_y_la_lista_de_dibujo_cruzan_hilos() {
        // `Core` va en un `Mutex<Core>` y se usa desde hilos de trabajo (plan.md §5.11); los
        // bloques se comparten como `Arc<DrawList>`.
        fn es_send<T: Send>() {}
        fn es_send_y_sync<T: Send + Sync>() {}
        es_send::<Core>();
        es_send_y_sync::<std::sync::Mutex<Core>>();
        es_send_y_sync::<Arc<DrawList>>();
    }
}
