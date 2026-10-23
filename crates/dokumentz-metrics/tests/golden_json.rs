//! El JSON del `Recorder` es estable: se compara byte a byte con `tests/golden/results_v1.json`
//! (T012). Todas las entradas son explícitas (marcas con instante dado, muestras fijas): nada
//! depende del reloj ni del perfil de compilación.
//!
//! Para regenerar el dorado tras un cambio **intencionado** del esquema:
//! `UPDATE_GOLDEN=1 cargo test -p dokumentz-metrics --test golden_json` y revisar el diff.

use dokumentz_metrics::results::{
    App, DisplayInfo, Env, FileInfo, Method, Results, RunInfo, SCHEMA,
};
use dokumentz_metrics::{Recorder, names};

const DORADO: &str = "tests/golden/results_v1.json";

fn documento() -> Results {
    let r = Recorder::to_path("results/dev/golden.json");
    let mut env = Env::from_std();
    env.os = "macOS 26.6.2".to_string();
    env.arch = "arm64".to_string();
    env.cpu = Some("Apple M1".to_string());
    env.ram_gb = Some(16.0);
    env.gpu = Some("Apple M1".to_string());
    env.display = Some(DisplayInfo {
        width_px: 2560,
        height_px: 1600,
        scale: 2.0,
        refresh_hz: Some(60.0),
    });
    env.vm = Some(false);
    env.power = Some("ac".to_string());
    env.session = Some("aqua".to_string());
    let mut app = App::new("dokumentz-app-gpui", "0.1.0");
    app.build = "release".to_string();
    app.lang = Some("es".to_string());
    app.commit = Some("090bdfb".to_string());
    r.set_run(RunInfo {
        run_id: "2026-11-03T10-12-00Z-macos-m1".to_string(),
        env,
        app,
        file: Some(FileInfo {
            id: "A1".to_string(),
            sha256: None,
            rows: Some(100_000),
            cols: Some(20),
        }),
        measure: "M-03".to_string(),
        scenario: "S1-wheel-10s".to_string(),
        cold: false,
        primary: Some(names::FRAME_INTERVAL_MS.to_string()),
    });

    r.mark_at(names::LAUNCH, 0.0);
    r.mark_at(names::WINDOW_READY, 85.25);
    r.mark_at(names::OPEN_REQUEST, 90.0);
    r.mark_at(names::FIRST_DATA_FRAME, 412.3456);

    // 98 cuadros de 16,7 ms y dos lentos: p50 = p95 = 16,7 ; p99 = 33,4 ; máx. = 50,1.
    for _ in 0..98 {
        r.sample(names::FRAME_INTERVAL_MS, 16.7);
    }
    r.sample(names::FRAME_INTERVAL_MS, 33.4);
    r.sample(names::FRAME_INTERVAL_MS, 50.1);
    for v in [1.5, 2.5, 3.0, 1.0] {
        r.sample(names::DRAW_BLOCK_MS, v);
    }
    for _ in 0..100 {
        r.sample(names::CELLS_DRAWN_PER_FRAME, 2800.0);
    }
    r.count(names::CELLS_IN_LIST, 8400);
    r.count(names::PLACEHOLDER_FRAMES, 0);
    r.report().expect("encendido")
}

#[test]
fn el_json_coincide_con_el_dorado() {
    let json = documento().to_json_string().unwrap();
    let ruta = format!("{}/{DORADO}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&ruta, &json).unwrap();
    }
    let dorado = std::fs::read_to_string(&ruta).expect("falta el archivo dorado");
    assert_eq!(
        json, dorado,
        "el JSON cambió; si es intencionado, regenere el dorado"
    );
}

#[test]
fn el_dorado_se_lee_y_vuelve_a_escribirse_igual() {
    let dorado = std::fs::read_to_string(format!("{}/{DORADO}", env!("CARGO_MANIFEST_DIR")))
        .expect("falta el archivo dorado");
    let leido = Results::from_json_str(&dorado).unwrap();
    assert_eq!(leido, documento());
    assert_eq!(leido.to_json_string().unwrap(), dorado);
    assert_eq!(leido.schema, SCHEMA);
    assert_eq!(leido.method, Method::Internal);
}

#[test]
fn el_orden_de_claves_es_el_del_esquema() {
    let json = documento().to_json_string().unwrap();
    let posiciones: Vec<_> = [
        "\"schema\"",
        "\"run_id\"",
        "\"criteria_sha\"",
        "\"method\"",
        "\"env\"",
        "\"app\"",
        "\"file\"",
        "\"measure\"",
        "\"scenario\"",
        "\"cold\"",
        "\"reps\"",
        "\"summary\"",
        "\"invalidated\"",
        "\"internal\"",
    ]
    .iter()
    .map(|k| {
        // Con los dos puntos, para no confundir la clave con un valor igual (`"internal"`).
        json.find(&format!("{k}:"))
            .unwrap_or_else(|| panic!("falta {k}"))
    })
    .collect();
    assert!(posiciones.windows(2).all(|w| w[0] < w[1]), "{posiciones:?}");
}

#[test]
fn rechaza_un_esquema_distinto() {
    let json = documento()
        .to_json_string()
        .unwrap()
        .replace(SCHEMA, "otro/9");
    assert!(Results::from_json_str(&json).is_err());
}
