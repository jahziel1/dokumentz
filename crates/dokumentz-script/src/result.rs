//! `ScriptResult`: lo que cada interfaz escribe al ejecutar un guion (`--script-out`).
//!
//! Es **determinista**: no lleva tiempos ni marcas de reloj (eso va al JSON de métricas), de
//! modo que el mismo guion produce el mismo archivo en las dos interfaces salvo el campo `app`
//! (ver [`ScriptResult::same_outcome`]).
//!
//! # Formato
//!
//! ```json
//! { "schema": "dokumentz-script-result/1", "script": "nav", "app": "dokumentz-app-gpui",
//!   "ok": true,
//!   "steps": [ { "index": 0, "step": "open", "ok": true },
//!              { "index": 3, "step": "assert_cell", "ok": true,
//!                "observed": { "type": "cell", "cell": "A1", "value": "Nombre" } } ] }
//! ```

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::script::{ScriptError, StepKind};

/// Identificador del formato de resultado.
pub const RESULT_SCHEMA: &str = "dokumentz-script-result/1";

fn schema_por_defecto() -> String {
    RESULT_SCHEMA.to_string()
}

/// Lo que la interfaz observó al ejecutar un `assert_*`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observed {
    /// Resultado de `assert_cell`.
    Cell {
        /// Celda leída (A1).
        cell: String,
        /// Hoja leída.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sheet: Option<String>,
        /// Texto mostrado.
        value: String,
        /// Contenido editable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formula: Option<String>,
    },
    /// Resultado de `assert_selection`.
    Selection {
        /// Rango seleccionado (A1).
        range: String,
        /// Celda activa.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<String>,
    },
    /// Resultado de `assert_formula_bar`.
    FormulaBar {
        /// Texto de la barra.
        text: String,
    },
}

/// Resultado de un paso.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
    /// Posición del paso en el guion (desde 0).
    pub index: usize,
    /// Clase del paso.
    pub step: StepKind,
    /// `false` si el paso falló al ejecutarse (archivo inexistente, tiempo agotado…). Un
    /// `assert_*` que observa un valor distinto del esperado **no** falla aquí: lo decide
    /// [`crate::check()`].
    pub ok: bool,
    /// Valor observado (solo en los `assert_*`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<Observed>,
    /// Motivo del fallo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StepResult {
    /// Paso ejecutado sin novedad.
    pub fn done(index: usize, step: StepKind) -> Self {
        Self {
            index,
            step,
            ok: true,
            observed: None,
            error: None,
        }
    }

    /// Comprobación con su valor observado.
    pub fn observed(index: usize, step: StepKind, observed: Observed) -> Self {
        Self {
            index,
            step,
            ok: true,
            observed: Some(observed),
            error: None,
        }
    }

    /// Paso que falló al ejecutarse.
    pub fn failed(index: usize, step: StepKind, error: impl Into<String>) -> Self {
        Self {
            index,
            step,
            ok: false,
            observed: None,
            error: Some(error.into()),
        }
    }
}

/// Resultado de ejecutar un guion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptResult {
    /// Siempre [`RESULT_SCHEMA`].
    #[serde(default = "schema_por_defecto")]
    pub schema: String,
    /// Nombre del guion ejecutado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Aplicación que lo ejecutó (`dokumentz-app-tauri`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// `true` si todos los pasos se ejecutaron (sin contar las diferencias de los `assert_*`).
    pub ok: bool,
    /// Error que abortó el guion, si lo hubo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Un resultado por paso ejecutado, en orden.
    pub steps: Vec<StepResult>,
}

impl ScriptResult {
    /// Resultado vacío y válido; se le van añadiendo pasos con [`ScriptResult::push`].
    pub fn new(script: Option<String>, app: Option<String>) -> Self {
        Self {
            schema: schema_por_defecto(),
            script,
            app,
            ok: true,
            error: None,
            steps: Vec::new(),
        }
    }

    /// Añade un paso; si falló, el resultado completo pasa a `ok: false`.
    pub fn push(&mut self, paso: StepResult) {
        if !paso.ok {
            self.ok = false;
        }
        self.steps.push(paso);
    }

    /// Aborta el guion con un error global.
    pub fn abort(&mut self, error: impl Into<String>) {
        self.ok = false;
        self.error = Some(error.into());
    }

    /// `true` si `otro` observó lo mismo que `self` (todo igual salvo el campo `app`): lo que
    /// T062 exige entre las dos interfaces.
    pub fn same_outcome(&self, otro: &ScriptResult) -> bool {
        let mut a = self.clone();
        let mut b = otro.clone();
        a.app = None;
        b.app = None;
        a == b
    }

    /// Lee un resultado desde texto JSON.
    pub fn from_json_str(texto: &str) -> Result<Self, ScriptError> {
        let r: Self = serde_json::from_str(texto).map_err(ScriptError::Json)?;
        if r.schema != RESULT_SCHEMA {
            return Err(ScriptError::Schema {
                found: r.schema,
                expected: RESULT_SCHEMA,
            });
        }
        Ok(r)
    }

    /// Lee un resultado desde un archivo.
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

    /// Escribe el JSON en `ruta` (crea las carpetas que falten).
    pub fn write_to(&self, ruta: impl AsRef<Path>) -> Result<(), ScriptError> {
        let ruta = ruta.as_ref();
        let io = |source| ScriptError::Io {
            path: ruta.to_path_buf(),
            source,
        };
        if let Some(carpeta) = ruta.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(carpeta).map_err(io)?;
        }
        fs::write(ruta, self.to_json_string()?).map_err(io)
    }
}
