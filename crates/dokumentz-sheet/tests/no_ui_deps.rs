//! Principio III de la constitución: el núcleo de la hoja no puede depender, ni directa ni
//! transitivamente, de ninguna biblioteca de interfaz (T004).
//!
//! La prueba ejecuta `cargo metadata --format-version 1` sobre el espacio de trabajo y recorre
//! el grafo resuelto desde `dokumentz-sheet` siguiendo solo dependencias normales y de
//! compilación (`build`); las de desarrollo no entran en el producto. No filtra por plataforma:
//! una dependencia de interfaz que solo aparezca en Windows o Linux también hace fallar la prueba.

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

use serde_json::Value;

/// Paquete cuyo grafo se vigila.
const RAIZ: &str = "dokumentz-sheet";

/// Nombres exactos prohibidos.
const PROHIBIDOS: &[&str] = &[
    "tauri",
    "wry",
    "tao",
    "gpui",
    "gtk",
    "gtk4",
    "webkit2gtk",
    "winit",
    "objc2-app-kit",
    "cocoa",
];

/// Prefijos prohibidos (`tauri-*`, `gpui_*`; se admiten ambas grafías del separador).
const PREFIJOS_PROHIBIDOS: &[&str] = &["tauri-", "tauri_", "gpui_", "gpui-"];

fn es_prohibido(nombre: &str) -> bool {
    PROHIBIDOS.contains(&nombre) || PREFIJOS_PROHIBIDOS.iter().any(|p| nombre.starts_with(p))
}

/// Ejecuta `cargo metadata` sobre el `Cargo.toml` del espacio de trabajo.
fn metadata() -> Value {
    let manifiesto = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
    let salida = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            manifiesto,
        ])
        .output()
        .expect("no se pudo ejecutar `cargo metadata`");
    assert!(
        salida.status.success(),
        "`cargo metadata` falló:\n{}",
        String::from_utf8_lossy(&salida.stderr)
    );
    serde_json::from_slice(&salida.stdout).expect("la salida de `cargo metadata` no es JSON")
}

/// Cadena de dependencias (de `RAIZ` hacia la primera prohibida) o `None` si el grafo está limpio.
fn buscar_prohibida(meta: &Value) -> Option<Vec<String>> {
    // id de paquete -> nombre
    let nombres: HashMap<&str, &str> = meta["packages"]
        .as_array()
        .expect("`packages` no es una lista")
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    // id de paquete -> ids de sus dependencias normales o de compilación
    let nodos: HashMap<&str, Vec<&str>> = meta["resolve"]["nodes"]
        .as_array()
        .expect("`resolve.nodes` no es una lista")
        .iter()
        .map(|n| {
            let deps = n["deps"]
                .as_array()
                .expect("`deps` no es una lista")
                .iter()
                .filter(|d| {
                    // `kind` es null (normal), "build" o "dev"; basta una entrada no `dev`.
                    d["dep_kinds"]
                        .as_array()
                        .expect("`dep_kinds` no es una lista")
                        .iter()
                        .any(|k| k["kind"].as_str() != Some("dev"))
                })
                .map(|d| d["pkg"].as_str().unwrap())
                .collect();
            (n["id"].as_str().unwrap(), deps)
        })
        .collect();

    let raiz = nombres
        .iter()
        .find(|(_, nombre)| **nombre == RAIZ)
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("`{RAIZ}` no aparece en el espacio de trabajo"));

    // Búsqueda en anchura guardando el padre, para poder mostrar la cadena completa.
    let mut padre: HashMap<&str, &str> = HashMap::new();
    let mut vistos: HashSet<&str> = HashSet::from([raiz]);
    let mut cola: VecDeque<&str> = VecDeque::from([raiz]);
    while let Some(actual) = cola.pop_front() {
        if actual != raiz && es_prohibido(nombres[actual]) {
            let mut cadena = vec![nombres[actual].to_string()];
            let mut cursor = actual;
            while let Some(&p) = padre.get(cursor) {
                cadena.push(nombres[p].to_string());
                cursor = p;
            }
            cadena.reverse();
            return Some(cadena);
        }
        for &dep in nodos.get(actual).map(Vec::as_slice).unwrap_or_default() {
            if vistos.insert(dep) {
                padre.insert(dep, actual);
                cola.push_back(dep);
            }
        }
    }
    None
}

#[test]
fn el_nucleo_no_depende_de_bibliotecas_de_interfaz() {
    let meta = metadata();
    if let Some(cadena) = buscar_prohibida(&meta) {
        panic!(
            "`{RAIZ}` alcanza una dependencia de interfaz (principio III): {}",
            cadena.join(" -> ")
        );
    }
}

#[test]
fn la_lista_de_prohibidos_reconoce_los_nombres_esperados() {
    for nombre in [
        "tauri",
        "tauri-runtime",
        "tauri-plugin-dialog",
        "wry",
        "tao",
        "gpui",
        "gpui_platform",
        "gtk",
        "gtk4",
        "webkit2gtk",
        "winit",
        "objc2-app-kit",
        "cocoa",
    ] {
        assert!(es_prohibido(nombre), "{nombre} debería estar prohibido");
    }
    for nombre in [
        "serde",
        "serde_json",
        "ironcalc",
        "ironcalc_base",
        "taos",
        "windows",
    ] {
        assert!(!es_prohibido(nombre), "{nombre} no debería estar prohibido");
    }
}
