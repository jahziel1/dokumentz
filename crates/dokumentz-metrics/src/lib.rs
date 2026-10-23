//! Instrumentación interna de Dokumentz (FR-012) y tipos del esquema de resultados.
//!
//! - [`Recorder`]: apagado por defecto; se enciende con `--metrics-out <ruta>` o
//!   `DOKUMENTZ_METRICS_OUT`. Marcas ([`Recorder::mark`]), tramos cronometrados
//!   ([`Recorder::span`] → [`Span`]), series de muestras ([`Recorder::sample`]), contadores
//!   ([`Recorder::count`]) y [`Recorder::write_json`].
//! - [`Histogram`]: muestras con percentiles por rango más cercano («nearest-rank»; definición en
//!   [`histogram`]).
//! - [`results`]: tipos del JSON `dokumentz-bench/1` (`plan.md` §9.7).
//!
//! Solo local: sin red, sin cuentas, sin telemetría. Las dependencias son `serde` y
//! `serde_json`; la prueba `off_and_no_net` vigila que no entre ninguna biblioteca de red.

pub mod histogram;
pub mod recorder;
pub mod results;

pub use histogram::{Histogram, HistogramSummary, percentile_sorted};
pub use recorder::{ArgsError, ENV_VAR, FLAG, MetricsError, Recorder, Span, names};
pub use results::{Results, RunInfo};

/// Versión del crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_no_esta_vacia() {
        assert!(!version().is_empty());
    }
}
