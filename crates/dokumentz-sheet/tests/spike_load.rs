//! Spike de carga de IronCalc 0.9.1 con 100 000 x 20 celdas (tarea T005, compuerta CG-1).
//!
//! Mide `load_from_xlsx` + `UserModel::from_model`, la memoria del propio
//! proceso tras cargar, `save_to_xlsx` y la reapertura del archivo guardado.
//!
//! Para que la memoria sea la del modelo y no la del generador del `.xlsx`, cada
//! medición corre en un PROCESO HIJO nuevo: el test padre genera el archivo y
//! relanza este mismo binario de pruebas 3 veces (`spike_load_hijo`); el hijo
//! imprime una línea `SPIKE_HIJO_JSON {…}` y el padre agrega las medianas, imprime
//! el JSON final y, en `--release`, lo guarda en `results/dev/T005.json`.
//!
//! Se ejecuta con (siempre en release, nunca en debug):
//! `cargo test -p dokumentz-sheet --release --test spike_load -- --ignored --nocapture`

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ironcalc::export::save_to_xlsx;
use ironcalc::import::load_from_xlsx;
use ironcalc_base::UserModel;
use rust_xlsxwriter::{Color, ExcelDateTime, Format, Workbook};
use serde_json::{Value, json};

/// Variable de entorno que activa el modo hijo y lleva la ruta del `.xlsx`.
const ENV_XLSX: &str = "DOKUMENTZ_SPIKE_XLSX";
const FILAS: u32 = 100_000;
const COLUMNAS: u16 = 20;
/// Umbrales de aviso de CG-1 (`tasks.md` T005).
const UMBRAL_CARGA_MS: f64 = 10_000.0;
const UMBRAL_MEMORIA_MB: f64 = 2_048.0;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn redondear(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn mediana(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// RSS del propio proceso en MiB, con `ps -o rss= -p <pid>` (KiB en macOS y Linux).
fn rss_mb() -> Option<f64> {
    let pid = std::process::id().to_string();
    let salida = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    let kib: f64 = String::from_utf8_lossy(&salida.stdout)
        .trim()
        .parse()
        .ok()?;
    Some(kib / 1024.0)
}

/// `footprint` de macOS (memoria física facturable del proceso) en MiB, sin sudo:
/// `footprint -p <pid> -j <archivo>` escribe un JSON con `processes[0].footprint` en bytes.
fn footprint_mb() -> Option<f64> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let dir = tempfile::tempdir().ok()?;
    let ruta = dir.path().join("footprint.json");
    let pid = std::process::id().to_string();
    let estado = Command::new("footprint")
        .args(["-p", &pid, "-j"])
        .arg(&ruta)
        .output()
        .ok()?;
    if !estado.status.success() {
        return None;
    }
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&ruta).ok()?).ok()?;
    let bytes = v["processes"][0]["footprint"].as_f64()?;
    Some(bytes / (1024.0 * 1024.0))
}

/// 100 000 x 20: números, textos, fechas y estilos (fila 0 = encabezados en negrita).
fn generar_xlsx(ruta: &Path) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let negrita = Format::new().set_bold();
    let fecha = Format::new().set_num_format("yyyy-mm-dd");
    let moneda = Format::new().set_num_format("#,##0.00");
    let moneda_fondo = Format::new()
        .set_num_format("#,##0.00")
        .set_background_color(Color::RGB(0xFFEEAA))
        .set_font_color(Color::RGB(0x7F0000));
    let dos_dec = Format::new().set_num_format("0.00");
    for c in 0..COLUMNAS {
        ws.write_string_with_format(0, c, format!("Col{c}"), &negrita)
            .unwrap();
    }
    for r in 1..FILAS {
        for c in 0..COLUMNAS {
            match c % 5 {
                0 => {
                    ws.write_number(r, c, f64::from((r * 7 + u32::from(c)) % 100_000))
                        .unwrap();
                }
                1 => {
                    ws.write_number_with_format(r, c, f64::from(r) * 0.37 + f64::from(c), &dos_dec)
                        .unwrap();
                }
                2 => {
                    ws.write_string(r, c, format!("fila {r} col {c}")).unwrap();
                }
                3 => {
                    let d = ExcelDateTime::from_ymd(
                        2020 + (r % 5) as u16,
                        1 + (r % 12) as u8,
                        1 + (r % 28) as u8,
                    )
                    .unwrap();
                    ws.write_datetime_with_format(r, c, &d, &fecha).unwrap();
                }
                _ => {
                    let f = if r % 10 == 0 { &moneda_fondo } else { &moneda };
                    ws.write_number_with_format(r, c, f64::from(r) * 1.5, f)
                        .unwrap();
                }
            }
        }
    }
    wb.save(ruta).expect("guardar xlsx sintético");
}

/// Muestra de celdas (texto formateado) para comprobar que guardar y reabrir no pierde datos.
fn muestra(um: &UserModel) -> Vec<String> {
    let filas = [1, 2, 3, 10, 500, 49_999, 50_000, 99_999, 100_000];
    let mut v = Vec::new();
    for f in filas {
        for c in 1..=i32::from(COLUMNAS) {
            v.push(um.get_formatted_cell_value(0, f, c).unwrap_or_default());
        }
    }
    v
}

/// Modo hijo: una medición completa en un proceso limpio. Solo actúa si el padre
/// puso `DOKUMENTZ_SPIKE_XLSX`; sin esa variable (p. ej. al correr `--ignored` a
/// secas, donde también se ejecuta) no hace nada.
#[test]
#[ignore = "lo lanza spike_load como proceso hijo"]
fn spike_load_hijo() {
    let Ok(entrada) = std::env::var(ENV_XLSX) else {
        return;
    };
    let entrada = PathBuf::from(entrada);
    let rss_antes = rss_mb();
    let fp_antes = footprint_mb();

    // Muestreador del pico de RSS mientras se carga (aproximado: cada 25 ms, con `ps`).
    let parar = Arc::new(AtomicBool::new(false));
    let pico_kib = Arc::new(AtomicU64::new(0));
    let hilo = {
        let (parar, pico_kib) = (Arc::clone(&parar), Arc::clone(&pico_kib));
        std::thread::spawn(move || {
            while !parar.load(Ordering::Relaxed) {
                if let Some(mb) = rss_mb() {
                    pico_kib.fetch_max((mb * 1024.0) as u64, Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        })
    };

    let t = Instant::now();
    let modelo = load_from_xlsx(entrada.to_str().unwrap(), "en", "UTC", "en").expect("cargar");
    let importar_ms = ms(t);
    let t = Instant::now();
    let um = UserModel::from_model(modelo);
    let from_model_ms = ms(t);
    let load_ms = importar_ms + from_model_ms;
    // El muestreador se detiene aquí: su máximo es el pico DURANTE la carga.
    parar.store(true, Ordering::Relaxed);
    hilo.join().unwrap();
    let rss_despues = rss_mb();
    let fp_despues = footprint_mb();

    let dim = um.get_model().workbook.worksheets[0].dimension();
    assert_eq!(
        (dim.max_row, dim.max_column),
        (FILAS as i32, i32::from(COLUMNAS))
    );
    let antes = muestra(&um);

    // Guardar (IronCalc no sobrescribe: se escribe a una ruta nueva).
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("guardado.xlsx");
    let t = Instant::now();
    save_to_xlsx(um.get_model(), salida.to_str().unwrap()).expect("guardar");
    let save_ms = ms(t);

    // Reabrir lo guardado.
    let t = Instant::now();
    let modelo2 = load_from_xlsx(salida.to_str().unwrap(), "en", "UTC", "en").expect("reabrir");
    let um2 = UserModel::from_model(modelo2);
    let reload_ms = ms(t);
    let rss_tras_recarga = rss_mb();
    let despues = muestra(&um2);
    let dim2 = um2.get_model().workbook.worksheets[0].dimension();

    let resultado = json!({
        "importar_ms": redondear(importar_ms),
        "from_model_ms": redondear(from_model_ms),
        "load_ms": redondear(load_ms),
        "rss_mb_antes": rss_antes.map(redondear),
        "rss_mb": rss_despues.map(redondear),
        "rss_mb_pico_durante_la_carga": redondear(pico_kib.load(Ordering::Relaxed) as f64 / 1024.0),
        "footprint_mb_antes": fp_antes.map(redondear),
        "footprint_mb": fp_despues.map(redondear),
        "save_ms": redondear(save_ms),
        "reload_ms": redondear(reload_ms),
        "rss_mb_con_dos_modelos": rss_tras_recarga.map(redondear),
        "tamano_original_mb": redondear(std::fs::metadata(&entrada).unwrap().len() as f64 / 1_048_576.0),
        "tamano_guardado_mb": redondear(std::fs::metadata(&salida).unwrap().len() as f64 / 1_048_576.0),
        "ida_y_vuelta_igual": antes == despues,
        "dimension_tras_recargar": [dim2.max_row, dim2.max_column],
    });
    println!("SPIKE_HIJO_JSON {resultado}");
    assert_eq!(antes, despues, "guardar y reabrir cambió valores visibles");
}

/// Padre: genera el `.xlsx`, lanza 3 hijos, agrega medianas y aplica CG-1.
#[test]
#[ignore = "pesada: genera y carga 100 000 x 20; correr solo en --release"]
fn spike_load() {
    if cfg!(debug_assertions) {
        println!("spike_load: no se mide con `debug`; correr con --release");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let xlsx = dir.path().join("100k_x_20.xlsx");
    let t = Instant::now();
    generar_xlsx(&xlsx);
    let generar_ms = ms(t);

    let exe = std::env::current_exe().unwrap();
    let mut corridas: Vec<Value> = Vec::new();
    for _ in 0..3 {
        let salida = Command::new(&exe)
            .args([
                "--ignored",
                "--exact",
                "spike_load_hijo",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ENV_XLSX, &xlsx)
            .output()
            .expect("lanzar el proceso hijo");
        let texto = String::from_utf8_lossy(&salida.stdout);
        assert!(
            salida.status.success(),
            "el hijo falló:\n{texto}\n{}",
            String::from_utf8_lossy(&salida.stderr)
        );
        let linea = texto
            .lines()
            // El arnés de pruebas imprime «test nombre ... » sin salto de línea antes.
            .find_map(|l| l.split_once("SPIKE_HIJO_JSON ").map(|(_, json)| json))
            .unwrap_or_else(|| panic!("el hijo no imprimió su JSON; salida:\n{texto}"));
        corridas.push(serde_json::from_str(linea).unwrap());
    }

    let med = |clave: &str| -> Option<f64> {
        let v: Vec<f64> = corridas.iter().filter_map(|c| c[clave].as_f64()).collect();
        (v.len() == corridas.len()).then(|| redondear(mediana(v)))
    };
    let load_ms = med("load_ms").unwrap();
    let rss_mb = med("rss_mb");
    let footprint_mb = med("footprint_mb");
    // La memoria que se compara con el umbral es la mayor de las dos medidas.
    let memoria_mb = rss_mb
        .into_iter()
        .chain(footprint_mb)
        .fold(0.0_f64, f64::max);
    let supera = load_ms > UMBRAL_CARGA_MS || memoria_mb > UMBRAL_MEMORIA_MB;

    let resultado = json!({
        "tarea": "T005",
        "ironcalc": "0.9.1",
        "libro": format!("{FILAS} filas x {COLUMNAS} columnas = {} celdas (números, textos, fechas y estilos)", u64::from(FILAS) * u64::from(COLUMNAS)),
        "corridas": corridas.len(),
        "mediana_de_3_procesos_nuevos": true,
        "generar_xlsx_ms": redondear(generar_ms),
        "load_ms": load_ms,
        "importar_ms": med("importar_ms"),
        "from_model_ms": med("from_model_ms"),
        "rss_mb_antes": med("rss_mb_antes"),
        "rss_mb": rss_mb,
        "rss_mb_pico_durante_la_carga": med("rss_mb_pico_durante_la_carga"),
        "footprint_mb_antes": med("footprint_mb_antes"),
        "footprint_mb": footprint_mb,
        "save_ms": med("save_ms"),
        "reload_ms": med("reload_ms"),
        "rss_mb_con_dos_modelos": med("rss_mb_con_dos_modelos"),
        "tamano_original_mb": med("tamano_original_mb"),
        "tamano_guardado_mb": med("tamano_guardado_mb"),
        "ida_y_vuelta_igual": corridas.iter().all(|c| c["ida_y_vuelta_igual"] == json!(true)),
        "cg1": {
            "umbral_carga_ms": UMBRAL_CARGA_MS,
            "umbral_memoria_mb": UMBRAL_MEMORIA_MB,
            "supera": supera,
        },
        "detalle_de_las_corridas": corridas,
    });
    let texto = serde_json::to_string_pretty(&resultado).unwrap();
    println!("{texto}");
    if supera {
        println!(
            "CG-1: SUPERADA (carga {load_ms} ms, memoria {memoria_mb} MB): avisar al dueño antes de seguir"
        );
    } else {
        println!(
            "CG-1: dentro de los umbrales (carga {load_ms} ms < 10000, memoria {memoria_mb} MB < 2048)"
        );
    }

    let destino = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/dev");
    std::fs::create_dir_all(&destino).unwrap();
    std::fs::write(destino.join("T005.json"), texto + "\n").unwrap();
}
