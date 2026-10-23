//! `Histogram`: muestras en memoria con percentiles.
//!
//! # Definición de percentil
//!
//! Se usa el método del **rango más cercano** («nearest-rank», el de Wikipedia/NIST sin
//! interpolación). Con `n` muestras ordenadas de menor a mayor, `x[1] <= … <= x[n]`, el
//! percentil `p` con `0 < p <= 100` es la muestra `x[k]` con
//!
//! ```text
//! k = ceil(p * n / 100)      (k se acota a 1..=n)
//! ```
//!
//! y `p = 0` devuelve el mínimo. Consecuencias que importan al leer los informes:
//!
//! - El resultado es **siempre una muestra real**; nunca se interpola entre dos.
//! - Con pocas muestras el p95 y el p99 coinciden con el máximo (con `n <= 20`, `p95 = max`).
//! - Una sola muestra es su propio p50, p95 y p99.
//!
//! Las mediciones de `bench-runner` (externo) usan la misma definición para que los números
//! internos y los externos sean comparables en método (no en valor: ver `plan.md` §9.9).
//!
//! # Redondeo
//!
//! [`Histogram::summary`] redondea sus valores a 3 decimales (microsegundos si la unidad es
//! ms): el JSON resultante es estable y no arrastra ruido de coma flotante.

use serde::{Deserialize, Serialize};

/// Resumen de un [`Histogram`] listo para serializar. Los campos están en el orden en que
/// aparecen en el JSON.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HistogramSummary {
    /// Número de muestras.
    pub count: u64,
    /// Mínimo.
    pub min: f64,
    /// Máximo.
    pub max: f64,
    /// Media aritmética (redondeada a 3 decimales).
    pub mean: f64,
    /// Percentil 50 (rango más cercano).
    pub p50: f64,
    /// Percentil 95 (rango más cercano).
    pub p95: f64,
    /// Percentil 99 (rango más cercano).
    pub p99: f64,
}

/// Conjunto de muestras `f64` (la unidad la fija el nombre de la serie: por convención las
/// duraciones terminan en `_ms`). Solo guarda muestras finitas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Histogram {
    muestras: Vec<f64>,
}

impl Histogram {
    /// Histograma vacío. No reserva memoria hasta la primera muestra.
    pub fn new() -> Self {
        Self::default()
    }

    /// Histograma que ya parte de `muestras`; descarta las no finitas.
    pub fn from_samples(muestras: impl IntoIterator<Item = f64>) -> Self {
        let mut h = Self::new();
        for m in muestras {
            h.record(m);
        }
        h
    }

    /// Añade una muestra. Devuelve `false` (y no la guarda) si no es finita (`NaN` o ±∞).
    pub fn record(&mut self, valor: f64) -> bool {
        if !valor.is_finite() {
            return false;
        }
        self.muestras.push(valor);
        true
    }

    /// Número de muestras guardadas.
    pub fn len(&self) -> usize {
        self.muestras.len()
    }

    /// `true` si no hay muestras.
    pub fn is_empty(&self) -> bool {
        self.muestras.is_empty()
    }

    /// Las muestras en el orden en que se registraron.
    pub fn samples(&self) -> &[f64] {
        &self.muestras
    }

    /// Mínimo, o `None` si está vacío.
    pub fn min(&self) -> Option<f64> {
        self.muestras.iter().copied().min_by(f64::total_cmp)
    }

    /// Máximo, o `None` si está vacío.
    pub fn max(&self) -> Option<f64> {
        self.muestras.iter().copied().max_by(f64::total_cmp)
    }

    /// Suma de las muestras (0 si está vacío).
    pub fn sum(&self) -> f64 {
        self.muestras.iter().sum()
    }

    /// Media aritmética, o `None` si está vacío.
    pub fn mean(&self) -> Option<f64> {
        (!self.muestras.is_empty()).then(|| self.sum() / self.muestras.len() as f64)
    }

    /// Percentil `p` (en porcentaje, 0–100) por rango más cercano; ver la definición en la
    /// documentación del módulo. `p` fuera de rango se acota a `[0, 100]`; `NaN` o un
    /// histograma vacío devuelven `None`. Ordena una copia: para varios percentiles use
    /// [`Histogram::summary`].
    pub fn percentile(&self, p: f64) -> Option<f64> {
        percentile_sorted(&self.ordenadas(), p)
    }

    /// Percentil 50.
    pub fn p50(&self) -> Option<f64> {
        self.percentile(50.0)
    }

    /// Percentil 95.
    pub fn p95(&self) -> Option<f64> {
        self.percentile(95.0)
    }

    /// Percentil 99.
    pub fn p99(&self) -> Option<f64> {
        self.percentile(99.0)
    }

    /// Resumen (redondeado a 3 decimales) o `None` si está vacío. Ordena una sola vez.
    pub fn summary(&self) -> Option<HistogramSummary> {
        let orden = self.ordenadas();
        let primera = *orden.first()?;
        let ultima = *orden.last()?;
        let pct = |p| percentile_sorted(&orden, p).map_or(0.0, redondear);
        Some(HistogramSummary {
            count: orden.len() as u64,
            min: redondear(primera),
            max: redondear(ultima),
            mean: redondear(orden.iter().sum::<f64>() / orden.len() as f64),
            p50: pct(50.0),
            p95: pct(95.0),
            p99: pct(99.0),
        })
    }

    fn ordenadas(&self) -> Vec<f64> {
        let mut v = self.muestras.clone();
        v.sort_by(f64::total_cmp);
        v
    }
}

/// Percentil por rango más cercano sobre una lista **ya ordenada** de menor a mayor.
/// Mismas reglas que [`Histogram::percentile`].
pub fn percentile_sorted(ordenadas: &[f64], p: f64) -> Option<f64> {
    if p.is_nan() || ordenadas.is_empty() {
        return None;
    }
    let n = ordenadas.len();
    let p = p.clamp(0.0, 100.0);
    // `p * n` antes de dividir: con p y n enteros el producto es exacto y el cociente por 100
    // también cuando es entero, así que `ceil` no salta de rango por ruido de coma flotante.
    let rango = (p * n as f64 / 100.0).ceil() as usize;
    Some(ordenadas[rango.clamp(1, n) - 1])
}

/// Redondea a 3 decimales (ver «Redondeo» en la documentación del módulo).
pub(crate) fn redondear(valor: f64) -> f64 {
    (valor * 1000.0).round() / 1000.0
}
