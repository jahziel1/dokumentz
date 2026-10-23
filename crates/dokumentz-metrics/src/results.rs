//! Tipos del esquema de resultados `dokumentz-bench/1` (`plan.md` §9.7).
//!
//! Los escriben `bench-runner` (método `external`), `compat-check` (`compat`) y las dos
//! interfaces con `--metrics-out` (`internal`, vía [`crate::Recorder`]); los lee `bench-report`.
//! El JSON Schema de `tools/bench-report/schema/results.v1.schema.json` se deriva de estos tipos.
//!
//! # Estabilidad del JSON
//!
//! - Las claves salen en el orden de declaración de cada campo y los mapas son `BTreeMap`
//!   (orden alfabético): la misma entrada produce siempre los mismos bytes.
//! - Los `Option` ausentes **no se escriben** (al leer, una clave ausente es `None`), salvo
//!   `criteria_sha`, que se escribe siempre (`null` en las corridas internas).
//! - Cambiar un nombre o un orden rompe el archivo dorado de `tests/golden/results_v1.json`.
//!
//! # Extensión respecto de §9.7
//!
//! El esquema del plan no tiene sitio para marcas, series ni contadores de la instrumentación
//! interna; se añade el bloque opcional [`Internal`] (clave `internal`). Los resultados
//! externos no lo llevan.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::histogram::Histogram;
pub use crate::histogram::HistogramSummary;

/// Identificador del esquema (campo `schema`).
pub const SCHEMA: &str = "dokumentz-bench/1";

/// Definición de percentil que declara el bloque [`Internal`].
pub const PERCENTILE_DEFINITION: &str = "nearest-rank";

/// Quién produjo el resultado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    /// Captura de pantalla y entrada sintética desde fuera (`bench-runner`).
    External,
    /// Instrumentación dentro de la app (`--metrics-out`); no comparable con ONLYOFFICE.
    Internal,
    /// Comprobación de compatibilidad con archivos reales (M-09).
    Compat,
}

/// Pantalla de la corrida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    /// Ancho en píxeles físicos.
    pub width_px: u32,
    /// Alto en píxeles físicos.
    pub height_px: u32,
    /// Factor de escala (2.0 en Retina).
    pub scale: f64,
    /// Frecuencia de refresco declarada.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_hz: Option<f64>,
}

/// Entorno de la corrida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Env {
    /// Sistema operativo (nombre y versión, o solo `std::env::consts::OS`).
    pub os: String,
    /// Arquitectura (`arm64`, `x86_64`…).
    pub arch: String,
    /// Procesador.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    /// Memoria RAM en GB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_gb: Option<f64>,
    /// GPU.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
    /// Pantalla.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplayInfo>,
    /// Motor web (solo Tauri).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_engine: Option<String>,
    /// `true` si la máquina es virtual.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm: Option<bool>,
    /// Alimentación (`ac`/`battery`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<String>,
    /// Sesión gráfica (`aqua`, `x11`, `wayland`, `win32`…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

impl Env {
    /// Entorno mínimo que conoce la biblioteca estándar (`os` y `arch`); el resto lo rellena
    /// quien llama.
    pub fn from_std() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu: None,
            ram_gb: None,
            gpu: None,
            display: None,
            web_engine: None,
            vm: None,
            power: None,
            session: None,
        }
    }
}

/// Aplicación medida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct App {
    /// `dokumentz-app-tauri`, `dokumentz-app-gpui`, `onlyoffice`…
    pub id: String,
    /// Versión.
    pub version: String,
    /// Commit de git.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// `release` o `debug`. Vacío: el [`crate::Recorder`] lo rellena con el perfil con el que se
    /// compiló esta biblioteca.
    pub build: String,
    /// Idioma (`es`/`en`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// SHA-256 del ejecutable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exe_sha256: Option<String>,
}

impl App {
    /// Aplicación con `id` y `version`; `build` vacío (se rellena al escribir).
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            commit: None,
            build: String::new(),
            lang: None,
            exe_sha256: None,
        }
    }
}

/// Archivo de datos usado (A1…A4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    /// `A1`…`A4`.
    pub id: String,
    /// SHA-256 del archivo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Filas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u64>,
    /// Columnas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<u64>,
}

/// Herramienta de captura (solo método externo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capture {
    /// `ffmpeg`…
    pub tool: String,
    /// Cuadros por segundo de la captura.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Cuanto de tiempo de la captura en ms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantum_ms: Option<f64>,
    /// Suelo de medición en ms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor_ms: Option<f64>,
}

/// Una repetición. Las medidas por cuadro/tecla (M-03, M-04) rellenan `samples`/`p50`…;
/// las de tiempo único (M-01, M-02, M-05, M-07) `value_ms`; M-06 `value_mb`/`peak_mb`;
/// M-08 `size_mb`/`installer_mb`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rep {
    /// Número de repetición (desde 1).
    pub n: u32,
    /// `false` si la repetición quedó invalidada.
    pub ok: bool,
    /// Número de muestras.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<u64>,
    /// Percentil 50 (nearest-rank).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50: Option<f64>,
    /// Percentil 95 (nearest-rank).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95: Option<f64>,
    /// Percentil 99 (nearest-rank).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p99: Option<f64>,
    /// Cuadros perdidos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropped: Option<u64>,
    /// Cuadros por segundo efectivos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_fps: Option<f64>,
    /// Tiempo único en ms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_ms: Option<f64>,
    /// Memoria en reposo en MB (mediana).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_mb: Option<f64>,
    /// Pico de memoria en MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_mb: Option<f64>,
    /// Tamaño del ejecutable en MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb: Option<f64>,
    /// Tamaño del instalador en MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installer_mb: Option<f64>,
    /// Notas libres.
    #[serde(default)]
    pub notes: String,
}

impl Rep {
    /// Repetición válida a partir de un histograma de muestras (p50/p95/p99 redondeados a
    /// 3 decimales); `None` si el histograma está vacío.
    pub fn from_histogram(n: u32, h: &Histogram) -> Option<Self> {
        let r = h.summary()?;
        Some(Self {
            n,
            ok: true,
            samples: Some(r.count),
            p50: Some(r.p50),
            p95: Some(r.p95),
            p99: Some(r.p99),
            dropped: None,
            effective_fps: None,
            value_ms: None,
            value_mb: None,
            peak_mb: None,
            size_mb: None,
            installer_mb: None,
            notes: String::new(),
        })
    }
}

/// Agregado de la corrida. Todos los campos son opcionales: cada medida rellena los suyos.
/// La agregación entre repeticiones (`plan.md` §9.7, spec §6) la hace `bench-report`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Mediana de los p95 de las repeticiones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_of_p95: Option<f64>,
    /// Peor p95 entre repeticiones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_p95: Option<f64>,
    /// Mediana.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median: Option<f64>,
    /// Percentil 95.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95: Option<f64>,
    /// Mínimo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Máximo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

impl Summary {
    /// Resumen de **una sola repetición** con las muestras de `h` (`median_of_p95` y
    /// `worst_p95` valen su p95); `Summary::default()` si `h` está vacío.
    pub fn from_histogram(h: &Histogram) -> Self {
        match h.summary() {
            None => Self::default(),
            Some(r) => Self {
                median_of_p95: Some(r.p95),
                worst_p95: Some(r.p95),
                median: Some(r.p50),
                p95: Some(r.p95),
                min: Some(r.min),
                max: Some(r.max),
            },
        }
    }
}

/// Una marca de tiempo (`launch`, `window_ready`, `open_request`, `first_data_frame`…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkEntry {
    /// Nombre de la marca.
    pub name: String,
    /// Milisegundos desde que se creó el `Recorder` (3 decimales).
    pub t_ms: f64,
}

/// Bloque propio de la instrumentación interna (extensión de §9.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Internal {
    /// Definición de percentil de los resúmenes (`nearest-rank`).
    pub percentile: String,
    /// Marcas en el orden en que se registraron.
    pub marks: Vec<MarkEntry>,
    /// Contadores, por nombre.
    pub counters: BTreeMap<String, u64>,
    /// Series de muestras (duraciones de tramos, `cells_drawn_per_frame`…), por nombre.
    pub histograms: BTreeMap<String, HistogramSummary>,
}

/// Documento completo `dokumentz-bench/1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Results {
    /// Siempre [`SCHEMA`].
    pub schema: String,
    /// Identificador de la corrida (nombre de carpeta).
    pub run_id: String,
    /// Hash del commit etiquetado `criterios-v1`; `null` en las corridas internas.
    pub criteria_sha: Option<String>,
    /// Quién lo produjo.
    pub method: Method,
    /// Entorno.
    pub env: Env,
    /// Aplicación.
    pub app: App,
    /// Archivo de datos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<FileInfo>,
    /// Medida (`M-01`…`M-09`).
    pub measure: String,
    /// Escenario (`S1-wheel-10s`…).
    pub scenario: String,
    /// `true` si fue arranque en frío.
    pub cold: bool,
    /// Captura (solo método externo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<Capture>,
    /// Repeticiones.
    pub reps: Vec<Rep>,
    /// Agregado.
    pub summary: Summary,
    /// Motivos de invalidación (vacío si la corrida es válida).
    pub invalidated: Vec<String>,
    /// Instrumentación interna (solo método `internal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<Internal>,
}

impl Results {
    /// JSON estable: sangría de 2 espacios y salto de línea final.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string_pretty(self)?;
        s.push('\n');
        Ok(s)
    }

    /// Lee un documento; rechaza un `schema` distinto de [`SCHEMA`].
    pub fn from_json_str(texto: &str) -> Result<Self, serde_json::Error> {
        let r: Self = serde_json::from_str(texto)?;
        if r.schema != SCHEMA {
            return Err(<serde_json::Error as serde::de::Error>::custom(format!(
                "esquema `{}` no soportado (se esperaba `{SCHEMA}`)",
                r.schema
            )));
        }
        Ok(r)
    }
}

/// Datos de la corrida que el [`crate::Recorder`] copia al documento. Lo que mide (marcas,
/// series, contadores) lo pone él.
#[derive(Debug, Clone, PartialEq)]
pub struct RunInfo {
    /// Identificador; vacío: se usa el nombre del archivo de salida sin extensión.
    pub run_id: String,
    /// Entorno.
    pub env: Env,
    /// Aplicación.
    pub app: App,
    /// Archivo de datos.
    pub file: Option<FileInfo>,
    /// Medida (`M-02`…); `internal` si no se indica.
    pub measure: String,
    /// Escenario.
    pub scenario: String,
    /// `true` si fue arranque en frío.
    pub cold: bool,
    /// Serie que llena `reps[0]` y `summary` (p. ej. `frame_interval_ms`); `None`: sin
    /// repeticiones.
    pub primary: Option<String>,
}

impl Default for RunInfo {
    fn default() -> Self {
        Self {
            run_id: String::new(),
            env: Env::from_std(),
            app: App::new("unknown", env!("CARGO_PKG_VERSION")),
            file: None,
            measure: "internal".to_string(),
            scenario: String::new(),
            cold: false,
            primary: None,
        }
    }
}

/// Perfil con el que se compiló esta biblioteca: `debug` o `release`.
pub(crate) fn perfil_de_compilacion() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
