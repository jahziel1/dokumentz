//! Interfaz B (GPUI) de Dokumentz: spike de E03 / T007.
//!
//! Abre una ventana con (1) una rejilla sintética que un `Element` propio pinta con 2 000
//! `paint_quad` y 500 `ShapedLine` y (2) un cuadro de texto con `ElementInputHandler` adaptado de
//! un ejemplo de Zed (ver `text_input.rs`), listo para la prueba manual T008: tecla muerta,
//! `ñ`, IME japonés y dictado.
//!
//! Procedencia de la dependencia: GPUI y `gpui_platform` vienen por git de
//! <https://github.com/zed-industries/zed>, commit `9e84b066b55bc3609a30dd965075c12255b38b45`
//! (licencia Apache-2.0). El commit y los parches replicados están comentados en el `Cargo.toml`
//! raíz.
//!
//! Banderas:
//! - `--exit-after-ms <n>`: cierra la ventana sola tras `n` milisegundos e imprime las medidas
//!   de dibujo recogidas (para arrancar el binario sin intervención).
//! - `--animate`: repinta de forma continua (un cuadro tras otro) para recoger más muestras.

mod spike_grid;
mod text_input;

use std::sync::atomic::Ordering;
use std::time::Duration;

use gpui::{
    App, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, Keystroke, SharedString,
    Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size, white,
};
use gpui_platform::application;

use spike_grid::{
    ANIMATE, COLS, EXPECTED_LINES, EXPECTED_QUADS, FRAMES, GridElement, LAST_HEIGHT_BITS,
    LAST_LINES, LAST_QUADS, LAST_WIDTH_BITS, PAINT_MAX_NS, PAINT_NS, PREPAINT_MAX_NS, PREPAINT_NS,
    ROWS,
};
use text_input::TextInput;

/// Commit de Zed del que salen `gpui` y `gpui_platform` (debe coincidir con el `Cargo.toml` raíz).
const ZED_COMMIT: &str = "9e84b066b55bc3609a30dd965075c12255b38b45";

actions!(spike, [Quit]);

#[derive(Debug, Default, PartialEq)]
struct Args {
    exit_after_ms: Option<u64>,
    animate: bool,
}

/// Lee `--exit-after-ms <n>` y `--animate`. Un argumento desconocido o un valor inválido es un
/// error para no ejecutar el spike con una opción mal escrita sin avisar.
fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--exit-after-ms" => {
                let value = it.next().ok_or_else(|| {
                    "--exit-after-ms necesita un valor en milisegundos".to_string()
                })?;
                let ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("--exit-after-ms: «{value}» no es un entero válido"))?;
                out.exit_after_ms = Some(ms);
            }
            "--animate" => out.animate = true,
            other => return Err(format!("argumento desconocido: {other}")),
        }
    }
    Ok(out)
}

struct SpikeView {
    text_input: Entity<TextInput>,
    recent_keystrokes: Vec<Keystroke>,
    focus_handle: FocusHandle,
}

impl Focusable for SpikeView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpikeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = self.text_input.read(cx);
        let estado: SharedString = format!(
            "contenido: {:?}  seleccion: {:?}  composicion: {:?}  teclado: {}",
            input.content.as_ref(),
            input.selected_range,
            input.marked_range,
            cx.keyboard_layout().name(),
        )
        .into();
        let teclas: Vec<String> = self
            .recent_keystrokes
            .iter()
            .rev()
            .take(6)
            .map(|ks| {
                format!(
                    "{} {}",
                    ks.unparse(cx.platform_info().os),
                    ks.key_char
                        .as_ref()
                        .map(|c| format!("-> {c:?}"))
                        .unwrap_or_default()
                )
            })
            .collect();

        div()
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xaaaaaa))
            .text_size(px(12.))
            .child(div().bg(white()).px_2().child(format!(
                "Dokumentz - spike GPUI (E03/T007) - zed {} - {} celdas, {} lineas",
                &ZED_COMMIT[..9],
                EXPECTED_QUADS,
                EXPECTED_LINES
            )))
            .child(div().bg(white()).px_2().child(
                "T008: prueba aqui tecla muerta (´ + a), ñ, IME japones (Hiragana) y dictado",
            ))
            .child(self.text_input.clone())
            .child(div().bg(white()).px_2().child(estado))
            .child(div().bg(white()).px_2().child(teclas.join("   |   ")))
            // La rejilla ocupa el resto de la ventana.
            .child(div().flex_1().min_h_0().w_full().child(GridElement))
    }
}

fn mean_us(total_ns: u64, frames: u64) -> f64 {
    if frames == 0 {
        0.0
    } else {
        total_ns as f64 / frames as f64 / 1000.0
    }
}

fn print_stats(requested_ms: Option<u64>, refresh: Option<Duration>, scale: f32) {
    let frames = FRAMES.load(Ordering::Relaxed);
    println!(
        "spike-gpui: zed={ZED_COMMIT} salida_ms={} cuadros={frames} quads={} lineas={} \
         rejilla={}x{} area_px={:.0}x{:.0} escala={scale} refresco_ms={} \
         prepaint_medio_us={:.1} prepaint_max_us={:.1} paint_medio_us={:.1} paint_max_us={:.1}",
        requested_ms.map_or("-".to_string(), |ms| ms.to_string()),
        LAST_QUADS.load(Ordering::Relaxed),
        LAST_LINES.load(Ordering::Relaxed),
        COLS,
        ROWS,
        f32::from_bits(LAST_WIDTH_BITS.load(Ordering::Relaxed)),
        f32::from_bits(LAST_HEIGHT_BITS.load(Ordering::Relaxed)),
        refresh.map_or("n/d".to_string(), |d| format!(
            "{:.2}",
            d.as_secs_f64() * 1000.0
        )),
        mean_us(PREPAINT_NS.load(Ordering::Relaxed), frames),
        PREPAINT_MAX_NS.load(Ordering::Relaxed) as f64 / 1000.0,
        mean_us(PAINT_NS.load(Ordering::Relaxed), frames),
        PAINT_MAX_NS.load(Ordering::Relaxed) as f64 / 1000.0,
    );
}

fn main() {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("dokumentz-app-gpui: {error}");
            eprintln!("uso: dokumentz-app-gpui [--exit-after-ms <n>] [--animate]");
            std::process::exit(2);
        }
    };
    ANIMATE.store(args.animate, Ordering::Relaxed);

    application().run(move |cx: &mut App| {
        text_input::bind_keys(cx);
        let quit_key = if cfg!(target_os = "macos") {
            "cmd-q"
        } else {
            "ctrl-q"
        };
        cx.bind_keys([KeyBinding::new(quit_key, Quit, None)]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        // En macOS la aplicación sigue viva sin ventanas; aquí cerrar la ventana termina el proceso.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1200.0), px(820.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| {
                    let text_input = cx
                        .new(|cx| TextInput::new("Escribe aqui (barra de formulas del spike)", cx));
                    cx.new(|cx| SpikeView {
                        text_input,
                        recent_keystrokes: vec![],
                        focus_handle: cx.focus_handle(),
                    })
                },
            )
            .expect("no se pudo abrir la ventana");

        let view = window
            .update(cx, |_, _, cx| cx.entity())
            .expect("la ventana recien abierta debe existir");
        cx.observe_keystrokes(move |ev, _, cx| {
            view.update(cx, |view, cx| {
                view.recent_keystrokes.push(ev.keystroke.clone());
                cx.notify();
            })
        })
        .detach();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.text_input.focus_handle(cx), cx);
                cx.activate(true);
            })
            .expect("la ventana recien abierta debe existir");

        if let Some(ms) = args.exit_after_ms {
            let handle = window;
            cx.spawn(async move |cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(ms))
                    .await;
                let (refresh, scale) = cx.update(|cx| {
                    handle
                        .update(cx, |_, window, cx| {
                            (window.refresh_interval(cx), window.scale_factor())
                        })
                        .unwrap_or((None, 1.0))
                });
                print_stats(Some(ms), refresh, scale);
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sin_argumentos_no_cierra_ni_anima() {
        assert_eq!(parse_args(a(&[])), Ok(Args::default()));
    }

    #[test]
    fn lee_exit_after_ms_y_animate() {
        assert_eq!(
            parse_args(a(&["--exit-after-ms", "1500", "--animate"])),
            Ok(Args {
                exit_after_ms: Some(1500),
                animate: true
            })
        );
    }

    #[test]
    fn rechaza_valores_invalidos_o_desconocidos() {
        assert!(parse_args(a(&["--exit-after-ms"])).is_err());
        assert!(parse_args(a(&["--exit-after-ms", "abc"])).is_err());
        assert!(parse_args(a(&["--otra"])).is_err());
    }

    #[test]
    fn el_numero_de_celdas_y_lineas_es_el_del_encargo() {
        assert_eq!(EXPECTED_QUADS, 2000);
        assert_eq!(EXPECTED_LINES, 500);
    }
}
