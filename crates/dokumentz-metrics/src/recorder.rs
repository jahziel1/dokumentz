//! `Recorder`, `Span` y lectura de la bandera `--metrics-out` (FR-012, `plan.md` §5.10).

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use crate::histogram::{Histogram, redondear};
use crate::results::{
    Internal, MarkEntry, Method, PERCENTILE_DEFINITION, Rep, Results, RunInfo, SCHEMA, Summary,
    perfil_de_compilacion,
};

/// Bandera de línea de comandos que enciende la instrumentación.
pub const FLAG: &str = "--metrics-out";

/// Variable de entorno equivalente a [`FLAG`] (la bandera manda si están las dos).
pub const ENV_VAR: &str = "DOKUMENTZ_METRICS_OUT";

/// Nombres comunes de marcas, series y contadores (`plan.md` §5.10), para que las dos
/// interfaces y el núcleo escriban lo mismo. Las series de duración terminan en `_ms`.
pub mod names {
    /// Marca: arranque del proceso.
    pub const LAUNCH: &str = "launch";
    /// Marca: la ventana está lista.
    pub const WINDOW_READY: &str = "window_ready";
    /// Marca: se pidió abrir un archivo.
    pub const OPEN_REQUEST: &str = "open_request";
    /// Marca: primer cuadro con datos de la hoja.
    pub const FIRST_DATA_FRAME: &str = "first_data_frame";

    /// Contador: celdas en la lista de dibujo.
    pub const CELLS_IN_LIST: &str = "cells_in_list";
    /// Serie: celdas dibujadas en cada cuadro (invariante de FR-003).
    pub const CELLS_DRAWN_PER_FRAME: &str = "cells_drawn_per_frame";
    /// Contador: cuadros pintados con el marco vacío mientras llega el bloque.
    pub const PLACEHOLDER_FRAMES: &str = "placeholder_frames";

    /// Serie: carga del archivo.
    pub const LOAD_MS: &str = "load_ms";
    /// Serie: `draw_block` en el núcleo.
    pub const DRAW_BLOCK_MS: &str = "draw_block_ms";
    /// Serie: codificación de la lista de dibujo.
    pub const ENCODE_MS: &str = "encode_ms";
    /// Serie: ida y vuelta del puente (Tauri).
    pub const IPC_MS: &str = "ipc_ms";
    /// Serie: pintado de un cuadro.
    pub const PAINT_MS: &str = "paint_ms";
    /// Serie: intervalo entre cuadros consecutivos.
    pub const FRAME_INTERVAL_MS: &str = "frame_interval_ms";
    /// Serie: latencia de tecla (entrada → siguiente cuadro).
    pub const KEY_LATENCY_MS: &str = "key_latency_ms";
}

/// Error al escribir el resultado.
#[derive(Debug)]
pub enum MetricsError {
    /// No se pudo crear la carpeta o el archivo de salida.
    Io {
        /// Ruta implicada.
        path: PathBuf,
        /// Error del sistema.
        source: io::Error,
    },
    /// No se pudo serializar el documento.
    Json(serde_json::Error),
}

impl fmt::Display for MetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "no se pudo escribir `{}`: {source}", path.display())
            }
            Self::Json(e) => write!(f, "no se pudo serializar el resultado: {e}"),
        }
    }
}

impl Error for MetricsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json(e) => Some(e),
        }
    }
}

/// Error al leer `--metrics-out`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsError(String);

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ArgsError {}

#[derive(Debug, Default)]
struct Estado {
    run: RunInfo,
    marcas: Vec<MarkEntry>,
    contadores: BTreeMap<String, u64>,
    series: BTreeMap<String, Histogram>,
}

#[derive(Debug)]
struct Activo {
    ruta: PathBuf,
    inicio: Instant,
    estado: Mutex<Estado>,
}

/// Registro de mediciones internas. **Apagado por defecto**: un `Recorder` apagado no mide
/// (no lee el reloj), no reserva memoria, no abre archivos y cada método vuelve de inmediato.
/// Solo se enciende con [`FLAG`] o [`ENV_VAR`] (o con [`Recorder::to_path`] en pruebas).
///
/// Es `Send + Sync`: se comparte con `Arc<Recorder>` entre hilos (`&self` en todos los
/// métodos). Nada de lo que hace toca la red (FR-012).
///
/// El resultado se escribe **solo** al llamar a [`Recorder::write_json`] (al salir o al
/// terminar el guion); soltar el `Recorder` no escribe nada.
///
/// # Definición de percentil
///
/// Las series se resumen con el método del rango más cercano («nearest-rank», sin
/// interpolación); ver [`crate::histogram`]. El JSON lo declara en `internal.percentile`.
#[derive(Debug)]
pub struct Recorder {
    activo: Option<Activo>,
}

impl Recorder {
    /// Recorder apagado.
    pub fn off() -> Self {
        Self { activo: None }
    }

    /// Recorder encendido que escribirá en `ruta`. El tiempo de las marcas cuenta desde aquí.
    pub fn to_path(ruta: impl Into<PathBuf>) -> Self {
        Self {
            activo: Some(Activo {
                ruta: ruta.into(),
                inicio: Instant::now(),
                estado: Mutex::new(Estado::default()),
            }),
        }
    }

    /// Lee `--metrics-out <ruta>` de los argumentos del proceso y, si no está,
    /// `DOKUMENTZ_METRICS_OUT`. Una bandera mal escrita (sin ruta) deja el Recorder apagado;
    /// use [`Recorder::parse`] si quiere detectarla.
    pub fn from_args_env() -> Self {
        Self::parse(std::env::args_os().skip(1), std::env::var_os(ENV_VAR))
            .unwrap_or_else(|_| Self::off())
    }

    /// Versión pura de [`Recorder::from_args_env`]: `args` sin el nombre del programa y el
    /// valor de la variable de entorno.
    ///
    /// - Admite `--metrics-out ruta` y `--metrics-out=ruta` (la forma con `=` exige UTF-8);
    ///   gana la última aparición y la bandera manda sobre la variable de entorno.
    /// - Se detiene en `--`.
    /// - Un valor vacío, ausente o que empiece por `--` es un error ([`ArgsError`]).
    /// - Una variable de entorno vacía cuenta como no definida.
    pub fn parse<I>(args: I, env: Option<OsString>) -> Result<Self, ArgsError>
    where
        I: IntoIterator,
        I::Item: Into<OsString>,
    {
        let mut ruta: Option<OsString> = None;
        let mut it = args.into_iter().map(Into::into);
        while let Some(arg) = it.next() {
            let Some(texto) = arg.to_str() else { continue };
            if texto == "--" {
                break;
            }
            if texto == FLAG {
                let valor = it
                    .next()
                    .filter(|v| !v.is_empty() && !empieza_por_guiones(v));
                ruta = Some(valor.ok_or_else(|| ArgsError(format!("`{FLAG}` necesita una ruta")))?);
            } else if let Some(valor) = texto.strip_prefix("--metrics-out=") {
                if valor.is_empty() {
                    return Err(ArgsError(format!("`{FLAG}=` necesita una ruta")));
                }
                ruta = Some(OsString::from(valor));
            }
        }
        let ruta = ruta.or_else(|| env.filter(|v| !v.is_empty()));
        Ok(ruta.map_or_else(Self::off, Self::to_path))
    }

    /// `true` si está encendido.
    pub fn is_on(&self) -> bool {
        self.activo.is_some()
    }

    /// Ruta de salida (`None` si está apagado).
    pub fn out_path(&self) -> Option<&Path> {
        self.activo.as_ref().map(|a| a.ruta.as_path())
    }

    /// Fija los datos de la corrida (entorno, app, archivo, medida…). Apagado: no hace nada.
    pub fn set_run(&self, info: RunInfo) {
        if let Some(a) = &self.activo {
            a.bloquear().run = info;
        }
    }

    /// Marca de tiempo ahora (ms desde que se creó el Recorder). Las repetidas se conservan
    /// en orden.
    pub fn mark(&self, nombre: &str) {
        if let Some(a) = &self.activo {
            let t = a.inicio.elapsed().as_secs_f64() * 1000.0;
            a.bloquear().marcas.push(MarkEntry {
                name: nombre.to_string(),
                t_ms: redondear(t),
            });
        }
    }

    /// Marca con un instante ya medido por quien llama (p. ej. `performance.now()` de la
    /// webview, convertido a ms desde el arranque). Ignora valores no finitos.
    pub fn mark_at(&self, nombre: &str, t_ms: f64) {
        if let Some(a) = &self.activo
            && t_ms.is_finite()
        {
            a.bloquear().marcas.push(MarkEntry {
                name: nombre.to_string(),
                t_ms: redondear(t_ms),
            });
        }
    }

    /// Abre un tramo cronometrado; al soltar la guardia (o llamar a [`Span::finish`]) añade su
    /// duración en ms a la serie `nombre`. Apagado: la guardia no lee el reloj ni registra nada.
    pub fn span<'a>(&'a self, nombre: &'a str) -> Span<'a> {
        let destino = self.activo.as_ref().map(|_| (self, Instant::now()));
        Span { destino, nombre }
    }

    /// Añade una muestra a la serie `nombre` (duraciones en ms medidas fuera, valores por
    /// cuadro como `cells_drawn_per_frame`…). Ignora valores no finitos.
    pub fn sample(&self, nombre: &str, valor: f64) {
        if let Some(a) = &self.activo {
            let mut e = a.bloquear();
            if let Some(h) = e.series.get_mut(nombre) {
                h.record(valor);
            } else {
                let mut h = Histogram::new();
                if h.record(valor) {
                    e.series.insert(nombre.to_string(), h);
                }
            }
        }
    }

    /// Suma `delta` al contador `nombre` (saturante).
    pub fn count(&self, nombre: &str, delta: u64) {
        if let Some(a) = &self.activo {
            let mut e = a.bloquear();
            if let Some(c) = e.contadores.get_mut(nombre) {
                *c = c.saturating_add(delta);
            } else {
                e.contadores.insert(nombre.to_string(), delta);
            }
        }
    }

    /// Documento con lo registrado hasta ahora (`None` si está apagado). No escribe nada.
    ///
    /// - `app.build` vacío se rellena con el perfil de compilación; si es `debug` se añade
    ///   `debug_build` a `invalidated` (nunca se mide con `debug`).
    /// - Si `RunInfo::primary` nombra una serie con muestras, `reps[0]` y `summary` salen de
    ///   ella; si no tiene muestras, se anota `sin_muestras:<serie>` en `invalidated`.
    pub fn report(&self) -> Option<Results> {
        let a = self.activo.as_ref()?;
        let e = a.bloquear();
        let mut app = e.run.app.clone();
        if app.build.is_empty() {
            app.build = perfil_de_compilacion().to_string();
        }
        let mut invalidated = Vec::new();
        if app.build == "debug" {
            invalidated.push("debug_build".to_string());
        }
        let run_id = if e.run.run_id.is_empty() {
            a.ruta
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            e.run.run_id.clone()
        };
        let (reps, summary) = match e.run.primary.as_deref() {
            None => (Vec::new(), Summary::default()),
            Some(nombre) => match e.series.get(nombre).filter(|h| !h.is_empty()) {
                Some(h) => (
                    Rep::from_histogram(1, h).into_iter().collect(),
                    Summary::from_histogram(h),
                ),
                None => {
                    invalidated.push(format!("sin_muestras:{nombre}"));
                    (Vec::new(), Summary::default())
                }
            },
        };
        Some(Results {
            schema: SCHEMA.to_string(),
            run_id,
            criteria_sha: None,
            method: Method::Internal,
            env: e.run.env.clone(),
            app,
            file: e.run.file.clone(),
            measure: e.run.measure.clone(),
            scenario: e.run.scenario.clone(),
            cold: e.run.cold,
            capture: None,
            reps,
            summary,
            invalidated,
            internal: Some(Internal {
                percentile: PERCENTILE_DEFINITION.to_string(),
                marks: e.marcas.clone(),
                counters: e.contadores.clone(),
                histograms: e
                    .series
                    .iter()
                    .filter_map(|(k, h)| Some((k.clone(), h.summary()?)))
                    .collect(),
            }),
        })
    }

    /// Escribe el JSON en la ruta de salida y la devuelve. Apagado: no crea ningún archivo ni
    /// carpeta y devuelve `Ok(None)`. Encendido: crea las carpetas que falten y escribe un
    /// único archivo (primero uno temporal en la misma carpeta y luego lo renombra, de modo que
    /// nunca queda un JSON a medias); llamarlo otra vez lo sobrescribe.
    pub fn write_json(&self) -> Result<Option<PathBuf>, MetricsError> {
        let (Some(a), Some(doc)) = (&self.activo, self.report()) else {
            return Ok(None);
        };
        let texto = doc.to_json_string().map_err(MetricsError::Json)?;
        let io_err = |path: &Path| {
            let path = path.to_path_buf();
            move |source| MetricsError::Io { path, source }
        };
        if let Some(carpeta) = a.ruta.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(carpeta).map_err(io_err(carpeta))?;
        }
        let mut temporal = a.ruta.clone().into_os_string();
        temporal.push(".tmp");
        let temporal = PathBuf::from(temporal);
        fs::write(&temporal, texto).map_err(io_err(&temporal))?;
        if let Err(e) = fs::rename(&temporal, &a.ruta) {
            let _ = fs::remove_file(&temporal);
            return Err(io_err(&a.ruta)(e));
        }
        Ok(Some(a.ruta.clone()))
    }
}

impl Activo {
    /// Un hilo que entró en pánico con el candado no debe tumbar la medición.
    fn bloquear(&self) -> MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn empieza_por_guiones(valor: &OsString) -> bool {
    valor.to_str().is_some_and(|t| t.starts_with("--"))
}

/// Guardia RAII de un tramo cronometrado (ver [`Recorder::span`]).
#[must_use = "la guardia mide hasta que se suelta; si no se guarda en una variable, mide 0"]
pub struct Span<'a> {
    destino: Option<(&'a Recorder, Instant)>,
    nombre: &'a str,
}

impl Span<'_> {
    /// Termina el tramo ya y devuelve su duración en ms (`None` si el Recorder está apagado).
    pub fn finish(mut self) -> Option<f64> {
        self.cerrar()
    }

    /// Descarta el tramo sin registrar nada (p. ej. si la operación falló).
    pub fn cancel(mut self) {
        self.destino = None;
    }

    fn cerrar(&mut self) -> Option<f64> {
        let (rec, t0) = self.destino.take()?;
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        rec.sample(self.nombre, ms);
        Some(ms)
    }
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        self.cerrar();
    }
}
