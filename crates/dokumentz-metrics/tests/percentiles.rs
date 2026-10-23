//! Percentiles de `Histogram` contra casos calculados a mano (T012).
//!
//! Definición bajo prueba: rango más cercano («nearest-rank»). Con `n` muestras ordenadas
//! `x[1] <= … <= x[n]`, el percentil `p` (0 < p <= 100) es `x[k]` con `k = ceil(p * n / 100)`;
//! `p = 0` devuelve el mínimo. Nunca interpola: el resultado es siempre una muestra real.

use dokumentz_metrics::Histogram;

fn con(muestras: &[f64]) -> Histogram {
    let mut h = Histogram::new();
    for &m in muestras {
        assert!(h.record(m));
    }
    h
}

#[test]
fn caso_clasico_de_cinco_muestras() {
    // Ejemplo canónico del método (lista ordenada 15, 20, 35, 40, 50), desordenado a propósito.
    let h = con(&[40.0, 15.0, 50.0, 20.0, 35.0]);
    // k = ceil(30*5/100) = ceil(1,5) = 2 -> 20
    assert_eq!(h.percentile(30.0), Some(20.0));
    // k = ceil(40*5/100) = 2 exacto -> 20 (el límite no salta a la muestra siguiente)
    assert_eq!(h.percentile(40.0), Some(20.0));
    // k = ceil(2,5) = 3 -> 35
    assert_eq!(h.percentile(50.0), Some(35.0));
    // k = ceil(3,75) = 4 -> 40
    assert_eq!(h.percentile(75.0), Some(40.0));
    assert_eq!(h.percentile(100.0), Some(50.0));
    assert_eq!(h.percentile(0.0), Some(15.0));
}

#[test]
fn de_uno_a_cien() {
    let datos: Vec<f64> = (1..=100).map(f64::from).collect();
    let h = con(&datos);
    assert_eq!(h.p50(), Some(50.0));
    assert_eq!(h.p95(), Some(95.0));
    assert_eq!(h.p99(), Some(99.0));
}

#[test]
fn de_uno_a_diez_redondea_hacia_arriba() {
    let datos: Vec<f64> = (1..=10).map(f64::from).collect();
    let h = con(&datos);
    // k = 5 exacto
    assert_eq!(h.p50(), Some(5.0));
    // k = ceil(9,5) = 10
    assert_eq!(h.p95(), Some(10.0));
    assert_eq!(h.p99(), Some(10.0));
}

#[test]
fn una_sola_muestra_y_vacio() {
    let h = con(&[16.7]);
    assert_eq!(h.p50(), Some(16.7));
    assert_eq!(h.p95(), Some(16.7));
    assert_eq!(h.p99(), Some(16.7));

    let vacio = Histogram::new();
    assert_eq!(vacio.p50(), None);
    assert_eq!(vacio.percentile(95.0), None);
    assert_eq!(vacio.min(), None);
    assert_eq!(vacio.max(), None);
    assert_eq!(vacio.mean(), None);
    assert!(vacio.summary().is_none());
}

#[test]
fn con_repetidos_y_cuadros_largos() {
    // 98 cuadros de 16,7 ms y dos lentos: el p95 sigue en 16,7 y el p99 salta al lento.
    let mut datos = vec![16.7; 98];
    datos.push(33.4);
    datos.push(50.1);
    let h = con(&datos);
    assert_eq!(h.len(), 100);
    // k = 95 -> 16,7 ; k = 99 -> 33,4 ; máximo 50,1
    assert_eq!(h.p95(), Some(16.7));
    assert_eq!(h.p99(), Some(33.4));
    assert_eq!(h.max(), Some(50.1));
    assert_eq!(h.min(), Some(16.7));
}

#[test]
fn percentil_fuera_de_rango_se_acota() {
    let h = con(&[1.0, 2.0, 3.0]);
    assert_eq!(h.percentile(-5.0), Some(1.0));
    assert_eq!(h.percentile(250.0), Some(3.0));
    // NaN no es un percentil válido.
    assert_eq!(h.percentile(f64::NAN), None);
}

#[test]
fn rechaza_muestras_no_finitas() {
    let mut h = Histogram::new();
    assert!(!h.record(f64::NAN));
    assert!(!h.record(f64::INFINITY));
    assert!(!h.record(f64::NEG_INFINITY));
    assert!(h.is_empty());
    assert!(h.record(2.0));
    assert_eq!(h.len(), 1);
}

#[test]
fn resumen_con_redondeo_a_micros() {
    let h = con(&[1.0, 2.0, 4.0]);
    let r = h.summary().expect("hay muestras");
    assert_eq!(r.count, 3);
    assert_eq!(r.min, 1.0);
    assert_eq!(r.max, 4.0);
    // media = 7/3 = 2,333333… -> redondeada a 3 decimales
    assert_eq!(r.mean, 2.333);
    // k = ceil(1,5) = 2 -> 2 ; k = ceil(2,85) = 3 -> 4 ; k = ceil(2,97) = 3 -> 4
    assert_eq!((r.p50, r.p95, r.p99), (2.0, 4.0, 4.0));
}
