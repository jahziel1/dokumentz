//! Guiones de prueba compartidos por las interfaces de Dokumentz (`plan.md` §5.10).
//!
//! - [`Script`] / [`Step`]: el guion JSON (`open`, `key`, `text`, `scroll`, `drag`, `wait_ms`,
//!   `wait_idle`, `mark`, `assert_cell`, `assert_selection`, `assert_formula_bar`, `save`,
//!   `quit`).
//! - [`ScriptResult`]: lo que cada interfaz escribe con `--script-out`.
//! - [`check()`]: compara el guion con el resultado; el binario `dokumentz-script-check` es su
//!   envoltorio de línea de comandos.
//!
//! El crate no depende del núcleo ni de ninguna interfaz: los guiones se ejecutan en cada
//! interfaz llamando a los mismos manejadores que la entrada real.

pub mod cell;
pub mod check;
pub mod result;
pub mod script;

pub use check::{Mismatch, Report, check};
pub use result::{Observed, RESULT_SCHEMA, ScriptResult, StepResult};
pub use script::{Modifier, Pos, SCHEMA, Script, ScriptError, Step, StepKind};

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
