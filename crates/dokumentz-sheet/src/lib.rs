//! Motor de la hoja de cálculo de Dokumentz.
//!
//! No depende de ninguna biblioteca de interfaz (principio III de la constitución);
//! la prueba `tests/no_ui_deps.rs` lo comprueba sobre el grafo de dependencias.
//!
//! Todo lo que decide qué se ve (texto formateado, estilo resuelto, geometría) lo calcula este
//! crate; las interfaces solo pintan. La entrada es [`Core`]; el bloque de cuadrícula que
//! pintan es un [`DrawList`], que `codec` pasa al formato binario v1 (`plan.md` §5).
//!
//! # Cómo se usa
//!
//! La interfaz guarda un [`Core`] en un `Mutex` (`plan.md` §5.11) y habla con él con tres
//! llamadas: [`Core::apply`] (un [`Command`] entra, un [`UiState`] sale), [`Core::draw_block`]
//! (el bloque de la cuadrícula que hay que pintar) y [`Core::encode_block`] (ese bloque en el
//! formato binario v1, para enviarlo por el puente). `Command` y `UiState` se serializan a
//! JSON de forma estable (ver [`command`]).
//!
//! ```
//! use std::sync::Mutex;
//! use dokumentz_sheet::{Advance, Command, Core, EditMode, OpenOptions};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Un libro nuevo (para abrir un archivo: `Core::open_xlsx(ruta, opciones)`).
//! let core = Mutex::new(Core::new_empty(OpenOptions::default())?);
//!
//! // El comando llega como JSON desde la interfaz web, o como valor desde GPUI.
//! let json = r#"{"SetViewport":{"w":1280.0,"h":800.0,"scale":2.0}}"#;
//! let comando: Command = serde_json::from_str(json)?;
//! core.lock().unwrap().apply(comando)?;
//!
//! // Escribir un valor en A1 y confirmar con Intro: la celda activa baja a A2.
//! let estado = {
//!     let mut core = core.lock().unwrap();
//!     core.apply(Command::BeginEdit { mode: EditMode::Replace, seed: Some("7".into()) })?;
//!     core.apply(Command::CommitEdit { text: "7".into(), advance: Advance::enter(false) })?
//! };
//! assert_eq!(estado.active, (2, 1));
//! assert!(estado.dirty && estado.can_undo);
//! let para_la_interfaz: String = serde_json::to_string(&estado)?; // el `UiState` viaja así
//! assert!(para_la_interfaz.starts_with(r#"{"data_version":"#));
//!
//! // Pintar: el bloque de pantalla más márgenes, listo para codificar y enviar.
//! let (bloque, bytes) = {
//!     let mut core = core.lock().unwrap();
//!     let peticion = core.view().request(0.0, 400.0);
//!     let bloque = core.draw_block(peticion)?;
//!     let bytes = core.encode_block(&bloque);
//!     (bloque, bytes)
//! };
//! assert_eq!(bloque.cell_text(&bloque.cells[0]), "7");
//! assert_eq!(&bytes[..4], &0x3144_4C50_u32.to_le_bytes()); // `PLD1`
//! # Ok(()) }
//! ```
//!
//! # Característica `metrics`
//!
//! Con `--features metrics` el crate depende de `dokumentz-metrics` y `Core::set_recorder`
//! conecta un `Recorder`; sin ella el crate no depende de él y no hay ni una instrucción de
//! medición (FR-012, `plan.md` §5.10). Un `Recorder` apagado no cuesta nada.

pub mod clipboard;
pub mod codec;
pub mod command;
pub mod core;
pub mod drawlist;
pub mod edit;
pub mod error;
pub mod geometry;
pub mod nav;
pub mod save;
pub mod view;

pub use crate::command::{Command, UiState};
pub use crate::core::{Core, Lang, OpenOptions, SheetTab, locale_for_region};
pub use crate::drawlist::{
    Align, CellStyle, DrawCell, DrawList, DrawRequest, Kind, Rgba, StrRef, VAlign,
};
pub use crate::edit::{Advance, EditMode, EditState};
pub use crate::error::{CodecError, CoreError};
pub use crate::geometry::{COL_HEADER_H, ROW_HEADER_W, SheetGeometry};
pub use crate::nav::{Dir, Hit, Selection};
pub use crate::view::{SheetMetrics, View};

/// Nombres de las series y contadores del núcleo (característica `metrics`).
#[cfg(feature = "metrics")]
pub use crate::core::nombres as metric_names;

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
