//! Comparación de un guion con su resultado: la lógica de `dokumentz-script-check`.
//!
//! `check(esperado, resultado)` **no se fía** del `ok` que anota la interfaz: vuelve a comparar
//! cada valor observado con el esperado del guion.
//!
//! # Reglas
//!
//! 1. El resultado debe tener `ok: true` y ningún paso con `ok: false`.
//! 2. Si el esperado es un guion completo (tiene pasos que no son `assert_*`), el resultado debe
//!    tener los mismos pasos, en el mismo orden y de las mismas clases (el guion se ejecutó
//!    entero). Si solo tiene `assert_*` (guion reducido), esta regla no se aplica.
//! 3. Las comprobaciones se emparejan **por orden**: la n-ésima `assert_*` del esperado con la
//!    n-ésima del resultado; deben ser de la misma clase y haber el mismo número.
//! 4. Comparación: `assert_cell`: celda (canónica), hoja (si las dos la dan), `value` y
//!    `formula` exactos si el esperado los da; `assert_selection`: rango canónico (esquinas
//!    ordenadas) y celda activa si el esperado la da; `assert_formula_bar`: texto exacto.

use std::ffi::OsString;
use std::fmt;
use std::io::Write;

use crate::cell::{normalize_cell, normalize_range};
use crate::result::{Observed, ScriptResult, StepResult};
use crate::script::{Script, Step};

/// Una diferencia entre lo esperado y lo observado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// Índice del paso del **resultado** al que se refiere (si aplica).
    pub step: Option<usize>,
    /// Qué difiere.
    pub message: String,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.step {
            Some(i) => write!(f, "paso {i}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

fn dif(step: Option<usize>, message: impl Into<String>) -> Mismatch {
    Mismatch {
        step,
        message: message.into(),
    }
}

/// Número de `assert_*` que comparó [`check`] y las diferencias halladas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// Comprobaciones emparejadas.
    pub compared: usize,
    /// Diferencias (vacío: todo coincide).
    pub mismatches: Vec<Mismatch>,
}

impl Report {
    /// `true` si no hubo diferencias.
    pub fn is_ok(&self) -> bool {
        self.mismatches.is_empty()
    }
}

/// Compara `esperado` con `resultado` según las reglas del módulo.
pub fn check(esperado: &Script, resultado: &ScriptResult) -> Report {
    let mut informe = Report::default();
    let m = &mut informe.mismatches;

    if !resultado.ok {
        let motivo = resultado.error.as_deref().unwrap_or("sin motivo");
        m.push(dif(None, format!("el guion no terminó bien: {motivo}")));
    }
    for paso in resultado.steps.iter().filter(|p| !p.ok) {
        let motivo = paso.error.as_deref().unwrap_or("sin motivo");
        m.push(dif(
            Some(paso.index),
            format!("{} falló: {motivo}", paso.step),
        ));
    }

    let completo = esperado.steps.iter().any(|s| !s.kind().is_assert());
    if completo {
        if esperado.steps.len() != resultado.steps.len() {
            m.push(dif(
                None,
                format!(
                    "el guion tiene {} pasos y el resultado {}",
                    esperado.steps.len(),
                    resultado.steps.len()
                ),
            ));
        }
        for (i, (e, r)) in esperado.steps.iter().zip(&resultado.steps).enumerate() {
            if e.kind() != r.step {
                m.push(dif(
                    Some(r.index),
                    format!(
                        "se esperaba `{}` en la posición {i} y hay `{}`",
                        e.kind(),
                        r.step
                    ),
                ));
            }
        }
    }

    let esperadas: Vec<&Step> = esperado
        .steps
        .iter()
        .filter(|s| s.kind().is_assert())
        .collect();
    let observadas: Vec<&StepResult> = resultado
        .steps
        .iter()
        .filter(|p| p.step.is_assert())
        .collect();
    if esperadas.len() != observadas.len() {
        m.push(dif(
            None,
            format!(
                "el esperado tiene {} comprobaciones y el resultado {}",
                esperadas.len(),
                observadas.len()
            ),
        ));
    }
    for (e, r) in esperadas.iter().zip(&observadas) {
        informe.compared += 1;
        comparar(e, r, &mut informe.mismatches);
    }
    informe
}

fn comparar(esperado: &Step, r: &StepResult, m: &mut Vec<Mismatch>) {
    let i = Some(r.index);
    if esperado.kind() != r.step {
        m.push(dif(
            i,
            format!("se esperaba `{}` y hay `{}`", esperado.kind(), r.step),
        ));
        return;
    }
    let Some(obs) = &r.observed else {
        m.push(dif(i, format!("`{}` no trae el valor observado", r.step)));
        return;
    };
    match (esperado, obs) {
        (
            Step::AssertCell {
                cell,
                sheet,
                value,
                formula,
            },
            Observed::Cell {
                cell: c,
                sheet: s,
                value: v,
                formula: f,
            },
        ) => {
            if normalize_cell(cell) != normalize_cell(c) {
                m.push(dif(
                    i,
                    format!("se leyó la celda `{c}` y se esperaba `{cell}`"),
                ));
            }
            if let (Some(es), Some(os)) = (sheet, s)
                && es != os
            {
                m.push(dif(i, format!("hoja `{os}` en lugar de `{es}`")));
            }
            if let Some(ev) = value
                && ev != v
            {
                m.push(dif(i, format!("{cell}: valor {v:?} en lugar de {ev:?}")));
            }
            if let Some(ef) = formula
                && f.as_ref() != Some(ef)
            {
                m.push(dif(
                    i,
                    format!("{cell}: contenido {f:?} en lugar de {ef:?}"),
                ));
            }
        }
        (
            Step::AssertSelection { range, active },
            Observed::Selection {
                range: r2,
                active: a2,
            },
        ) => {
            if normalize_range(range) != normalize_range(r2) {
                m.push(dif(i, format!("selección `{r2}` en lugar de `{range}`")));
            }
            if let Some(ea) = active {
                let igual = a2.as_deref().and_then(normalize_cell) == normalize_cell(ea);
                if !igual {
                    m.push(dif(i, format!("celda activa {a2:?} en lugar de `{ea}`")));
                }
            }
        }
        (Step::AssertFormulaBar { text }, Observed::FormulaBar { text: t }) => {
            if text != t {
                m.push(dif(
                    i,
                    format!("barra de fórmulas {t:?} en lugar de {text:?}"),
                ));
            }
        }
        _ => m.push(dif(
            i,
            "el valor observado no corresponde a la clase del paso",
        )),
    }
}

/// Texto de uso.
pub const USO: &str = "uso: dokumentz-script-check <esperado.json> <resultado.json>\n\
\n\
Compara un guion (los valores esperados están en sus pasos assert_*) con el resultado que\n\
escribió una interfaz con --script-out.\n\
\n\
códigos de salida: 0 coincide; 1 hay diferencias; 2 error de uso, de lectura o de formato\n";

/// Cuerpo de `dokumentz-script-check`: `args` sin el nombre del programa. Devuelve el código de
/// salida (0 coincide, 1 diferencias, 2 error de uso o de lectura).
pub fn run_cli(args: &[OsString], salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        let _ = salida.write_all(USO.as_bytes());
        return 0;
    }
    let [esperado, resultado] = args else {
        let _ = errores.write_all(USO.as_bytes());
        return 2;
    };
    let esperado_s = match Script::from_path(esperado) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(
                errores,
                "error en el esperado {}: {e}",
                esperado.to_string_lossy()
            );
            return 2;
        }
    };
    let resultado_s = match ScriptResult::from_path(resultado) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(
                errores,
                "error en el resultado {}: {e}",
                resultado.to_string_lossy()
            );
            return 2;
        }
    };
    let informe = check(&esperado_s, &resultado_s);
    if informe.is_ok() {
        let _ = writeln!(salida, "OK: {} comprobaciones coinciden", informe.compared);
        0
    } else {
        for d in &informe.mismatches {
            let _ = writeln!(errores, "DIFERENCIA {d}");
        }
        let _ = writeln!(errores, "FALLA: {} diferencias", informe.mismatches.len());
        1
    }
}
