//! Comportamiento del `Recorder` encendido: bandera, marcas, tramos, series, contadores y
//! escritura del JSON (T012). El caso «apagado» está en `off_and_no_net.rs`.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use dokumentz_metrics::results::{App, Method, RunInfo};
use dokumentz_metrics::{FLAG, Recorder, names};

fn args(lista: &[&str]) -> Vec<OsString> {
    lista.iter().map(OsString::from).collect()
}

fn ruta_de(r: &Recorder) -> Option<PathBuf> {
    r.out_path().map(PathBuf::from)
}

#[test]
fn recorder_es_send_y_sync() {
    fn exige<T: Send + Sync>() {}
    exige::<Recorder>();
}

#[test]
fn bandera_con_ruta_en_dos_formas() {
    let r = Recorder::parse(args(&["--open", "a.xlsx", FLAG, "res/m.json"]), None).unwrap();
    assert!(r.is_on());
    assert_eq!(ruta_de(&r), Some(PathBuf::from("res/m.json")));

    let r = Recorder::parse(args(&["--metrics-out=otra/m.json"]), None).unwrap();
    assert_eq!(ruta_de(&r), Some(PathBuf::from("otra/m.json")));
}

#[test]
fn la_bandera_manda_sobre_la_variable_y_gana_la_ultima() {
    let env = Some(OsString::from("de-entorno.json"));
    let r = Recorder::parse(args(&[FLAG, "uno.json", FLAG, "dos.json"]), env.clone()).unwrap();
    assert_eq!(ruta_de(&r), Some(PathBuf::from("dos.json")));

    // Sin bandera, la variable de entorno enciende.
    let r = Recorder::parse(args(&["--open", "a.xlsx"]), env).unwrap();
    assert_eq!(ruta_de(&r), Some(PathBuf::from("de-entorno.json")));
}

#[test]
fn sin_bandera_ni_variable_esta_apagado() {
    let r = Recorder::parse(args(&["--open", "a.xlsx"]), None).unwrap();
    assert!(!r.is_on());
    // Una variable vacía cuenta como no definida.
    let r = Recorder::parse(args(&[]), Some(OsString::new())).unwrap();
    assert!(!r.is_on());
}

#[test]
fn se_detiene_en_doble_guion() {
    let r = Recorder::parse(args(&["--", FLAG, "x.json"]), None).unwrap();
    assert!(!r.is_on());
}

#[test]
fn bandera_sin_ruta_es_error() {
    assert!(Recorder::parse(args(&[FLAG]), None).is_err());
    assert!(Recorder::parse(args(&[FLAG, ""]), None).is_err());
    assert!(Recorder::parse(args(&[FLAG, "--script", "g.json"]), None).is_err());
    assert!(Recorder::parse(args(&["--metrics-out="]), None).is_err());
}

#[test]
fn from_args_env_sin_entorno_ni_bandera_esta_apagado() {
    // El arnés de pruebas no pasa `--metrics-out`; si el desarrollador exportó la variable,
    // no hay nada que afirmar.
    if std::env::var_os(dokumentz_metrics::ENV_VAR).is_none() {
        assert!(!Recorder::from_args_env().is_on());
    }
}

#[test]
fn marcas_en_orden_y_con_instante_explicito() {
    let r = Recorder::to_path("no-se-escribe.json");
    r.mark_at(names::LAUNCH, 0.0);
    r.mark_at(names::WINDOW_READY, 120.4567);
    r.mark_at("ignorada", f64::NAN);
    r.mark(names::OPEN_REQUEST);
    let doc = r.report().unwrap();
    let marcas = doc.internal.unwrap().marks;
    let nombres: Vec<_> = marcas.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(nombres, ["launch", "window_ready", "open_request"]);
    assert_eq!(marcas[1].t_ms, 120.457);
    assert!(marcas[2].t_ms >= 0.0);
}

#[test]
fn span_registra_al_soltar_y_finish_devuelve_los_ms() {
    let r = Recorder::to_path("no-se-escribe.json");
    {
        let _s = r.span("a_ms");
    }
    let ms = r.span("a_ms").finish().expect("encendido");
    assert!(ms >= 0.0);
    r.span("cancelado_ms").cancel();
    let hs = r.report().unwrap().internal.unwrap().histograms;
    assert_eq!(hs["a_ms"].count, 2);
    assert!(!hs.contains_key("cancelado_ms"));
}

#[test]
fn series_y_contadores() {
    let r = Recorder::to_path("no-se-escribe.json");
    for v in [10.0, 20.0, 30.0, 40.0] {
        r.sample(names::CELLS_DRAWN_PER_FRAME, v);
    }
    r.sample(names::CELLS_DRAWN_PER_FRAME, f64::NAN); // se ignora
    r.count(names::PLACEHOLDER_FRAMES, 2);
    r.count(names::PLACEHOLDER_FRAMES, 3);
    r.count("satura", u64::MAX);
    r.count("satura", 5);
    let i = r.report().unwrap().internal.unwrap();
    let h = i.histograms[names::CELLS_DRAWN_PER_FRAME];
    assert_eq!(
        (h.count, h.min, h.max, h.p50, h.p95),
        (4, 10.0, 40.0, 20.0, 40.0)
    );
    assert_eq!(i.counters[names::PLACEHOLDER_FRAMES], 5);
    assert_eq!(i.counters["satura"], u64::MAX);
    assert_eq!(i.percentile, "nearest-rank");
}

#[test]
fn se_puede_compartir_entre_hilos() {
    let r = Arc::new(Recorder::to_path("no-se-escribe.json"));
    let hilos: Vec<_> = (0..4)
        .map(|_| {
            let r = Arc::clone(&r);
            std::thread::spawn(move || {
                for i in 0..250 {
                    r.sample("x", f64::from(i));
                    r.count("n", 1);
                }
            })
        })
        .collect();
    for h in hilos {
        h.join().unwrap();
    }
    let i = r.report().unwrap().internal.unwrap();
    assert_eq!(i.histograms["x"].count, 1000);
    assert_eq!(i.counters["n"], 1000);
}

fn info(build: &str, primary: Option<&str>) -> RunInfo {
    let mut app = App::new("dokumentz-app-gpui", "0.1.0");
    app.build = build.to_string();
    RunInfo {
        app,
        primary: primary.map(String::from),
        ..RunInfo::default()
    }
}

#[test]
fn la_serie_principal_llena_reps_y_resumen() {
    let r = Recorder::to_path("res/T999.json");
    r.set_run(info("release", Some(names::FRAME_INTERVAL_MS)));
    for v in 1..=100 {
        r.sample(names::FRAME_INTERVAL_MS, f64::from(v));
    }
    let doc = r.report().unwrap();
    assert_eq!(doc.method, Method::Internal);
    assert_eq!(doc.criteria_sha, None);
    assert_eq!(
        doc.run_id, "T999",
        "sin run_id se usa el nombre del archivo"
    );
    assert!(doc.invalidated.is_empty());
    let rep = &doc.reps[0];
    assert_eq!((rep.n, rep.ok, rep.samples), (1, true, Some(100)));
    assert_eq!(
        (rep.p50, rep.p95, rep.p99),
        (Some(50.0), Some(95.0), Some(99.0))
    );
    assert_eq!((doc.summary.min, doc.summary.max), (Some(1.0), Some(100.0)));
}

#[test]
fn serie_principal_sin_muestras_y_build_debug_invalidan() {
    let r = Recorder::to_path("res/T998.json");
    r.set_run(info("debug", Some("no_existe_ms")));
    let doc = r.report().unwrap();
    assert!(doc.reps.is_empty());
    assert_eq!(
        doc.invalidated,
        ["debug_build", "sin_muestras:no_existe_ms"]
    );
}

#[test]
fn build_vacio_se_rellena_con_el_perfil_de_compilacion() {
    let r = Recorder::to_path("res/T997.json");
    r.set_run(info("", None));
    let doc = r.report().unwrap();
    let esperado = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    assert_eq!(doc.app.build, esperado);
    assert_eq!(doc.invalidated.is_empty(), !cfg!(debug_assertions));
}

#[test]
fn write_json_crea_carpetas_escribe_un_solo_archivo_y_se_puede_repetir() {
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("results").join("dev").join("m.json");
    let r = Recorder::to_path(&salida);
    r.set_run(info("release", None));
    r.count("c", 1);

    assert_eq!(r.write_json().unwrap(), Some(salida.clone()));
    r.count("c", 1);
    assert_eq!(
        r.write_json().unwrap(),
        Some(salida.clone()),
        "se puede repetir"
    );

    let carpeta = salida.parent().unwrap();
    let nombres: Vec<_> = std::fs::read_dir(carpeta)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(nombres, ["m.json"], "no queda ningún temporal");

    let texto = std::fs::read_to_string(&salida).unwrap();
    assert!(texto.ends_with("}\n"));
    let doc = dokumentz_metrics::Results::from_json_str(&texto).unwrap();
    assert_eq!(doc.internal.unwrap().counters["c"], 2);
}

#[test]
fn write_json_con_ruta_imposible_devuelve_error_sin_panico() {
    let dir = tempfile::tempdir().unwrap();
    // La «carpeta» es un archivo: no se puede crear nada debajo.
    let archivo = dir.path().join("archivo");
    std::fs::write(&archivo, b"x").unwrap();
    let r = Recorder::to_path(archivo.join("m.json"));
    let e = r.write_json().unwrap_err();
    assert!(e.to_string().contains("no se pudo escribir"), "{e}");
}
