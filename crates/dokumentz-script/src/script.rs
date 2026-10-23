//! `Script` y `Step`: el guion JSON que ejecutan las interfaces (`plan.md` §5.10).
//!
//! Cada interfaz ejecuta los pasos llamando a **los mismos manejadores** que la entrada real
//! (no al núcleo directamente). Un guion sirve igual para CI, pruebas funcionales y
//! mediciones internas.
//!
//! # Formato
//!
//! ```json
//! { "schema": "dokumentz-script/1", "name": "nav",
//!   "steps": [ { "step": "open", "path": "testdata/ci/A1.xlsx" },
//!              { "step": "key", "key": "ArrowDown", "mods": ["shift"], "repeat": 3 },
//!              { "step": "assert_selection", "range": "A1:A4", "active": "A1" },
//!              { "step": "quit" } ] }
//! ```
//!
//! - La clave `step` elige el paso; las claves desconocidas se rechazan.
//! - Teclas (`key`): valores de `KeyboardEvent.key` de W3C UI Events (`ArrowDown`, `Enter`,
//!   `Tab`, `Escape`, `F2`, `PageDown`, `Home`, `Backspace`, `a`…); `text` escribe cadenas
//!   completas como si se teclearan.
//! - [`Modifier::Command`] es la tecla de atajos de la plataforma (Cmd en macOS, Ctrl en el
//!   resto), para que un guion valga en los tres sistemas.
//! - Posiciones ([`Pos`]): una celda (`{"cell":"B2"}`, su centro) o píxeles lógicos relativos
//!   a la esquina superior izquierda del área de la hoja, cabeceras incluidas
//!   (`{"x":120,"y":80}`). **Un clic es un `drag` con `from == to`.**
//! - Las comprobaciones (`assert_*`) no abortan el guion: cada interfaz anota lo observado en
//!   el [`crate::ScriptResult`] y [`crate::check()`] lo compara después con lo esperado.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cell::{normalize_cell, normalize_range};

/// Identificador del formato de guion.
pub const SCHEMA: &str = "dokumentz-script/1";

/// Tope de una espera, en ms (10 minutos): un guion que pide más está mal escrito.
pub const MAX_WAIT_MS: u64 = 600_000;

/// Tope de repeticiones de una tecla o de rondas de desplazamiento.
pub const MAX_REPEAT: u32 = 100_000;

fn schema_por_defecto() -> String {
    SCHEMA.to_string()
}
fn uno() -> u32 {
    1
}
fn es_uno(n: &u32) -> bool {
    *n == 1
}
fn cero(n: &u64) -> bool {
    *n == 0
}
fn espera_por_defecto() -> u64 {
    10_000
}

/// Modificador de teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    /// Mayúsculas.
    Shift,
    /// Control.
    Ctrl,
    /// Alt/Opción.
    Alt,
    /// Meta (Cmd en macOS, Windows en el resto).
    Meta,
    /// Tecla de atajos de la plataforma: Cmd en macOS, Ctrl en Windows y Linux.
    Command,
}

/// Posición del puntero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Pos {
    /// Centro de una celda, en la hoja activa (`"B2"`).
    Cell {
        /// Dirección A1.
        cell: String,
    },
    /// Píxeles lógicos desde la esquina superior izquierda del área de la hoja.
    Point {
        /// Distancia horizontal.
        x: f64,
        /// Distancia vertical.
        y: f64,
    },
}

/// Un paso del guion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    /// Abre un archivo (como `--open`).
    Open {
        /// Ruta del archivo.
        path: String,
    },
    /// Pulsa una tecla (y la suelta) `repeat` veces.
    Key {
        /// Valor `KeyboardEvent.key`.
        key: String,
        /// Modificadores pulsados.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        mods: Vec<Modifier>,
        /// Veces que se pulsa (por defecto 1).
        #[serde(default = "uno", skip_serializing_if = "es_uno")]
        repeat: u32,
    },
    /// Escribe texto como si se tecleara.
    Text {
        /// Texto a escribir.
        text: String,
    },
    /// Gira la rueda: `count` rondas de `(dx, dy)` píxeles lógicos separadas `every_ms`.
    Scroll {
        /// Desplazamiento horizontal por ronda (positivo: hacia la derecha).
        #[serde(default)]
        dx: f64,
        /// Desplazamiento vertical por ronda (positivo: hacia abajo).
        #[serde(default)]
        dy: f64,
        /// Número de rondas (por defecto 1).
        #[serde(default = "uno", skip_serializing_if = "es_uno")]
        count: u32,
        /// Milisegundos entre rondas (por defecto 0).
        #[serde(default, skip_serializing_if = "cero")]
        every_ms: u64,
    },
    /// Pulsa en `from`, arrastra hasta `to` durante `duration_ms` y suelta. Con `from == to`
    /// es un clic.
    Drag {
        /// Punto de partida.
        from: Pos,
        /// Punto de llegada.
        to: Pos,
        /// Duración del arrastre en ms (por defecto 0).
        #[serde(default, skip_serializing_if = "cero")]
        duration_ms: u64,
    },
    /// Espera un tiempo fijo.
    WaitMs {
        /// Milisegundos.
        ms: u64,
    },
    /// Espera a que la interfaz esté quieta (sin trabajo en curso ni cuadros pendientes).
    WaitIdle {
        /// Tiempo máximo en ms (por defecto 10 000); si se agota, el paso falla.
        #[serde(default = "espera_por_defecto")]
        timeout_ms: u64,
    },
    /// Registra una marca en el `Recorder` de métricas.
    Mark {
        /// Nombre de la marca.
        name: String,
    },
    /// Anota el contenido de una celda. Debe llevar `value`, `formula` o ambos.
    AssertCell {
        /// Dirección A1.
        cell: String,
        /// Hoja por nombre (por defecto, la activa).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sheet: Option<String>,
        /// Texto mostrado (ya formateado).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Contenido editable (fórmula o valor tal como se escribió).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formula: Option<String>,
    },
    /// Anota la selección.
    AssertSelection {
        /// Rango A1 (`"B2"` o `"B2:D5"`); las esquinas se comparan ordenadas.
        range: String,
        /// Celda activa.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<String>,
    },
    /// Anota el texto de la barra de fórmulas.
    AssertFormulaBar {
        /// Texto exacto.
        text: String,
    },
    /// Guarda. Sin `path`, sobre el archivo abierto; con `path`, «Guardar como».
    Save {
        /// Ruta de destino.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
    /// Termina la aplicación. Solo puede ser el último paso. (Es una variante con llaves vacías,
    /// `Step::Quit {}`, para que `deny_unknown_fields` rechace claves de más.)
    Quit {},
}

impl Step {
    /// Clase del paso.
    pub fn kind(&self) -> StepKind {
        match self {
            Step::Open { .. } => StepKind::Open,
            Step::Key { .. } => StepKind::Key,
            Step::Text { .. } => StepKind::Text,
            Step::Scroll { .. } => StepKind::Scroll,
            Step::Drag { .. } => StepKind::Drag,
            Step::WaitMs { .. } => StepKind::WaitMs,
            Step::WaitIdle { .. } => StepKind::WaitIdle,
            Step::Mark { .. } => StepKind::Mark,
            Step::AssertCell { .. } => StepKind::AssertCell,
            Step::AssertSelection { .. } => StepKind::AssertSelection,
            Step::AssertFormulaBar { .. } => StepKind::AssertFormulaBar,
            Step::Save { .. } => StepKind::Save,
            Step::Quit {} => StepKind::Quit,
        }
    }

    fn validar(&self) -> Result<(), String> {
        fn celda(c: &str) -> Result<(), String> {
            normalize_cell(c)
                .map(|_| ())
                .ok_or_else(|| format!("`{c}` no es una celda válida"))
        }
        fn pos(p: &Pos) -> Result<(), String> {
            match p {
                Pos::Cell { cell } => celda(cell),
                Pos::Point { x, y } if x.is_finite() && y.is_finite() => Ok(()),
                Pos::Point { .. } => Err("las coordenadas deben ser finitas".to_string()),
            }
        }
        fn rondas(n: u32, que: &str) -> Result<(), String> {
            if (1..=MAX_REPEAT).contains(&n) {
                Ok(())
            } else {
                Err(format!("`{que}` debe estar entre 1 y {MAX_REPEAT}"))
            }
        }
        fn espera(ms: u64, que: &str) -> Result<(), String> {
            if ms <= MAX_WAIT_MS {
                Ok(())
            } else {
                Err(format!("`{que}` no puede pasar de {MAX_WAIT_MS} ms"))
            }
        }
        match self {
            Step::Open { path } if path.is_empty() => Err("`path` está vacío".to_string()),
            Step::Key { key, repeat, .. } => {
                if key.is_empty() {
                    return Err("`key` está vacío".to_string());
                }
                rondas(*repeat, "repeat")
            }
            Step::Text { text } if text.is_empty() => Err("`text` está vacío".to_string()),
            Step::Scroll {
                dx,
                dy,
                count,
                every_ms,
            } => {
                if !dx.is_finite() || !dy.is_finite() {
                    return Err("`dx` y `dy` deben ser finitos".to_string());
                }
                rondas(*count, "count")?;
                espera(*every_ms, "every_ms")
            }
            Step::Drag {
                from,
                to,
                duration_ms,
            } => {
                pos(from)?;
                pos(to)?;
                espera(*duration_ms, "duration_ms")
            }
            Step::WaitMs { ms } => espera(*ms, "ms"),
            Step::WaitIdle { timeout_ms } => espera(*timeout_ms, "timeout_ms"),
            Step::Mark { name } if name.is_empty() => Err("`name` está vacío".to_string()),
            Step::AssertCell {
                cell,
                value,
                formula,
                ..
            } => {
                celda(cell)?;
                if value.is_none() && formula.is_none() {
                    return Err("`assert_cell` necesita `value`, `formula` o ambos".to_string());
                }
                Ok(())
            }
            Step::AssertSelection { range, active } => {
                normalize_range(range).ok_or_else(|| format!("`{range}` no es un rango válido"))?;
                active.as_deref().map_or(Ok(()), celda)
            }
            Step::Save { path: Some(p) } if p.is_empty() => Err("`path` está vacío".to_string()),
            _ => Ok(()),
        }
    }
}

/// Clase de paso, sin datos (la usan los resultados).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// `open`
    Open,
    /// `key`
    Key,
    /// `text`
    Text,
    /// `scroll`
    Scroll,
    /// `drag`
    Drag,
    /// `wait_ms`
    WaitMs,
    /// `wait_idle`
    WaitIdle,
    /// `mark`
    Mark,
    /// `assert_cell`
    AssertCell,
    /// `assert_selection`
    AssertSelection,
    /// `assert_formula_bar`
    AssertFormulaBar,
    /// `save`
    Save,
    /// `quit`
    Quit,
}

impl StepKind {
    /// `true` para los tres `assert_*`.
    pub fn is_assert(self) -> bool {
        matches!(
            self,
            Self::AssertCell | Self::AssertSelection | Self::AssertFormulaBar
        )
    }
}

impl fmt::Display for StepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nombre = match self {
            Self::Open => "open",
            Self::Key => "key",
            Self::Text => "text",
            Self::Scroll => "scroll",
            Self::Drag => "drag",
            Self::WaitMs => "wait_ms",
            Self::WaitIdle => "wait_idle",
            Self::Mark => "mark",
            Self::AssertCell => "assert_cell",
            Self::AssertSelection => "assert_selection",
            Self::AssertFormulaBar => "assert_formula_bar",
            Self::Save => "save",
            Self::Quit => "quit",
        };
        f.write_str(nombre)
    }
}

/// Error al leer, validar o escribir guiones y resultados.
#[derive(Debug)]
pub enum ScriptError {
    /// No se pudo leer o escribir el archivo.
    Io {
        /// Ruta implicada.
        path: PathBuf,
        /// Error del sistema.
        source: io::Error,
    },
    /// El texto no es un JSON válido para el tipo esperado.
    Json(serde_json::Error),
    /// El campo `schema` no es el soportado.
    Schema {
        /// Valor encontrado.
        found: String,
        /// Valor esperado.
        expected: &'static str,
    },
    /// Un paso no es válido (`step` cuenta desde 0).
    Invalid {
        /// Índice del paso.
        step: usize,
        /// Qué está mal.
        message: String,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Json(e) => write!(f, "JSON no válido: {e}"),
            Self::Schema { found, expected } => {
                write!(
                    f,
                    "esquema `{found}` no soportado (se esperaba `{expected}`)"
                )
            }
            Self::Invalid { step, message } => write!(f, "paso {step}: {message}"),
        }
    }
}

impl Error for ScriptError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

/// Un guion: lista ordenada de pasos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// Siempre [`SCHEMA`].
    #[serde(default = "schema_por_defecto")]
    pub schema: String,
    /// Nombre legible (p. ej. `nav`); se copia al resultado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Pasos, en orden.
    pub steps: Vec<Step>,
}

impl Script {
    /// Guion con `steps` y sin nombre.
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            schema: schema_por_defecto(),
            name: None,
            steps,
        }
    }

    /// Lee y valida un guion desde texto JSON.
    pub fn from_json_str(texto: &str) -> Result<Self, ScriptError> {
        let s: Self = serde_json::from_str(texto).map_err(ScriptError::Json)?;
        s.validate()?;
        Ok(s)
    }

    /// Lee y valida un guion desde un archivo.
    pub fn from_path(ruta: impl AsRef<Path>) -> Result<Self, ScriptError> {
        let ruta = ruta.as_ref();
        let texto = fs::read_to_string(ruta).map_err(|source| ScriptError::Io {
            path: ruta.to_path_buf(),
            source,
        })?;
        Self::from_json_str(&texto)
    }

    /// JSON con sangría y salto de línea final.
    pub fn to_json_string(&self) -> Result<String, ScriptError> {
        let mut s = serde_json::to_string_pretty(self).map_err(ScriptError::Json)?;
        s.push('\n');
        Ok(s)
    }

    /// Comprueba el esquema y cada paso (direcciones A1, topes, `assert_cell` con algo que
    /// comparar, `quit` solo al final).
    pub fn validate(&self) -> Result<(), ScriptError> {
        if self.schema != SCHEMA {
            return Err(ScriptError::Schema {
                found: self.schema.clone(),
                expected: SCHEMA,
            });
        }
        let ultimo = self.steps.len().saturating_sub(1);
        for (i, paso) in self.steps.iter().enumerate() {
            paso.validar()
                .map_err(|message| ScriptError::Invalid { step: i, message })?;
            if matches!(paso, Step::Quit {}) && i != ultimo {
                return Err(ScriptError::Invalid {
                    step: i,
                    message: "`quit` solo puede ser el último paso".to_string(),
                });
            }
        }
        Ok(())
    }
}
