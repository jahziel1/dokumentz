// Rejilla sintética tipo hoja para el spike de GPUI (E03 / T007).
//
// Un `Element` propio (como hace `input.rs` de Zed y como prevé `plan.md` §7.2) que en cada
// cuadro pinta 2 000 `paint_quad` (50 columnas x 40 filas de celdas) y 500 `ShapedLine` (una
// de cada cuatro celdas lleva texto). Mide la duración de `prepaint` y `paint` (instrumentación
// interna, §7.8) en contadores atómicos que `main.rs` imprime al salir. No es un benchmark:
// el rendimiento se mide en la fase 0b con la interfaz real.
//
// Procedencia: código propio de Dokumentz, escrito contra la API de GPUI del commit
// 9e84b066b55bc3609a30dd965075c12255b38b45 de https://github.com/zed-industries/zed (Apache-2.0).
// La estructura del `Element` sigue la de `crates/gpui/examples/input.rs` de ese commit.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use gpui::{
    App, BorderStyle, Bounds, ContentMask, Edges, Element, ElementId, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, LayoutId, PaintQuad, Pixels, Point, ShapedLine, Style,
    TextAlign, TextRun, Window, point, px, quad, relative, rgb, size,
};

/// Columnas y filas de la rejilla: 50 x 40 = 2 000 celdas = 2 000 `paint_quad`.
pub const COLS: usize = 50;
pub const ROWS: usize = 40;
/// Una de cada `TEXT_EVERY` celdas lleva texto: 2 000 / 4 = 500 `ShapedLine`.
pub const TEXT_EVERY: usize = 4;
pub const EXPECTED_QUADS: usize = COLS * ROWS;
pub const EXPECTED_LINES: usize = EXPECTED_QUADS / TEXT_EVERY;

const FONT_SIZE: f32 = 10.0;
/// Alto de línea de los textos de celda (la altura de la celda lo recorta si es menor).
const LINE_HEIGHT: f32 = 13.0;

/// Contadores de instrumentación, leídos por `main.rs`.
pub static FRAMES: AtomicU64 = AtomicU64::new(0);
pub static PREPAINT_NS: AtomicU64 = AtomicU64::new(0);
pub static PAINT_NS: AtomicU64 = AtomicU64::new(0);
pub static PREPAINT_MAX_NS: AtomicU64 = AtomicU64::new(0);
pub static PAINT_MAX_NS: AtomicU64 = AtomicU64::new(0);
pub static LAST_QUADS: AtomicU64 = AtomicU64::new(0);
pub static LAST_LINES: AtomicU64 = AtomicU64::new(0);
pub static LAST_WIDTH_BITS: AtomicU32 = AtomicU32::new(0);
pub static LAST_HEIGHT_BITS: AtomicU32 = AtomicU32::new(0);
/// Si está activo, cada cuadro pide el siguiente (repintado continuo para medir).
pub static ANIMATE: AtomicBool = AtomicBool::new(false);

pub struct GridElement;

pub struct GridPaint {
    quads: Vec<PaintQuad>,
    lines: Vec<(Point<Pixels>, ShapedLine)>,
}

impl IntoElement for GridElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn fetch_max(slot: &AtomicU64, value: u64) {
    slot.fetch_max(value, Ordering::Relaxed);
}

impl Element for GridElement {
    type RequestLayoutState = ();
    type PrepaintState = GridPaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let t0 = Instant::now();

        // La rejilla se reparte en el espacio disponible: siempre se ven las 2 000 celdas.
        let cell_w = bounds.size.width / COLS as f32;
        let cell_h = bounds.size.height / ROWS as f32;

        let style = window.text_style();
        let font = style.font();
        let grid_line: Hsla = rgb(0xd0d7de).into();
        let text_color: Hsla = rgb(0x1f2328).into();
        let frame = FRAMES.load(Ordering::Relaxed);

        let mut quads = Vec::with_capacity(EXPECTED_QUADS);
        let mut lines = Vec::with_capacity(EXPECTED_LINES);

        for row in 0..ROWS {
            for col in 0..COLS {
                let index = row * COLS + col;
                let origin = point(
                    bounds.left() + cell_w * col as f32,
                    bounds.top() + cell_h * row as f32,
                );
                let cell = Bounds::new(origin, size(cell_w, cell_h));

                // Cabeceras de fila y columna más oscuras; resto en bandas; una celda
                // «seleccionada» que se desplaza con el cuadro cuando hay animación.
                let selected = ANIMATE.load(Ordering::Relaxed)
                    && index as u64 == (frame / 2) % EXPECTED_QUADS as u64;
                let background: Hsla = if selected {
                    rgb(0xb6d4fe).into()
                } else if row == 0 || col == 0 {
                    rgb(0xeaeef2).into()
                } else if (row + col) % 2 == 0 {
                    rgb(0xffffff).into()
                } else {
                    rgb(0xf6f8fa).into()
                };
                quads.push(quad(
                    cell,
                    px(0.),
                    background,
                    Edges::all(px(0.5)),
                    grid_line,
                    BorderStyle::default(),
                ));

                if index.is_multiple_of(TEXT_EVERY) {
                    let text = format!("{}", (index * 7) % 1000);
                    let run = TextRun {
                        len: text.len(),
                        font: font.clone(),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let line =
                        window
                            .text_system()
                            .shape_line(text.into(), px(FONT_SIZE), &[run], None);
                    // Texto centrado en vertical dentro de la celda.
                    let text_origin = point(
                        origin.x + px(2.),
                        origin.y + (cell_h - px(LINE_HEIGHT)) / 2.,
                    );
                    lines.push((text_origin, line));
                }
            }
        }

        let ns = t0.elapsed().as_nanos() as u64;
        PREPAINT_NS.fetch_add(ns, Ordering::Relaxed);
        fetch_max(&PREPAINT_MAX_NS, ns);
        GridPaint { quads, lines }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let t0 = Instant::now();
        let n_quads = prepaint.quads.len() as u64;
        let n_lines = prepaint.lines.len() as u64;

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for q in prepaint.quads.drain(..) {
                window.paint_quad(q);
            }
            for (origin, line) in prepaint.lines.drain(..) {
                line.paint(origin, px(LINE_HEIGHT), TextAlign::Left, None, window, cx)
                    .ok();
            }
        });

        let ns = t0.elapsed().as_nanos() as u64;
        PAINT_NS.fetch_add(ns, Ordering::Relaxed);
        fetch_max(&PAINT_MAX_NS, ns);
        FRAMES.fetch_add(1, Ordering::Relaxed);
        LAST_QUADS.store(n_quads, Ordering::Relaxed);
        LAST_LINES.store(n_lines, Ordering::Relaxed);
        LAST_WIDTH_BITS.store(f32::from(bounds.size.width).to_bits(), Ordering::Relaxed);
        LAST_HEIGHT_BITS.store(f32::from(bounds.size.height).to_bits(), Ordering::Relaxed);

        if ANIMATE.load(Ordering::Relaxed) {
            window.request_animation_frame();
        }
    }
}
