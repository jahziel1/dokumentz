// En Windows, una compilación release no abre una consola extra.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dokumentz_app_tauri_lib::run();
}
