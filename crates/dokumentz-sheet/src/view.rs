//! Vista de la hoja: hoja visible, desplazamiento y tamaño de la ventana (`plan.md` §5.3).
//!
//! El desplazamiento se guarda en **píxeles lógicos de hoja**, no en filas: así el mismo
//! valor sirve a la interfaz de Tauri (con su desplazamiento escalado) y a la de GPUI.
//! La rueda y las barras (`Core::scroll_to`) acotan `scroll` a `[0, extensión - área de celdas]`;
//! la navegación por teclado (`nav.rs`) puede llevarlo más allá, hasta el tamaño de la hoja, y
//! entonces la extensión crece para cubrir lo que se ve.

use serde::{Deserialize, Serialize};

use crate::drawlist::DrawRequest;
use crate::geometry::{COL_HEADER_H, ROW_HEADER_W};

/// Estado de la vista de la hoja. Los campos son públicos para leerlos; para cambiarlos use
/// `Core::{set_viewport, scroll_to, scroll_by}`, que los mantienen acotados.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Hoja que se muestra.
    pub sheet: u32,
    /// Desplazamiento horizontal: coordenada x de hoja que queda en el borde izquierdo del
    /// área de celdas.
    pub scroll_x: f64,
    /// Desplazamiento vertical (ídem, y).
    pub scroll_y: f64,
    /// Ancho de todo el control de la cuadrícula, cabecera de filas incluida (px lógicos).
    pub viewport_w: f32,
    /// Alto de todo el control, cabecera de columnas incluida (px lógicos).
    pub viewport_h: f32,
    /// Factor de escala de la pantalla (píxeles físicos por lógico). Solo informativo para el
    /// núcleo: las medidas de la hoja siempre son lógicas.
    pub scale: f32,
}

impl Default for View {
    fn default() -> Self {
        View {
            sheet: 0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            // Un tamaño razonable hasta que la interfaz mande `set_viewport`.
            viewport_w: 1280.0,
            viewport_h: 800.0,
            scale: 1.0,
        }
    }
}

impl View {
    /// Ancho del área de celdas: el control menos la cabecera de filas.
    pub fn cells_w(&self) -> f64 {
        f64::from((self.viewport_w - ROW_HEADER_W).max(0.0))
    }

    /// Alto del área de celdas: el control menos la cabecera de columnas.
    pub fn cells_h(&self) -> f64 {
        f64::from((self.viewport_h - COL_HEADER_H).max(0.0))
    }

    /// Pasa un punto del control (px lógicos, origen en la esquina superior izquierda, cabeceras
    /// incluidas) a coordenadas de hoja: se descuentan las cabeceras y se suma el
    /// desplazamiento. Un punto sobre una cabecera da una coordenada menor que el borde visible
    /// (puede ser negativa, y `hit_test` la trata aparte).
    pub fn to_sheet(&self, x: f32, y: f32) -> (f64, f64) {
        (
            self.scroll_x + f64::from(x - ROW_HEADER_W),
            self.scroll_y + f64::from(y - COL_HEADER_H),
        )
    }

    /// Petición de bloque que cubre el área de celdas visible, con los márgenes dados (en px
    /// de hoja) a cada lado. Es lo que pide la interfaz para pintar y precargar.
    pub fn request(&self, margin_x: f32, margin_y: f32) -> DrawRequest {
        DrawRequest {
            sheet: self.sheet,
            x0: self.scroll_x,
            y0: self.scroll_y,
            w: self.cells_w() as f32,
            h: self.cells_h() as f32,
            margin_x,
            margin_y,
        }
    }
}

/// Medidas de la hoja que la interfaz necesita para su barra de desplazamiento.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SheetMetrics {
    /// Extensión desplazable `(ancho, alto)` en px de hoja: rango usado, más una pantalla de
    /// holgura, ampliado para incluir la selección (así la celda activa siempre es alcanzable)
    /// y lo que se está viendo (así la barra representa adonde llegó el teclado).
    pub extent: (f64, f64),
    /// Máximo desplazamiento `(x, y)`: `max(0, extensión - área de celdas)`.
    pub max_scroll: (f64, f64),
    /// Última fila con dato (0 si la hoja está vacía).
    pub used_rows: i32,
    /// Última columna con dato (0 si la hoja está vacía).
    pub used_cols: i32,
}
