//! Genera el contexto de Tauri y restringe el IPC a los comandos del spike.

/// Comandos de la aplicación. Cada uno obtiene un permiso `allow-<nombre-con-guiones>`
/// que `capabilities/default.json` concede de forma explícita.
const COMANDOS: &[&str] = &["spike_args", "ipc_echo", "ipc_channel_push", "spike_report"];

fn main() {
    let manifiesto = tauri_build::AppManifest::new().commands(COMANDOS);
    let atributos = tauri_build::Attributes::new().app_manifest(manifiesto);
    tauri_build::try_build(atributos).expect("fallo al preparar el contexto de Tauri");
}
