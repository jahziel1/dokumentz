//! Edición de celdas, idioma de las fórmulas e historial (`plan.md` §5.5–§5.7).
//!
//! # Edición (FR-005)
//!
//! Una edición es una sesión ([`EditState`]) sobre la celda activa: `BeginEdit` la abre con el
//! texto que corresponda al modo, `SetEditText` lo actualiza (para que la celda y la barra de
//! fórmulas muestren lo mismo, [`Core::formula_bar`]) y `CommitEdit` lo escribe con la política
//! de idioma y mueve la celda activa según [`Advance`]. `CancelEdit` descarta la sesión: el
//! libro no se tocó hasta el `CommitEdit`, así que Esc conserva el valor anterior sin pasar
//! por el historial. `CommitEdit` sin sesión abierta escribe en la celda activa.
//!
//! La regla de avance es la de Office: Intro baja (Mayús+Intro sube), Tab va a la derecha
//! (Mayús+Tab a la izquierda) — [`Advance::enter`] y [`Advance::tab`]. Con un rango de varias
//! celdas seleccionado el avance recorre el rango sin cambiarlo y da la vuelta al llegar al
//! final (como Excel; `[NO VERIFICADO]` en ONLYOFFICE, `plan.md` §5.6); con una sola celda
//! mueve la celda activa. Las filas y columnas ocultas se saltan.
//!
//! # Idioma de las fórmulas (FR-006, `plan.md` §5.7)
//!
//! Una fórmula tecleada con los nombres de función del otro idioma (`=SUM(...)` en una sesión
//! `es`) debe aceptarse. IronCalc guarda las fórmulas en inglés y las muestra en el idioma
//! activo, pero **solo entiende al teclear** los nombres del idioma activo: con otros nombres
//! deja `#¿NOMBRE?` (`#NAME?` en inglés) y conserva el texto con el nombre en minúsculas.
//!
//! T024 midió dos formas de aceptarlas sobre un A2 sintético de 100 000 × 20 con 10 000
//! fórmulas, en `--release` (`results/dev/T024.json`, `tests/formulas_es_en.rs`). Una edición
//! normal cuesta una evaluación completa, ≈ 31–35 ms:
//!
//! * **(A) Reintento con `undo`**: escribir en el idioma activo y, si queda el error de nombre,
//!   `undo()`, `set_language(otro)`, escribir de nuevo y `set_language(activo)`. Cuesta ≈ 3,0×
//!   (tres evaluaciones completas) y ≈ 2,0× con la evaluación pausada (dos): justo en el límite
//!   de la regla de `plan.md` §5.7 («≤ 2×»). Además **rompe Rehacer**: la entrada de historial
//!   guarda el texto tecleado y `redo` lo vuelve a analizar con el idioma *activo*, así que el
//!   texto del otro idioma da otra vez `#¿NOMBRE?`. Descartada y borrada del código; la prueba
//!   `el_reintento_con_undo_rompe_rehacer` conserva la evidencia.
//! * **(B) Traducción** (la que se usa, ≈ 1,0×): un modelo de borrador (vacío, mismo `locale` y
//!   zona, con los nombres definidos del libro) decide con el propio analizador del motor si el
//!   texto falla en el idioma activo y se entiende en el otro, y lo reescribe con los nombres
//!   del idioma activo (`get_cell_content` tras `set_language`). No hay tabla de nombres
//!   propia: los datos son los de IronCalc. El libro real recibe **una** escritura del texto
//!   ya traducido: una evaluación, una entrada de historial y un Rehacer que funciona (el
//!   texto de la entrada está en el idioma activo). Reconoce también `TRUE`/`FALSE` sueltos en
//!   español (dan `#ERROR!`, no error de nombre). Límites: una fórmula que mezcla nombres de
//!   los dos idiomas no se traduce a medias (queda el error de nombre con la parte válida
//!   intacta), y las referencias estructuradas a tablas no se reproducen en el borrador.
//!
//! **Defecto de IronCalc 0.9.1 (T006):** una fórmula que no se puede *analizar* (no el error de
//! nombre) se guarda con su texto tal cual en la tabla de fórmulas de la hoja, y otra fórmula
//! válida que se teclee después con el mismo texto interno (`IF(1>0,2,3)`) reutiliza esa
//! entrada fallida y se ve con `#ERROR!`. Los paréntesis, el `+` unario y los espacios no la
//! esquivan (el texto interno los normaliza). El núcleo no lo puede corregir (la tabla no es
//! accesible); lo que sí hace es no empeorarlo: el borrador de (B) se crea de nuevo en cada
//! fórmula (nunca acumula entradas fallidas) y el libro real solo recibe el texto que el
//! usuario tecleó o su traducción.
//!
//! **Rehacer y `SetLang`:** `redo` vuelve a analizar el texto de la entrada con el idioma activo
//! en ese momento (límite de IronCalc, mismo mecanismo que arriba). Si se cambia de idioma
//! entre Deshacer y Rehacer, las fórmulas rehechas pueden quedar con `#¿NOMBRE?`.

use ironcalc_base::UserModel;
use ironcalc_base::language::get_language;
use ironcalc_base::types::CellType;
use serde::{Deserialize, Serialize};

use crate::core::{Core, Lang};
use crate::error::CoreError;
use crate::nav::Dir;

/// Cómo se abre la edición de la celda activa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditMode {
    /// Se empezó a escribir sobre la celda: el texto inicial es solo lo tecleado (`seed`).
    Replace,
    /// F2 (o Ctrl+U en macOS) o doble clic: el texto inicial es el contenido actual más `seed`.
    Append,
    /// Clic en la barra de fórmulas: igual que `Append` para el texto; la interfaz decide dónde
    /// va el foco.
    FormulaBar,
}

/// Adónde va la celda activa tras confirmar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Advance {
    /// Una celda hacia abajo (Intro).
    Down,
    /// Una celda hacia arriba (Mayús+Intro).
    Up,
    /// Una celda a la derecha (Tab).
    Right,
    /// Una celda a la izquierda (Mayús+Tab).
    Left,
    /// No se mueve (confirmar con clic en otra parte, o desde la barra de fórmulas).
    Stay,
}

impl Advance {
    /// El avance de la tecla Intro: baja; con Mayús, sube.
    pub fn enter(shift: bool) -> Advance {
        if shift { Advance::Up } else { Advance::Down }
    }

    /// El avance de la tecla Tab: va a la derecha; con Mayús, a la izquierda.
    pub fn tab(shift: bool) -> Advance {
        if shift { Advance::Left } else { Advance::Right }
    }

    fn dir(self) -> Option<Dir> {
        match self {
            Advance::Down => Some(Dir::Down),
            Advance::Up => Some(Dir::Up),
            Advance::Right => Some(Dir::Right),
            Advance::Left => Some(Dir::Left),
            Advance::Stay => None,
        }
    }
}

/// Sesión de edición abierta (`UiState.edit`). Filas y columnas son base 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditState {
    /// Hoja de la celda editada (base 0).
    pub sheet: u32,
    /// Fila de la celda editada.
    pub row: i32,
    /// Columna de la celda editada.
    pub col: i32,
    /// Modo con el que se abrió.
    pub mode: EditMode,
    /// Contenido de la celda al abrir (en el idioma activo): lo que conserva Esc.
    pub original: String,
    /// Texto que se está escribiendo; el que muestran la celda y la barra de fórmulas.
    pub text: String,
}

/// Celda del modelo de borrador donde se prueba el texto: la última de la hoja, para que ninguna
/// referencia habitual (`A1:A3`, `1:1`, `A:A`) la incluya y la vuelva circular.
const BORRADOR_FILA: i32 = 1_048_576;
const BORRADOR_COL: i32 = 16_384;

/// Texto de un error del motor en un idioma (`#¿NOMBRE?`, `#NAME?`, `#ERROR!`…).
fn error_nombre(lang: Lang) -> &'static str {
    get_language(lang.id()).map_or("", |l| l.errors.name.as_str())
}

fn error_analisis(lang: Lang) -> &'static str {
    get_language(lang.id()).map_or("", |l| l.errors.error.as_str())
}

/// ¿La celda es un error y su texto es `texto(Es)` o `texto(En)`? No depende del idioma
/// activo del modelo en ese momento.
fn es_error_de(
    um: &UserModel<'_>,
    sheet: u32,
    row: i32,
    col: i32,
    texto: fn(Lang) -> &'static str,
) -> bool {
    if !matches!(um.get_cell_type(sheet, row, col), Ok(CellType::ErrorValue)) {
        return false;
    }
    um.get_formatted_cell_value(sheet, row, col)
        .is_ok_and(|v| v == texto(Lang::Es) || v == texto(Lang::En))
}

fn es_error_de_nombre(um: &UserModel<'_>, sheet: u32, row: i32, col: i32) -> bool {
    es_error_de(um, sheet, row, col, error_nombre)
}

fn es_error_de_analisis(um: &UserModel<'_>, sheet: u32, row: i32, col: i32) -> bool {
    es_error_de(um, sheet, row, col, error_analisis)
}

/// Nombres definidos del libro: `(nombre, ámbito, fórmula)`.
type NombresDefinidos = Vec<(String, Option<u32>, String)>;

/// Modelo de borrador nuevo (vacío, con el `locale` y la zona del documento) con `texto`
/// escrito en la última celda de la hoja, en el idioma `lang`. Con `nombres`, el borrador también
/// los define (sin importar a qué apunten: solo importa que existan, para que una fórmula que
/// los usa no falle por ellos).
fn modelo_con_texto<'a>(
    locale: &'a str,
    tz: &'a str,
    lang: Lang,
    texto: &str,
    nombres: &NombresDefinidos,
) -> Option<UserModel<'a>> {
    let mut m = UserModel::new_empty("borrador", locale, tz, lang.id()).ok()?;
    if !nombres.is_empty() {
        // Un nombre definido solo puede apuntar a un rango: todos apuntan a A1 de la única hoja
        // del borrador (cuyo nombre depende del idioma).
        let hoja = m.get_worksheets_properties().first()?.name.clone();
        let destino = format!("'{hoja}'!$A$1");
        m.pause_evaluation();
        for (nombre, _ambito, _formula) in nombres {
            // Un nombre que el borrador rechaza se omite.
            let _ = m.new_defined_name(nombre, None, &destino);
        }
        m.resume_evaluation();
    }
    m.set_user_input(0, BORRADOR_FILA, BORRADOR_COL, texto)
        .ok()?;
    Some(m)
}

/// ¿El borrador quedó con error de nombre o de análisis (la fórmula no se entiende)?
fn fallo_de_entendimiento(m: &UserModel<'_>) -> bool {
    es_error_de_nombre(m, 0, BORRADOR_FILA, BORRADOR_COL)
        || es_error_de_analisis(m, 0, BORRADOR_FILA, BORRADOR_COL)
}

/// Siguiente celda del rango `(f0, c0, f1, c1)` a partir de `(fila, col)` en la dirección
/// `dir`, dando la vuelta al llegar al final y saltando lo oculto (`visible`). Abajo/arriba
/// recorren cada columna y pasan a la siguiente; derecha/izquierda, cada fila. Devuelve la
/// misma celda si no hay otra visible.
fn siguiente_en_rango(
    rango: (i32, i32, i32, i32),
    activa: (i32, i32),
    dir: Dir,
    visible: &dyn Fn(i32, i32) -> bool,
) -> (i32, i32) {
    let (f0, c0, f1, c1) = rango;
    let (nf, nc) = (i64::from(f1 - f0 + 1), i64::from(c1 - c0 + 1));
    let total = nf * nc;
    let por_columnas = matches!(dir, Dir::Down | Dir::Up);
    let paso: i64 = if matches!(dir, Dir::Down | Dir::Right) {
        1
    } else {
        -1
    };
    let (df, dc) = (i64::from(activa.0 - f0), i64::from(activa.1 - c0));
    let mut idx = if por_columnas {
        dc * nf + df
    } else {
        df * nc + dc
    };
    // Una fila o columna oculta se salta; si casi todo estuviera oculto no se busca sin fin.
    for _ in 0..total.min(100_000) {
        idx = (idx + paso).rem_euclid(total);
        let (f, c) = if por_columnas {
            (idx % nf, idx / nf)
        } else {
            (idx / nc, idx % nc)
        };
        let celda = (f0 + f as i32, c0 + c as i32);
        if visible(celda.0, celda.1) {
            return celda;
        }
    }
    activa
}

impl Core {
    // ------------------------------------------------------------------ edición

    /// Abre una edición sobre la celda activa (`BeginEdit`). El texto inicial es `seed` en el
    /// modo [`EditMode::Replace`] y el contenido de la celda (en el idioma activo) más `seed`
    /// en los otros. Si ya hay una edición de esa misma celda solo cambia el modo y añade
    /// `seed` (F2 en mitad de la escritura); si es de otra, la sustituye. Lleva la vista a la
    /// celda activa.
    pub fn begin_edit(&mut self, mode: EditMode, seed: Option<String>) -> Result<(), CoreError> {
        self.sync_view_sheet();
        let (sheet, row, col) = self.um.get_selected_cell();
        let seed = seed.unwrap_or_default();
        if let Some(e) = &mut self.edit
            && (e.sheet, e.row, e.col) == (sheet, row, col)
        {
            e.mode = mode;
            e.text.push_str(&seed);
            return Ok(());
        }
        let original = self.cell_input(sheet, row, col)?;
        let text = match mode {
            EditMode::Replace => seed,
            EditMode::Append | EditMode::FormulaBar => format!("{original}{seed}"),
        };
        self.edit = Some(EditState {
            sheet,
            row,
            col,
            mode,
            original,
            text,
        });
        self.ensure_visible();
        Ok(())
    }

    /// Actualiza el texto de la edición abierta (`SetEditText`); sin edición no hace nada. No
    /// toca el libro.
    pub fn set_edit_text(&mut self, text: String) {
        if let Some(e) = &mut self.edit {
            e.text = text;
        }
    }

    /// Confirma la edición (`CommitEdit`): escribe `text` en la celda editada (o en la activa si
    /// no hay edición abierta) con la política de idioma, cierra la sesión y mueve la celda
    /// activa según `advance`. Si la escritura falla (por ejemplo, una celda combinada) la
    /// sesión sigue abierta para poder corregir el texto. El avance solo se aplica si la celda
    /// activa sigue siendo la editada.
    pub fn commit_edit(&mut self, text: String, advance: Advance) -> Result<(), CoreError> {
        // Instrumentación (T030): `commit_edit_ms`, la edición completa con su evaluación.
        crate::core::tramo!(self, crate::core::nombres::COMMIT_EDIT_MS);
        let (sheet, row, col) = match &self.edit {
            Some(e) => (e.sheet, e.row, e.col),
            None => {
                self.sync_view_sheet();
                self.um.get_selected_cell()
            }
        };
        self.write_input(sheet, row, col, &text)?;
        self.edit = None;
        self.advance_selection((sheet, row, col), advance)
    }

    /// Descarta la edición (`CancelEdit`, Esc): la celda conserva su valor anterior porque el
    /// libro no se tocó. No entra en el historial.
    pub fn cancel_edit(&mut self) {
        self.edit = None;
    }

    /// Edición abierta, si la hay.
    pub fn edit_state(&self) -> Option<&EditState> {
        self.edit.as_ref()
    }

    /// Contenido de la celda tal como se edita (`get_cell_content`): la fórmula, con los
    /// nombres en el idioma activo, o el valor. Vacío si no tiene dato.
    pub fn cell_input(&self, sheet: u32, row: i32, col: i32) -> Result<String, CoreError> {
        self.um
            .get_cell_content(sheet, row, col)
            .map_err(CoreError::Motor)
    }

    /// Texto de la barra de fórmulas: el de la edición abierta, si la hay (así la celda y la
    /// barra muestran lo mismo mientras se escribe), o el contenido de la celda activa.
    pub fn formula_bar(&self) -> String {
        if let Some(e) = &self.edit {
            return e.text.clone();
        }
        let (sheet, row, col) = self.um.get_selected_cell();
        self.um
            .get_cell_content(sheet, row, col)
            .unwrap_or_default()
    }

    /// Mueve la celda activa tras confirmar (ver el módulo).
    fn advance_selection(
        &mut self,
        editada: (u32, i32, i32),
        advance: Advance,
    ) -> Result<(), CoreError> {
        let Some(dir) = advance.dir() else {
            return Ok(());
        };
        self.sync_view_sheet();
        let sel = self.selection();
        if (sel.sheet, sel.active.0, sel.active.1) != editada {
            return Ok(());
        }
        if sel.is_single_cell() {
            return self.move_active(dir, false);
        }
        let g = self.geometry(sel.sheet)?;
        let (f0, c0, f1, c1) = sel.range;
        let (fila, col) = siguiente_en_rango(sel.range, sel.active, dir, &|f, c| {
            g.row_h(f) > 0.0 && g.col_w(c) > 0.0
        });
        // La edición ya se escribió: si el motor no deja mover la celda activa dentro del rango
        // (una celda combinada, por ejemplo), la selección se queda como estaba y no es un error.
        if self.um.set_selected_cell(fila, col).is_ok() {
            // El rango se conserva: la celda activa es una celda interior, no una esquina.
            if self.um.set_selected_range(f0, c0, f1, c1).is_err() {
                let _ =
                    self.um
                        .set_selected_range(sel.range.0, sel.range.1, sel.range.2, sel.range.3);
            }
            self.ensure_cell_visible(fila, col);
        }
        Ok(())
    }

    // ------------------------------------------------------------------ escritura e idioma

    /// Escribe `text` en una celda como si se tecleara, con la política de idioma de `plan.md`
    /// §5.7 (ver el módulo): una fórmula con los nombres del otro idioma se traduce antes de
    /// escribirla. Marca el documento como modificado y pone al día la geometría si hace falta.
    pub(crate) fn write_input(
        &mut self,
        sheet: u32,
        row: i32,
        col: i32,
        text: &str,
    ) -> Result<(), CoreError> {
        if sheet as usize >= self.um.get_model().workbook.worksheets.len() {
            return Err(CoreError::HojaInvalida(sheet));
        }
        let alto_antes = self.um.get_row_height(sheet, row).ok();
        // Es fórmula lo que el motor considera fórmula: empieza por `=` o por `+`/`-`.
        let traducida = if text.starts_with(['=', '+', '-']) {
            self.traducir_formula(text, self.lang())
        } else {
            None
        };
        self.um
            .set_user_input(sheet, row, col, traducida.as_deref().unwrap_or(text))
            .map_err(CoreError::Motor)?;
        self.mark_changed();
        self.note_cell_write(sheet, row, col, alto_antes);
        Ok(())
    }

    /// Traducción (B): si `text` falla en el idioma activo y se entiende en el otro, devuelve
    /// el mismo texto con los nombres del idioma activo; si no hace falta o no se puede,
    /// `None`. Usa modelos de borrador nuevos (el defecto de 0.9.1 describe por qué no se
    /// reutilizan): uno por idioma, con el `locale` del documento. Considera que falla tanto
    /// el error de nombre como el de análisis (`TRUE` y `FALSE` sueltos en español, por
    /// ejemplo, dan `#ERROR!`). Si el texto falla, se repite con los nombres definidos del
    /// libro, para que un nombre del libro no se confunda con una función del otro idioma.
    /// No traduce lo que sigue fallando en el otro idioma (una función que no existe en
    /// ninguno): traducir solo una parte estropearía las funciones que sí eran válidas.
    fn traducir_formula(&self, text: &str, lang: Lang) -> Option<String> {
        let (locale, tz) = (self.locale, self.tz);
        let sin_nombres = NombresDefinidos::new();
        // ¿Se acepta tal cual en el idioma activo? Entonces no hay nada que traducir.
        if !fallo_de_entendimiento(&modelo_con_texto(locale, tz, lang, text, &sin_nombres)?) {
            return None;
        }
        let nombres = self.um.get_defined_name_list();
        if !nombres.is_empty()
            && !fallo_de_entendimiento(&modelo_con_texto(locale, tz, lang, text, &nombres)?)
        {
            return None;
        }
        // ¿Se entiende en el otro idioma?
        let mut otro = modelo_con_texto(locale, tz, lang.other(), text, &nombres)?;
        if fallo_de_entendimiento(&otro) {
            return None;
        }
        let en_otro = otro.get_cell_content(0, BORRADOR_FILA, BORRADOR_COL).ok()?;
        otro.set_language(lang.id()).ok()?;
        let traducida = otro.get_cell_content(0, BORRADOR_FILA, BORRADOR_COL).ok()?;
        // Sin nombres que traducir (o sin cambios) se escribe el texto original.
        (traducida != en_otro).then_some(traducida)
    }

    /// Cambia el idioma de los nombres de función (`SetLang`). Las fórmulas guardadas se
    /// muestran en el nuevo idioma; no cambia el `locale` (separadores y decimal) ni marca el
    /// documento como modificado, pero sí sube `data_version` (los errores se rotulan en el
    /// idioma activo).
    pub fn set_lang(&mut self, lang: Lang) -> Result<(), CoreError> {
        self.um.set_language(lang.id()).map_err(CoreError::Motor)?;
        self.set_lang_state(lang);
        self.bump_version();
        Ok(())
    }

    // ------------------------------------------------------------------ historial

    /// ¿Hay algo que deshacer?
    pub fn can_undo(&self) -> bool {
        self.um.can_undo()
    }

    /// ¿Hay algo que rehacer?
    pub fn can_redo(&self) -> bool {
        self.um.can_redo()
    }

    /// Deshace la última entrada del historial (`Undo`) y evalúa el libro. Sin nada que deshacer,
    /// o con una edición abierta (el deshacer del texto es de la interfaz), no hace nada.
    pub fn undo(&mut self) -> Result<(), CoreError> {
        if self.edit.is_some() || !self.um.can_undo() {
            return Ok(());
        }
        let hojas = self.um.get_model().workbook.worksheets.len();
        self.um.undo().map_err(CoreError::Motor)?;
        self.after_history_step(hojas);
        Ok(())
    }

    /// Rehace la última entrada deshecha (`Redo`). Mismas reglas que [`Core::undo`].
    pub fn redo(&mut self) -> Result<(), CoreError> {
        if self.edit.is_some() || !self.um.can_redo() {
            return Ok(());
        }
        let hojas = self.um.get_model().workbook.worksheets.len();
        self.um.redo().map_err(CoreError::Motor)?;
        self.after_history_step(hojas);
        Ok(())
    }

    /// Tras deshacer o rehacer puede haber cambiado cualquier cosa: datos, rango usado, altura
    /// de filas, hojas. Se sube la versión, se marca el documento y se descarta la geometría (se
    /// reconstruye perezosamente al volver a pintar). Si cambió el número de hojas, se
    /// resincroniza además la vista.
    fn after_history_step(&mut self, hojas_antes: usize) {
        let hojas = self.um.get_model().workbook.worksheets.len();
        if hojas != hojas_antes {
            self.after_sheet_structure_change();
        } else {
            self.invalidate_geometry();
            self.mark_changed();
            self.sync_view_sheet();
        }
    }
}
