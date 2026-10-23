//! Interfaz A (Tauri): spike del puente IPC (T009, compuerta CG-2) y del canvas 2D (T010).
//!
//! El binario acepta `--spike ipc|canvas|all` y `--exit`: con `--spike` el frontend corre la
//! medición al arrancar y entrega el JSON a [`spike_report`], que lo imprime en stdout;
//! con `--exit` el proceso se cierra solo al terminar (sin intervención).

use std::io::Write;
use std::time::Duration;

use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::{AppHandle, State};

/// Tamaño máximo que `ipc_echo` y `ipc_channel_push` aceptan (el spike usa hasta 512 KiB).
const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;

/// Con `--spike` y `--exit`, si el frontend no entrega el informe en este tiempo, se aborta.
const WATCHDOG: Duration = Duration::from_secs(240);

/// Códigos de salida del binario.
const EXIT_ARGS: i32 = 2;
const EXIT_WATCHDOG: i32 = 3;

/// Banderas de línea de órdenes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpikeArgs {
    /// `ipc`, `canvas` o `all`; `None` abre la ventana en modo manual.
    pub spike: Option<String>,
    /// Cerrar el proceso al terminar la medición.
    pub exit: bool,
}

/// Interpreta `--spike <valor>`, `--spike=<valor>` y `--exit`; ignora el resto
/// (macOS y Tauri pueden añadir argumentos propios).
pub fn parse_spike_args<I, S>(args: I) -> Result<SpikeArgs, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = SpikeArgs::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let arg = arg.as_ref();
        let value = if arg == "--spike" {
            Some(
                it.next()
                    .ok_or_else(|| "--spike necesita un valor: ipc, canvas o all".to_string())?
                    .as_ref()
                    .to_string(),
            )
        } else {
            arg.strip_prefix("--spike=").map(str::to_string)
        };
        if let Some(value) = value {
            if !matches!(value.as_str(), "ipc" | "canvas" | "all") {
                return Err(format!("--spike {value}: valores válidos ipc, canvas, all"));
            }
            out.spike = Some(value);
        } else if arg == "--exit" {
            out.exit = true;
        }
    }
    Ok(out)
}

/// Versión del paquete, para los informes.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Datos que el frontend necesita para arrancar y para etiquetar el informe.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpikeInfo {
    spike: Option<String>,
    exit: bool,
    app_version: &'static str,
    tauri_version: &'static str,
    os: &'static str,
    arch: &'static str,
}

/// `n` bytes con un patrón que el frontend comprueba (`i % 251`), para detectar truncados.
pub fn payload(n: usize) -> Vec<u8> {
    let n = n.min(MAX_PAYLOAD_BYTES);
    (0..n).map(|i| (i % 251) as u8).collect()
}

#[tauri::command]
async fn spike_args(args: State<'_, SpikeArgs>) -> Result<SpikeInfo, String> {
    Ok(SpikeInfo {
        spike: args.spike.clone(),
        exit: args.exit,
        app_version: version(),
        tauri_version: tauri::VERSION,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    })
}

/// Responde con `n` bytes sin pasar por JSON: en JavaScript llega un `ArrayBuffer`.
#[tauri::command]
async fn ipc_echo(n: usize) -> Response {
    Response::new(payload(n))
}

/// Empuja `n` bytes por el `Channel`; en JavaScript también llegan como `ArrayBuffer`.
/// (Un `Channel<Vec<u8>>` serializaría a JSON: aquí se usa `InvokeResponseBody::Raw`.)
#[tauri::command]
async fn ipc_channel_push(n: usize, on_chunk: Channel<InvokeResponseBody>) -> Result<(), String> {
    on_chunk
        .send(InvokeResponseBody::Raw(payload(n)))
        .map_err(|e| e.to_string())
}

/// Recibe el JSON de latencias del frontend y lo imprime en stdout (una línea).
/// Con `done` y `--exit` cierra el proceso (código 0 si `ok`, 1 si no).
#[tauri::command]
async fn spike_report(
    app: AppHandle,
    args: State<'_, SpikeArgs>,
    json: String,
    done: bool,
    ok: bool,
) -> Result<(), String> {
    // Se valida que sea JSON, pero se imprime tal cual llegó (compacto, orden de claves del frontend).
    serde_json::from_str::<serde_json::Value>(&json).map_err(|e| format!("JSON inválido: {e}"))?;
    let mut out = std::io::stdout().lock();
    writeln!(out, "{json}").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    if done && args.exit {
        app.exit(if ok { 0 } else { 1 });
    }
    Ok(())
}

pub fn run() {
    let args = match parse_spike_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(msg) => {
            eprintln!("dokumentz-app-tauri: {msg}");
            std::process::exit(EXIT_ARGS);
        }
    };
    let watchdog = args.exit && args.spike.is_some();

    tauri::Builder::default()
        .manage(args)
        .invoke_handler(tauri::generate_handler![
            spike_args,
            ipc_echo,
            ipc_channel_push,
            spike_report
        ])
        .setup(move |app| {
            if watchdog {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(WATCHDOG);
                    eprintln!("dokumentz-app-tauri: sin informe tras {WATCHDOG:?}, se aborta");
                    handle.exit(EXIT_WATCHDOG);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación Tauri");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_no_esta_vacia() {
        assert!(!version().is_empty());
    }

    #[test]
    fn args_vacios_son_modo_manual() {
        let a = parse_spike_args(Vec::<&str>::new()).unwrap();
        assert_eq!(a, SpikeArgs::default());
    }

    #[test]
    fn args_spike_y_exit() {
        let a = parse_spike_args(["--spike", "ipc", "--exit"]).unwrap();
        assert_eq!(a.spike.as_deref(), Some("ipc"));
        assert!(a.exit);
        let b = parse_spike_args(["--spike=canvas"]).unwrap();
        assert_eq!(b.spike.as_deref(), Some("canvas"));
        assert!(!b.exit);
    }

    #[test]
    fn args_desconocidos_se_ignoran() {
        let a = parse_spike_args(["-psn_0_1234", "--exit", "--otro"]).unwrap();
        assert_eq!(a.spike, None);
        assert!(a.exit);
    }

    #[test]
    fn args_invalidos_fallan() {
        assert!(parse_spike_args(["--spike"]).is_err());
        assert!(parse_spike_args(["--spike", "webgl"]).is_err());
        assert!(parse_spike_args(["--spike=nada"]).is_err());
    }

    #[test]
    fn payload_tiene_n_bytes_y_patron() {
        for n in [0usize, 1, 16 * 1024, 128 * 1024, 512 * 1024] {
            let p = payload(n);
            assert_eq!(p.len(), n);
            if let Some(&last) = p.last() {
                assert_eq!(usize::from(last), (n - 1) % 251);
            }
        }
    }

    #[test]
    fn payload_respeta_el_maximo() {
        assert_eq!(payload(usize::MAX).len(), MAX_PAYLOAD_BYTES);
    }
}
