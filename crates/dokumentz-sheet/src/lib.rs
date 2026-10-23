//! Motor de la hoja de cálculo de Dokumentz.
//!
//! No depende de ninguna biblioteca de interfaz (principio III de la constitución);
//! la prueba `tests/no_ui_deps.rs` lo comprueba sobre el grafo de dependencias.

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
