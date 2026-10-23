//! Spike de API de IronCalc 0.9.1 (tarea T006, etapa E02 de la fase 0).
//!
//! Cada `#[test]` comprueba uno de los 12 hallazgos de `tasks.md` (T006) y
//! cada comprobación lleva un comentario `// HALLAZGO:` que dice qué se
//! comprobó. Los puntos extra (`x0N`) son hallazgos que salieron al probar y que
//! afectan al diseño de `plan.md` §5.
//!
//! Las mediciones de tiempo solo se registran en `--release` (nunca se mide con
//! `debug`); las pruebas pesadas (6) se ignoran en `debug`. En `--release` cada
//! prueba añade su resultado a `results/dev/T006.json`.
//!
//! Versión probada: `ironcalc` / `ironcalc_base` = 0.9.1 (fijada con `=` en el
//! `Cargo.toml` raíz). Si se actualiza, hay que volver a correr este archivo:
//! algunas aserciones documentan a propósito un comportamiento defectuoso.

use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use ironcalc::export::{save_to_xlsx, save_xlsx_to_writer};
use ironcalc::import::load_from_xlsx;
use ironcalc_base::UserModel;
use ironcalc_base::expressions::types::Area;
use ironcalc_base::types::{CellType, Color as IcColor};
use ironcalc_base::{COLUMN_WIDTH_FACTOR, ROW_HEIGHT_FACTOR};
use rust_xlsxwriter::{
    Chart, ChartType, Color, DataValidation, DataValidationRule, ExcelDateTime, Format, Formula,
    Image, Note, Workbook,
};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Utilidades comunes
// ---------------------------------------------------------------------------

/// Serializa las pruebas de tiempo para que no compitan entre sí por la CPU.
static PESADA: Mutex<()> = Mutex::new(());
/// Protege la lectura-modificación-escritura de `results/dev/T006.json`.
static ARCHIVO: Mutex<()> = Mutex::new(());

/// Imprime el resultado de un hallazgo y, en `--release`, lo guarda en
/// `results/dev/T006.json` (en `debug` no se guarda: no se mide con `debug`).
fn registrar(clave: &str, valor: Value) {
    println!("HALLAZGO {clave}: {valor}");
    if cfg!(debug_assertions) {
        return;
    }
    let _guardia = ARCHIVO.lock().unwrap_or_else(|e| e.into_inner());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/dev");
    std::fs::create_dir_all(&dir).expect("crear results/dev");
    let ruta = dir.join("T006.json");
    let mut raiz: serde_json::Map<String, Value> = std::fs::read_to_string(&ruta)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    raiz.insert(clave.to_string(), valor);
    raiz.insert(
        "_meta".to_string(),
        json!({
            "tarea": "T006",
            "ironcalc": "0.9.1",
            "perfil": "release",
            "maquina": "Apple M1, 16 GB, macOS (ver research.md)",
        }),
    );
    std::fs::write(&ruta, serde_json::to_string_pretty(&raiz).unwrap())
        .expect("escribir T006.json");
}

fn nuevo(locale: &'static str, idioma: &'static str) -> UserModel<'static> {
    UserModel::new_empty("spike", locale, "UTC", idioma).expect("UserModel::new_empty")
}

fn abrir(ruta: &Path, locale: &'static str, idioma: &'static str) -> UserModel<'static> {
    let modelo = load_from_xlsx(ruta.to_str().unwrap(), locale, "UTC", idioma).expect("cargar");
    UserModel::from_model(modelo)
}

fn valor(um: &UserModel, hoja: u32, fila: i32, col: i32) -> String {
    um.get_formatted_cell_value(hoja, fila, col)
        .expect("get_formatted_cell_value")
}

fn contenido(um: &UserModel, hoja: u32, fila: i32, col: i32) -> String {
    um.get_cell_content(hoja, fila, col)
        .expect("get_cell_content")
}

fn mediana(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn redondear(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Genera un `.xlsx` sintético: `filas` x `cols` con números, textos, fechas y
/// estilos (fila 0 = encabezados en negrita). En la columna `cols` (U si son 20)
/// fila 0 va la celda de entrada (=2) y en la columna `cols + 1` (V) las filas
/// 1..=`formulas` llevan `=$U$1*A{n}`, o sea `formulas` fórmulas que dependen
/// de esa única celda (el A2 sintético de `spec.md`).
fn generar_xlsx(ruta: &Path, filas: u32, cols: u16, formulas: u32) {
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
    for c in 0..cols {
        ws.write_string_with_format(0, c, format!("Col{c}"), &negrita)
            .unwrap();
    }
    for r in 1..filas {
        for c in 0..cols {
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
    if formulas > 0 {
        ws.write_number(0, cols, 2).unwrap();
        for r in 1..=formulas {
            ws.write_formula(
                r,
                cols + 1,
                Formula::new(format!("=$U$1*A{}", r + 1)).set_result("0"),
            )
            .unwrap();
        }
    }
    wb.save(ruta).expect("guardar xlsx sintético");
}

/// PNG de 1x1 píxel (transparente) para probar una imagen incrustada.
const PNG_1X1: [u8; 69] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
    0x00, 0x05, 0xFE, 0x02, 0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// Nombres de las entradas de un paquete OPC (`.xlsx`).
fn partes(bytes: &[u8]) -> Vec<String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
    (0..zip.len())
        .map(|i| {
            zip.by_index(i)
                .unwrap()
                .name()
                .expect("nombre de entrada")
                .to_string()
        })
        .collect()
}

/// Texto de una entrada del paquete (vacío si no existe).
fn parte_texto(bytes: &[u8], nombre: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
    let mut s = String::new();
    if let Ok(mut f) = zip.by_name(nombre) {
        f.read_to_string(&mut s).unwrap();
    }
    s
}

// ---------------------------------------------------------------------------
// (1) Filas y columnas base 1
// ---------------------------------------------------------------------------

#[test]
fn h01_filas_y_columnas_base_1() {
    let mut um = nuevo("en", "en");
    // HALLAZGO: la hoja es base 0 y la fila/columna es base 1: A1 es (0, 1, 1).
    um.set_user_input(0, 1, 1, "hola").unwrap();
    assert_eq!(contenido(&um, 0, 1, 1), "hola");
    // HALLAZGO: fila 0 y columna 0 no existen (error, no escritura en A1).
    assert!(um.set_user_input(0, 0, 1, "x").is_err());
    assert!(um.set_user_input(0, 1, 0, "x").is_err());
    // HALLAZGO: la celda activa inicial es (hoja 0, fila 1, columna 1).
    assert_eq!(um.get_selected_cell(), (0, 1, 1));
    // HALLAZGO: los límites son 1 048 576 filas x 16 384 columnas (inclusive).
    assert!(um.set_user_input(0, 1_048_576, 16_384, "fin").is_ok());
    assert!(um.set_user_input(0, 1_048_577, 1, "x").is_err());
    assert!(um.set_user_input(0, 1, 16_385, "x").is_err());
    // HALLAZGO: una hoja inexistente es error (una sola hoja al crear el libro).
    assert!(um.set_user_input(1, 1, 1, "x").is_err());
    // HALLAZGO: lo mismo al leer un .xlsx: lo escrito en la fila 0, columna 0
    // de rust_xlsxwriter (A1) queda en (0, 1, 1).
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("a1.xlsx");
    let mut wb = Workbook::new();
    wb.add_worksheet().write_string(0, 0, "en A1").unwrap();
    wb.save(&ruta).unwrap();
    let um2 = abrir(&ruta, "en", "en");
    assert_eq!(contenido(&um2, 0, 1, 1), "en A1");
    registrar(
        "h01_base_1",
        json!({"hoja_base": 0, "fila_columna_base": 1, "max_filas": 1_048_576, "max_columnas": 16_384}),
    );
}

// ---------------------------------------------------------------------------
// (2) Unidad y valor por defecto de get_row_height / get_column_width
// ---------------------------------------------------------------------------

#[test]
fn h02_unidad_y_valor_por_defecto_de_tamanos() {
    let mut um = nuevo("en", "en");
    // HALLAZGO: por defecto una fila mide 25,0 y una columna 90,0.
    assert_eq!(um.get_row_height(0, 1).unwrap(), 25.0);
    assert_eq!(um.get_column_width(0, 1).unwrap(), 90.0);
    // HALLAZGO: los setters y los getters usan la misma unidad (píxeles
    // lógicos de IronCalc): lo que se escribe es lo que se lee.
    um.set_columns_width(0, 2, 3, 120.0).unwrap();
    um.set_rows_height(0, 2, 2, 40.0).unwrap();
    assert_eq!(um.get_column_width(0, 2).unwrap(), 120.0);
    assert_eq!(um.get_column_width(0, 3).unwrap(), 120.0);
    assert_eq!(um.get_row_height(0, 2).unwrap(), 40.0);
    // HALLAZGO: una columna u fila oculta mide 0,0.
    um.set_columns_hidden(0, 5, 5, true).unwrap();
    um.set_rows_hidden(0, 5, 5, true).unwrap();
    assert_eq!(um.get_column_width(0, 5).unwrap(), 0.0);
    assert_eq!(um.get_row_height(0, 5).unwrap(), 0.0);

    // HALLAZGO: al leer un .xlsx la conversión es ancho(caracteres) x 9 y
    // alto(puntos) x 1,5625 (constantes públicas COLUMN_WIDTH_FACTOR y
    // ROW_HEIGHT_FACTOR). Un píxel de IronCalc NO es un píxel de Excel: la fila
    // por defecto mide 25 (Excel: 20 px a 96 ppp) y la columna por defecto 90
    // (Excel: 64 px). La geometría del núcleo es la de IronCalc, no la de Office.
    assert_eq!(COLUMN_WIDTH_FACTOR, 9.0);
    assert_eq!(ROW_HEIGHT_FACTOR, 1.5625);
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("tamanos.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(0, 0, "x").unwrap();
    ws.set_column_width(1, 20.0).unwrap();
    ws.set_row_height(5, 30.0).unwrap();
    wb.save(&ruta).unwrap();
    let um2 = abrir(&ruta, "en", "en");
    let hoja = &um2.get_model().workbook.worksheets[0];
    let ancho_xlsx = hoja.cols[0].width; // caracteres, tal como está en el archivo
    let alto_xlsx = hoja.rows[0].height; // puntos
    assert_eq!(alto_xlsx, 30.0);
    assert_eq!(
        um2.get_column_width(0, 2).unwrap(),
        ancho_xlsx * COLUMN_WIDTH_FACTOR
    );
    assert_eq!(um2.get_row_height(0, 6).unwrap(), 30.0 * ROW_HEIGHT_FACTOR);
    // HALLAZGO: lo no definido en el archivo cae al valor por defecto.
    assert_eq!(um2.get_column_width(0, 1).unwrap(), 90.0);
    assert_eq!(um2.get_row_height(0, 1).unwrap(), 25.0);
    registrar(
        "h02_tamanos",
        json!({
            "fila_defecto": 25.0, "columna_defecto": 90.0,
            "factor_ancho": COLUMN_WIDTH_FACTOR, "factor_alto": ROW_HEIGHT_FACTOR,
            "ancho_20_caracteres_px": um2.get_column_width(0, 2).unwrap(),
            "alto_30_puntos_px": um2.get_row_height(0, 6).unwrap(),
        }),
    );
}

// ---------------------------------------------------------------------------
// (3) Formato de `key` en on_expand_selected_range  y  (4) campos de SelectedView
// ---------------------------------------------------------------------------

#[test]
fn h03_key_de_on_expand_selected_range() {
    let mut um = nuevo("en", "en");
    um.set_selected_cell(2, 2).unwrap(); // B2
    // HALLAZGO: `key` es el valor `KeyboardEvent.key` del DOM: "ArrowRight",
    // "ArrowLeft", "ArrowUp", "ArrowDown". `range` es [fila_ini, col_ini,
    // fila_fin, col_fin] normalizado (ini <= fin) y la celda activa (ancla) no se mueve.
    um.on_expand_selected_range("ArrowRight").unwrap();
    assert_eq!(um.get_selected_view().range, [2, 2, 2, 3]);
    um.on_expand_selected_range("ArrowDown").unwrap();
    assert_eq!(um.get_selected_view().range, [2, 2, 3, 3]);
    um.on_expand_selected_range("ArrowLeft").unwrap();
    assert_eq!(um.get_selected_view().range, [2, 2, 3, 2]);
    um.on_expand_selected_range("ArrowUp").unwrap();
    assert_eq!(um.get_selected_view().range, [2, 2, 2, 2]);
    um.on_expand_selected_range("ArrowLeft").unwrap();
    assert_eq!(um.get_selected_view().range, [2, 1, 2, 2], "normalizado");
    assert_eq!(um.get_selected_cell(), (0, 2, 2), "el ancla no se mueve");
    // HALLAZGO: cualquier otra cadena (sin prefijo "Arrow", minúsculas, con
    // modificador, con espacios) se ignora SIN error: no hay forma de saber que
    // la tecla fue rechazada. El núcleo debe traducir `Dir` a estas cadenas exactas.
    let antes = um.get_selected_view().range;
    for mala in [
        "Right",
        "arrowright",
        "ArrowRight ",
        "Shift+ArrowRight",
        "",
        "PageDown",
    ] {
        assert!(um.on_expand_selected_range(mala).is_ok(), "{mala:?}");
        assert_eq!(
            um.get_selected_view().range,
            antes,
            "{mala:?} no debe mover nada"
        );
    }
    registrar(
        "h03_key_expand",
        json!({"validas": ["ArrowRight", "ArrowLeft", "ArrowUp", "ArrowDown"], "otras": "se ignoran sin error", "range": "[fila_ini, col_ini, fila_fin, col_fin] normalizado"}),
    );
}

#[test]
fn h04_campos_de_selected_view() {
    let mut um = nuevo("en", "en");
    um.set_selected_cell(3, 4).unwrap();
    um.on_expand_selected_range("ArrowDown").unwrap();
    um.set_top_left_visible_cell(10, 5).unwrap();
    let vista = um.get_selected_view();
    // HALLAZGO: `SelectedView` expone `sheet`, `row` y `column` (la celda activa
    // o ancla), `range` y `top_row`/`left_column` (la celda de arriba a la
    // izquierda visible), todos públicos. NO expone la celda «foco» (la esquina
    // que se mueve al extender): hay que deducirla de `range` y el ancla.
    let (sheet, row, column, range, top_row, left_column) = (
        vista.sheet,
        vista.row,
        vista.column,
        vista.range,
        vista.top_row,
        vista.left_column,
    );
    assert_eq!((sheet, row, column), (0, 3, 4));
    assert_eq!(range, [3, 4, 4, 4]);
    assert_eq!((top_row, left_column), (10, 5));
    // HALLAZGO: no tiene más campos que esos seis: el JSON de su serialización
    // (deriva Serialize/Deserialize) tiene exactamente esas claves. Fuera de las
    // pruebas internas de IronCalc NO es `Debug` ni `PartialEq` y NO se puede
    // nombrar: `pub use ui::SelectedView` está bajo `#[cfg(test)]` en
    // `user_model/mod.rs`. Se usa solo por inferencia (`let v = um.get_selected_view()`).
    let json_vista = serde_json::to_value(um.get_selected_view()).unwrap();
    let mut claves: Vec<&str> = json_vista
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    claves.sort_unstable();
    assert_eq!(
        claves,
        ["column", "left_column", "range", "row", "sheet", "top_row"]
    );
    registrar(
        "h04_selected_view",
        json!({"campos": claves, "nombrable": false, "foco_expuesto": false}),
    );
}

// ---------------------------------------------------------------------------
// (5) Idioma de fórmulas
// ---------------------------------------------------------------------------

#[test]
fn h05_idioma_de_formulas() {
    let mut um = nuevo("en", "en");
    um.set_user_input(0, 1, 1, "=TRUE()").unwrap();
    um.set_user_input(0, 2, 1, "=SUM(1,2)").unwrap();
    assert_eq!(valor(&um, 0, 1, 1), "TRUE");
    um.set_language("es").unwrap();
    assert_eq!(um.get_language(), "es");
    // HALLAZGO: las fórmulas se guardan en inglés y se muestran en el idioma activo
    // (get_cell_content cambia de texto sin tocar la celda).
    assert_eq!(contenido(&um, 0, 1, 1), "=VERDADERO()");
    assert_eq!(contenido(&um, 0, 2, 1), "=SUMA(1,2)");
    // HALLAZGO: con idioma `es`, `=TRUE()` da error de nombre `#¿NOMBRE?` (no `#NAME?`).
    um.set_user_input(0, 3, 1, "=TRUE()").unwrap();
    assert_eq!(valor(&um, 0, 3, 1), "#¿NOMBRE?");
    assert_eq!(um.get_cell_type(0, 3, 1).unwrap(), CellType::ErrorValue);
    // HALLAZGO: una fórmula con nombres en inglés NO se acepta en `es`
    // (set_user_input devuelve Ok pero la celda queda con error de nombre, y el
    // texto se conserva con el nombre desconocido en minúsculas).
    um.set_user_input(0, 4, 1, "=SUM(1,2)").unwrap();
    assert_eq!(valor(&um, 0, 4, 1), "#¿NOMBRE?");
    assert_eq!(contenido(&um, 0, 4, 1), "=sum(1,2)");
    // HALLAZGO: los nombres en español sí se aceptan.
    um.set_user_input(0, 5, 1, "=VERDADERO()").unwrap();
    assert_eq!(valor(&um, 0, 5, 1), "VERDADERO");
    um.set_user_input(0, 6, 1, "=SUMA(1,2)").unwrap();
    assert_eq!(valor(&um, 0, 6, 1), "3");
    // HALLAZGO: en `en` el error se llama `#NAME?` (el reintento del núcleo
    // reconoce el error de nombre por este texto, que depende del idioma).
    um.set_language("en").unwrap();
    um.set_user_input(0, 7, 1, "=VERDADERO()").unwrap();
    assert_eq!(valor(&um, 0, 7, 1), "#NAME?");
    // HALLAZGO: la hoja inicial se llama "Hoja1" en `es` y "Sheet1" en `en`.
    assert_eq!(
        nuevo("en", "es").get_worksheets_properties()[0].name,
        "Hoja1"
    );
    assert_eq!(
        nuevo("en", "en").get_worksheets_properties()[0].name,
        "Sheet1"
    );
    registrar(
        "h05_idioma",
        json!({"error_nombre_es": "#¿NOMBRE?", "error_nombre_en": "#NAME?", "ingles_en_es": "error de nombre; texto conservado en minúsculas"}),
    );
}

#[test]
fn h05_set_language_no_evalua_ni_entra_en_el_historial() {
    // HALLAZGO: `set_language` no entra en el historial: un libro recién creado
    // sigue sin nada que deshacer, y deshacer después de cambiar de idioma
    // deshace la edición, no el idioma.
    let mut um = nuevo("en", "en");
    um.set_language("es").unwrap();
    assert!(!um.can_undo());
    um.set_user_input(0, 1, 1, "5").unwrap();
    um.set_language("en").unwrap();
    assert_eq!(um.get_language(), "en");
    um.undo().unwrap();
    assert_eq!(contenido(&um, 0, 1, 1), "");
    assert!(!um.can_undo());

    // HALLAZGO: `set_language` no evalúa: con la evaluación pausada se escribe
    // `=1+1` (queda sin evaluar y se ve `#ERROR!`), se reanuda (reanudar tampoco
    // evalúa), se cambia el idioma, y la celda sigue sin evaluar hasta `evaluate()`.
    let mut um = nuevo("en", "en");
    um.pause_evaluation();
    um.set_user_input(0, 1, 1, "=1+1").unwrap();
    assert_eq!(valor(&um, 0, 1, 1), "#ERROR!");
    um.resume_evaluation();
    assert_eq!(valor(&um, 0, 1, 1), "#ERROR!");
    um.set_language("es").unwrap();
    assert_eq!(valor(&um, 0, 1, 1), "#ERROR!");
    um.evaluate();
    assert_eq!(valor(&um, 0, 1, 1), "2");
    registrar(
        "h05_set_language",
        json!({"evalua": false, "entra_en_historial": false, "celda_sin_evaluar_se_ve": "#ERROR!"}),
    );
}

#[test]
fn h05_locale_define_separadores_y_decimales() {
    // HALLAZGO: el idioma (`language`) solo decide los NOMBRES de función; el
    // `locale` decide el separador de argumentos y el decimal al teclear. Con
    // locale `es`: argumentos con `;` y decimal `,`. Con locale `en` + idioma
    // `es`: nombres en español con `,` de argumentos y `.` decimal (como Excel
    // en español de México). El plan (§5.2) suponía `locale_id` = idioma: eso
    // fija el convenio de España; es una decisión del dueño, no un detalle.
    let mut es_es = nuevo("es", "es");
    es_es.set_user_input(0, 1, 1, "=SUMA(1;2)").unwrap();
    assert_eq!(valor(&es_es, 0, 1, 1), "3");
    es_es.set_user_input(0, 2, 1, "=SUMA(1,2)").unwrap();
    assert_eq!(
        valor(&es_es, 0, 2, 1),
        "1,2",
        "con locale es la coma es decimal: SUMA(1,2) = SUMA(1.2)"
    );
    es_es.set_user_input(0, 3, 1, "1,5").unwrap();
    assert_eq!(es_es.get_cell_type(0, 3, 1).unwrap(), CellType::Number);
    es_es.set_user_input(0, 4, 1, "1.5").unwrap();
    assert_eq!(es_es.get_cell_type(0, 4, 1).unwrap(), CellType::Text);

    let mut en_es = nuevo("en", "es");
    en_es.set_user_input(0, 1, 1, "=SUMA(1,2)").unwrap();
    assert_eq!(valor(&en_es, 0, 1, 1), "3");
    en_es.set_user_input(0, 2, 1, "=SUMA(1;2)").unwrap();
    assert_eq!(
        valor(&en_es, 0, 2, 1),
        "#ERROR!",
        "con locale en el `;` no separa argumentos"
    );
    en_es.set_user_input(0, 3, 1, "1.5").unwrap();
    assert_eq!(en_es.get_cell_type(0, 3, 1).unwrap(), CellType::Number);
    en_es.set_user_input(0, 4, 1, "1,5").unwrap();
    assert_eq!(en_es.get_cell_type(0, 4, 1).unwrap(), CellType::Text);

    // HALLAZGO: las funciones del conjunto de T024 existen en español con el
    // nombre esperado (BUSCARV, BUSCARX, SI.ERROR, CONTAR.SI, SI).
    for (celda, f, esperado) in [
        (1, "=SI(A10>0,\"si\",\"no\")", "si"),
        (2, "=SI.ERROR(1/0,9)", "9"),
        (3, "=BUSCARV(7,A10:A11,1,FALSO())", "7"),
        (4, "=CONTAR.SI(A10:A11,\">5\")", "1"),
        (5, "=BUSCARX(7,A10:A11,A10:A11)", "7"),
    ] {
        let mut um = nuevo("en", "es");
        um.set_user_input(0, 10, 1, "5").unwrap();
        um.set_user_input(0, 11, 1, "7").unwrap();
        um.set_user_input(0, celda, 1, f).unwrap();
        assert_eq!(valor(&um, 0, celda, 1), esperado, "{f}");
    }
    registrar(
        "h05_locale",
        json!({
            "locale_es": {"argumentos": ";", "decimal": ","},
            "locale_en_idioma_es": {"argumentos": ",", "decimal": "."},
            "decision_pendiente": "qué locale usa Lang::Es (España o México)",
        }),
    );
}

#[test]
fn h05_reintento_con_undo_deja_una_entrada_de_historial() {
    // HALLAZGO: la estrategia (A) de plan §5.7 funciona: en `es`, teclear la
    // fórmula en inglés da error; `undo()`, `set_language("en")`, repetir la
    // entrada, `set_language("es")` deja la fórmula correcta y UNA sola entrada
    // de historial neta (el intento fallido se descarta de la pila de rehacer).
    let mut um = nuevo("en", "es");
    um.set_user_input(0, 1, 1, "5").unwrap();
    um.set_user_input(0, 2, 1, "7").unwrap();
    um.set_user_input(0, 3, 1, "=SUM(A1:A2)").unwrap();
    assert_eq!(valor(&um, 0, 3, 1), "#¿NOMBRE?");
    um.undo().unwrap();
    assert_eq!(contenido(&um, 0, 3, 1), "");
    assert!(um.can_redo());
    um.set_language("en").unwrap();
    um.set_user_input(0, 3, 1, "=SUM(A1:A2)").unwrap();
    um.set_language("es").unwrap();
    assert_eq!(valor(&um, 0, 3, 1), "12");
    assert_eq!(contenido(&um, 0, 3, 1), "=SUMA(A1:A2)");
    assert!(!um.can_redo(), "el intento fallido ya no se puede rehacer");
    um.undo().unwrap(); // una sola entrada deshace C3...
    assert_eq!(contenido(&um, 0, 3, 1), "");
    assert_eq!(contenido(&um, 0, 2, 1), "7"); // ...sin tocar A2
    um.undo().unwrap();
    assert_eq!(contenido(&um, 0, 2, 1), "");
}

#[test]
fn h05_bug_formula_con_error_de_analisis_contamina_otra_igual() {
    // HALLAZGO (defecto de 0.9.1, hallado de paso): si una fórmula no se puede
    // ANALIZAR (aquí `,` es decimal en locale `es`) se guarda con su texto en
    // inglés, y otra fórmula válida que se escriba después con el mismo texto
    // interno REUTILIZA la entrada fallida: `=SI(1>0;2;3)` se ve como
    // `=IF(1>0,2,3)` y da `#ERROR!`. No es el caso del error de NOMBRE (ese texto
    // queda en minúsculas y no choca). Si una versión posterior lo arregla,
    // esta aserción falla y hay que revisar plan §5.7.
    let mut um = nuevo("es", "es");
    um.set_user_input(0, 1, 1, "=IF(1>0,2,3)").unwrap();
    um.set_user_input(0, 2, 1, "=SI(1>0;2;3)").unwrap();
    assert_eq!(contenido(&um, 0, 2, 1), "=IF(1>0,2,3)");
    assert_eq!(valor(&um, 0, 2, 1), "#ERROR!");
    // Con otro texto interno no pasa nada.
    um.set_user_input(0, 3, 1, "=SI(2>1;2;3)").unwrap();
    assert_eq!(valor(&um, 0, 3, 1), "2");
    registrar(
        "h05_bug_formula_fallida",
        json!({"reproduccion": "es/es: =IF(1>0,2,3) y luego =SI(1>0;2;3) -> segunda queda =IF(1>0,2,3) con #ERROR!"}),
    );
}

// ---------------------------------------------------------------------------
// (6) Coste de evaluate()  y  (7) set_user_input evalúa todo; pause_evaluation lo evita
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "medición: solo en --release")]
fn h06_coste_de_evaluate_y_de_una_edicion() {
    let _guardia = PESADA.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let grande = dir.path().join("grande.xlsx");
    let chico = dir.path().join("chico.xlsx");

    let t = Instant::now();
    generar_xlsx(&grande, 100_000, 20, 10_000);
    let generar_ms = ms(t);
    generar_xlsx(&chico, 2, 20, 10_000);

    // Libro de 100 000 x 20 con 10 000 fórmulas que dependen de U1.
    let t = Instant::now();
    let modelo = load_from_xlsx(grande.to_str().unwrap(), "en", "UTC", "en").unwrap();
    let carga_ms = ms(t);
    let mut um = UserModel::from_model(modelo);

    // Valor guardado en el archivo (0, el que pone rust_xlsxwriter) hasta evaluar.
    assert_eq!(valor(&um, 0, 2, 22), "0");
    let t = Instant::now();
    um.evaluate();
    let primera_ms = ms(t);
    // HALLAZGO: V2 = U1 * A2 = 2 * 7 = 14, y V10001 = 2 * A10001.
    assert_eq!(valor(&um, 0, 2, 22), "14");
    let a_10001: f64 = valor(&um, 0, 10_001, 1).parse().unwrap();
    assert_eq!(
        valor(&um, 0, 10_001, 22).parse::<f64>().unwrap(),
        2.0 * a_10001
    );

    let evaluate_ms = mediana(
        (0..3)
            .map(|_| {
                let t = Instant::now();
                um.evaluate();
                ms(t)
            })
            .collect(),
    );

    // Una edición completa: cambiar U1 (de la que dependen las 10 000 fórmulas).
    let mut edicion = Vec::new();
    for (i, v) in ["3", "4", "5"].iter().enumerate() {
        let t = Instant::now();
        um.set_user_input(0, 1, 21, v).unwrap();
        edicion.push(ms(t));
        assert_eq!(valor(&um, 0, 2, 22), (7 * (i + 3)).to_string());
    }
    let edit_ms = mediana(edicion);

    // Edición de una celda SIN relación con las fórmulas (datos, columna C).
    let t = Instant::now();
    um.set_user_input(0, 50_000, 3, "texto nuevo").unwrap();
    let edit_independiente_ms = ms(t);

    // (7) con la evaluación pausada la edición no evalúa.
    um.pause_evaluation();
    let mut pausadas = Vec::new();
    for v in ["6", "7", "8"] {
        let t = Instant::now();
        um.set_user_input(0, 1, 21, v).unwrap();
        pausadas.push(ms(t));
    }
    let edit_pausada_ms = mediana(pausadas);
    // HALLAZGO: con la evaluación pausada, V2 sigue valiendo lo de la última evaluación (7*5 = 35).
    assert_eq!(valor(&um, 0, 2, 22), "35");
    um.resume_evaluation();
    assert_eq!(valor(&um, 0, 2, 22), "35", "reanudar no evalúa");
    let t = Instant::now();
    um.evaluate();
    let reanudar_y_evaluar_ms = ms(t);
    assert_eq!(valor(&um, 0, 2, 22), "56"); // 7 * 8

    // Coste del reintento de idioma (plan §5.7, T024) sobre este libro, ya en `es`:
    // edición normal, estrategia (A) ingenua y (A) con la evaluación pausada.
    um.set_language("es").unwrap();
    let t = Instant::now();
    um.set_user_input(0, 1, 24, "=SUMA(A2:A100)").unwrap();
    let normal_es_ms = ms(t);
    let esperado = valor(&um, 0, 1, 24);
    let t = Instant::now();
    um.set_user_input(0, 2, 24, "=SUM(A2:A100)").unwrap(); // intento en el idioma activo: falla
    assert_eq!(valor(&um, 0, 2, 24), "#¿NOMBRE?");
    um.undo().unwrap();
    um.set_language("en").unwrap();
    um.set_user_input(0, 2, 24, "=SUM(A2:A100)").unwrap();
    um.set_language("es").unwrap();
    let reintento_a_ms = ms(t);
    assert_eq!(valor(&um, 0, 2, 24), esperado);
    let t = Instant::now();
    um.set_user_input(0, 3, 24, "=SUM(A2:A100)").unwrap(); // 1.ª evaluación: se descubre el error
    assert_eq!(valor(&um, 0, 3, 24), "#¿NOMBRE?");
    um.pause_evaluation();
    um.undo().unwrap();
    um.set_language("en").unwrap();
    um.set_user_input(0, 3, 24, "=SUM(A2:A100)").unwrap();
    um.set_language("es").unwrap();
    um.resume_evaluation();
    um.evaluate(); // 2.ª y última evaluación
    let reintento_a_pausa_ms = ms(t);
    assert_eq!(valor(&um, 0, 3, 24), esperado);

    // Mismo libro SIN fórmulas: se vacían las 10 000 con la evaluación pausada.
    um.pause_evaluation();
    um.range_clear_contents(&Area {
        sheet: 0,
        row: 2,
        column: 22,
        width: 1,
        height: 10_000,
    })
    .unwrap();
    um.resume_evaluation();
    let sin_formulas_ms = mediana(
        (0..3)
            .map(|_| {
                let t = Instant::now();
                um.evaluate();
                ms(t)
            })
            .collect(),
    );
    let t = Instant::now();
    um.set_user_input(0, 1, 21, "9").unwrap();
    let edit_sin_formulas_ms = ms(t);
    drop(um);

    // 10 000 fórmulas con casi nada de datos: cuánto cuestan las fórmulas solas.
    let mut um_chico = abrir(&chico, "en", "en");
    um_chico.evaluate();
    let evaluate_chico_ms = mediana(
        (0..3)
            .map(|_| {
                let t = Instant::now();
                um_chico.evaluate();
                ms(t)
            })
            .collect(),
    );
    let t = Instant::now();
    um_chico.set_user_input(0, 1, 21, "3").unwrap();
    let edit_chico_ms = ms(t);

    // HALLAZGO (6): una edición cuesta lo mismo que una evaluación completa del
    // libro, también si la celda editada no tiene nada que ver con las fórmulas.
    assert!(
        edit_ms > 0.5 * evaluate_ms,
        "edit {edit_ms} vs evaluate {evaluate_ms}"
    );
    assert!(edit_independiente_ms > 0.5 * evaluate_ms);
    // HALLAZGO (7): con `pause_evaluation` la edición es órdenes de magnitud más barata.
    assert!(
        edit_pausada_ms < 0.1 * edit_ms,
        "pausada {edit_pausada_ms} vs {edit_ms}"
    );

    registrar(
        "h06_h07_coste_evaluate",
        json!({
            "libro": "100000 x 20 (2 000 000 celdas) + 10 000 fórmulas =$U$1*A{n} que dependen de U1",
            "generar_xlsx_ms": redondear(generar_ms),
            "carga_ms": redondear(carga_ms),
            "primera_evaluacion_ms": redondear(primera_ms),
            "evaluate_ms_mediana_de_3": redondear(evaluate_ms),
            "edicion_de_U1_ms_mediana_de_3": redondear(edit_ms),
            "edicion_de_celda_independiente_ms": redondear(edit_independiente_ms),
            "edicion_con_pause_ms_mediana_de_3": redondear(edit_pausada_ms),
            "resume_y_evaluate_ms": redondear(reanudar_y_evaluar_ms),
            "reintento_de_idioma": {
                "edicion_normal_en_es_ms": redondear(normal_es_ms),
                "estrategia_A_ingenua_ms": redondear(reintento_a_ms),
                "estrategia_A_con_pause_ms": redondear(reintento_a_pausa_ms),
            },
            "mismo_libro_sin_formulas": {
                "evaluate_ms_mediana_de_3": redondear(sin_formulas_ms),
                "edicion_ms": redondear(edit_sin_formulas_ms),
            },
            "solo_formulas_sin_datos": {
                "libro": "2 filas x 20 + 10 000 fórmulas",
                "evaluate_ms_mediana_de_3": redondear(evaluate_chico_ms),
                "edicion_ms": redondear(edit_chico_ms),
            },
        }),
    );
}

#[test]
fn h07_set_user_input_evalua_todo_y_pause_lo_evita() {
    let mut um = nuevo("en", "en");
    um.set_user_input(0, 1, 1, "1").unwrap();
    um.set_user_input(0, 1, 2, "=A1*2").unwrap();
    um.set_user_input(0, 1, 3, "=RAND()").unwrap();
    assert_eq!(valor(&um, 0, 1, 2), "2");
    let r0 = valor(&um, 0, 1, 3);
    // HALLAZGO: editar una celda sin ninguna relación (D5) reevalúa también la
    // fórmula volátil C1: se evalúa TODO el libro, sin grafo de dependencias.
    um.set_user_input(0, 5, 4, "x").unwrap();
    let r1 = valor(&um, 0, 1, 3);
    assert_ne!(r0, r1, "RAND() debería haberse recalculado al editar D5");

    // HALLAZGO: con `pause_evaluation()` la edición no evalúa nada: B1 conserva
    // su valor viejo y C1 (volátil) también.
    um.pause_evaluation();
    um.set_user_input(0, 1, 1, "5").unwrap();
    assert_eq!(valor(&um, 0, 1, 2), "2");
    assert_eq!(valor(&um, 0, 1, 3), r1);
    // `resume_evaluation()` solo baja la bandera, no evalúa.
    um.resume_evaluation();
    assert_eq!(valor(&um, 0, 1, 2), "2");
    // `evaluate()` explícito pone todo al día.
    um.evaluate();
    assert_eq!(valor(&um, 0, 1, 2), "10");
    assert_ne!(valor(&um, 0, 1, 3), r1);
    // HALLAZGO: la siguiente edición ya evalúa de nuevo (la pausa no es permanente).
    um.set_user_input(0, 1, 1, "6").unwrap();
    assert_eq!(valor(&um, 0, 1, 2), "12");
    // HALLAZGO: deshacer/rehacer también evalúan (el valor de B1 vuelve a 10).
    um.undo().unwrap();
    assert_eq!(valor(&um, 0, 1, 2), "10");
    registrar(
        "h07_semantica",
        json!({"edicion_evalua_todo": true, "formula_volatil_independiente_se_recalcula": true, "pause_evita": true, "resume_evalua": false, "undo_evalua": true}),
    );
}

// ---------------------------------------------------------------------------
// (8) resolve_color  y  (9) workbook público
// ---------------------------------------------------------------------------

fn es_rrggbb(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[test]
fn h08_resolve_color_devuelve_rrggbb() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("colores.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let rgb = Format::new()
        .set_background_color(Color::RGB(0xFFCC00))
        .set_font_color(Color::RGB(0x336699));
    let tema = Format::new()
        .set_background_color(Color::Theme(4, 3))
        .set_font_color(Color::Theme(9, 0));
    ws.write_string_with_format(0, 0, "rgb", &rgb).unwrap();
    ws.write_string_with_format(1, 0, "tema", &tema).unwrap();
    ws.write_string(2, 0, "plano").unwrap();
    wb.save(&ruta).unwrap();
    let um = abrir(&ruta, "en", "en");

    let est = um.get_cell_style(0, 1, 1).unwrap();
    // HALLAZGO: un color RGB del archivo se resuelve a `#RRGGBB` (mayúsculas).
    assert_eq!(est.fill.color, IcColor::Rgb("#FFCC00".to_string()));
    assert_eq!(um.resolve_color(&est.fill.color), "#FFCC00");
    assert_eq!(um.resolve_color(&est.font.color), "#336699");
    // HALLAZGO: un color de tema se resuelve (con su tinte) a `#RRGGBB`.
    let est = um.get_cell_style(0, 2, 1).unwrap();
    assert!(matches!(est.fill.color, IcColor::Theme(4, _)));
    let fondo_tema = um.resolve_color(&est.fill.color);
    assert!(es_rrggbb(&fondo_tema), "{fondo_tema}");
    assert!(es_rrggbb(&um.resolve_color(&est.font.color)));
    // HALLAZGO: SIN color (`Color::None`, p. ej. celda sin relleno) devuelve la
    // cadena VACÍA, no un color: el núcleo debe tratarla como «sin relleno».
    let est = um.get_cell_style(0, 3, 1).unwrap();
    assert_eq!(est.fill.color, IcColor::None);
    assert_eq!(um.resolve_color(&est.fill.color), "");
    // HALLAZGO: el color de fuente por defecto es el tema 1 y resuelve a `#000000`.
    assert_eq!(um.resolve_color(&est.font.color), "#000000");
    registrar(
        "h08_resolve_color",
        json!({"formato": "#RRGGBB", "sin_color": "cadena vacía", "tema_resuelto": fondo_tema}),
    );
}

#[test]
fn h09_workbook_publico_y_coste_de_get_row_height() {
    let _guardia = PESADA.lock().unwrap_or_else(|e| e.into_inner());
    // 100 000 filas, todas con altura propia (peor caso para el escaneo lineal).
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("filas.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_column_width(2, 30.0).unwrap();
    for r in 0..100_000u32 {
        ws.write_number(r, 0, f64::from(r)).unwrap();
        ws.set_row_height(r, 16.0 + f64::from(r % 3)).unwrap();
    }
    wb.save(&ruta).unwrap();
    let um = abrir(&ruta, "en", "en");

    // HALLAZGO: `get_model()` devuelve `&Model` y `Model.workbook` es PÚBLICO,
    // igual que `Workbook.worksheets`, `Worksheet.rows` / `.cols` y los campos de
    // `Row` y `Col`: se pueden leer las excepciones de tamaño sin llamar 100 000
    // veces a `get_row_height`. Unidades en bruto: `Row.height` en puntos y
    // `Col.width` en caracteres (hay que aplicar ROW_HEIGHT_FACTOR / COLUMN_WIDTH_FACTOR).
    let hoja = &um.get_model().workbook.worksheets[0];
    assert_eq!(hoja.rows.len(), 100_000);
    assert_eq!(hoja.cols.len(), 1);
    let (c0, c1) = (hoja.cols[0].min, hoja.cols[0].max);
    assert_eq!((c0, c1), (3, 3));
    assert!(hoja.cols[0].custom_width && !hoja.cols[0].hidden);
    let fila = &hoja.rows[10];
    assert_eq!(fila.r, 11);
    assert_eq!(
        um.get_row_height(0, 11).unwrap(),
        fila.height * ROW_HEIGHT_FACTOR
    );
    // HALLAZGO: `Row` solo existe para filas con altura/estilo/oculta explícitos
    // (las demás no tienen entrada), y `Worksheet::dimension()` también es público.
    let dim = hoja.dimension();
    assert_eq!(
        (dim.min_row, dim.max_row, dim.min_column, dim.max_column),
        (1, 100_000, 1, 1)
    );

    // HALLAZGO: `get_row_height` es LINEAL en el número de entradas `Row`
    // (`Worksheet::row_height` recorre `rows`), así que construir la geometría
    // llamándola fila por fila es CUADRÁTICO cuando el archivo trae altura propia
    // en todas las filas. Se mide con 1 000 llamadas repartidas y se
    // extrapola a 100 000 (no se corre completo: serían decenas de segundos).
    // La muestra es una de cada 100 filas, para que el coste medio sea el de todo el rango.
    let t = Instant::now();
    let mut acumulado = 0.0;
    for r in (1..=100_000).step_by(100) {
        acumulado += um.get_row_height(0, r).unwrap();
    }
    let mil_llamadas_ms = ms(t);
    assert!(acumulado > 0.0);
    let por_llamada_us = mil_llamadas_ms * 1000.0 / 1000.0;
    let extrapolado_ms = mil_llamadas_ms * 100.0;
    // Camino recomendado: leer `rows` una vez y quedarse con lo distinto del valor por defecto.
    let t = Instant::now();
    let excepciones: Vec<(i32, f64)> = hoja
        .rows
        .iter()
        .filter(|f| f.hidden || f.height * ROW_HEIGHT_FACTOR != 25.0)
        .map(|f| {
            (
                f.r,
                if f.hidden {
                    0.0
                } else {
                    f.height * ROW_HEIGHT_FACTOR
                },
            )
        })
        .collect();
    let leer_rows_ms = ms(t);
    assert_eq!(excepciones.len(), 100_000);
    registrar(
        "h09_workbook_publico",
        json!({
            "workbook_publico": true, "rows_y_cols_publicos": true, "dimension_publica": true,
            "filas_con_entrada_row": hoja.rows.len(),
            "get_row_height_1000_llamadas_ms": redondear(mil_llamadas_ms),
            "get_row_height_por_llamada_us": redondear(por_llamada_us),
            "get_row_height_x100000_extrapolado_ms": redondear(extrapolado_ms),
            "leer_rows_directamente_100000_ms": redondear(leer_rows_ms),
        }),
    );
}

// ---------------------------------------------------------------------------
// (10) Partes no soportadas al cargar
// ---------------------------------------------------------------------------

#[test]
fn h10_la_carga_no_informa_partes_no_soportadas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("variado.xlsx");
    let mut wb = Workbook::new();
    wb.define_name("MiRango", "=Sheet1!$A$1:$A$3").unwrap();
    let ws = wb.add_worksheet();
    for (i, v) in [3.0, 5.0, 8.0].iter().enumerate() {
        ws.write_number(i as u32, 0, *v).unwrap();
    }
    ws.merge_range(4, 0, 4, 2, "combinada", &Format::new())
        .unwrap();
    ws.insert_note(6, 0, &Note::new("una nota")).unwrap();
    let mut grafico = Chart::new(ChartType::Column);
    grafico.add_series().set_values("=Sheet1!$A$1:$A$3");
    ws.insert_chart(8, 3, &grafico).unwrap();
    ws.insert_image(20, 3, &Image::new_from_buffer(&PNG_1X1).unwrap())
        .unwrap();
    ws.add_data_validation(
        0,
        4,
        5,
        4,
        &DataValidation::new().allow_whole_number(DataValidationRule::Between(1, 10)),
    )
    .unwrap();
    wb.save(&ruta).unwrap();
    let original = std::fs::read(&ruta).unwrap();

    // HALLAZGO: `load_from_xlsx` devuelve `Result<Model, XlsxError>`: la firma no
    // tiene canal de avisos y la carga de un archivo con gráfico, imagen y
    // validación de datos termina en `Ok` sin ninguna señal.
    let modelo = load_from_xlsx(ruta.to_str().unwrap(), "en", "UTC", "en");
    let um = UserModel::from_model(modelo.expect("la carga no se queja de lo que no entiende"));
    // Lo que sí modela se conserva.
    assert_eq!(um.get_model().get_merged_cells(0).unwrap().len(), 1);
    assert_eq!(um.get_defined_name_list().len(), 1);
    let comentarios_en_el_modelo = um.get_model().workbook.worksheets[0].comments.len();

    let partes_origen = partes(&original);
    for esperada in [
        "xl/charts/chart1.xml",
        "xl/drawings/drawing1.xml",
        "xl/media/image1.png",
    ] {
        assert!(
            partes_origen.iter().any(|p| p == esperada),
            "{esperada} en el original"
        );
    }
    assert!(parte_texto(&original, "xl/worksheets/sheet1.xml").contains("<dataValidations"));

    // Guardar con IronCalc pierde esas partes en silencio.
    let guardado = save_xlsx_to_writer(um.get_model(), Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    let partes_guardado = partes(&guardado);
    // HALLAZGO: tras guardar, el gráfico, el dibujo y la imagen ya no están, ni
    // la validación de datos en la hoja.
    for perdida in [
        "xl/charts/chart1.xml",
        "xl/drawings/drawing1.xml",
        "xl/media/image1.png",
    ] {
        assert!(
            !partes_guardado.iter().any(|p| p == perdida),
            "{perdida} no debería sobrevivir"
        );
    }
    assert!(!parte_texto(&guardado, "xl/worksheets/sheet1.xml").contains("<dataValidations"));
    let perdidas: Vec<&String> = partes_origen
        .iter()
        .filter(|p| !p.ends_with('/') && !partes_guardado.contains(p))
        .collect();

    // Detección propia (la que el núcleo debe hacer al abrir, aclaración 8):
    // mirar las entradas del paquete y la hoja. Prototipo mínimo.
    let avisos = detectar_no_soportado(&original);
    assert!(avisos.iter().any(|a| a.contains("gráfico")));
    assert!(
        avisos
            .iter()
            .any(|a| a.contains("imagen") || a.contains("dibujo"))
    );
    assert!(avisos.iter().any(|a| a.contains("validación")));
    registrar(
        "h10_partes_no_soportadas",
        json!({
            "carga_informa_partes_no_soportadas": false,
            "notas_leidas_al_modelo": comentarios_en_el_modelo,
            "partes_perdidas_al_guardar": perdidas,
            "deteccion_propia_necesaria": true,
            "avisos_del_prototipo": avisos,
        }),
    );
}

/// Prototipo de la detección propia: partes del paquete que IronCalc descarta.
fn detectar_no_soportado(bytes: &[u8]) -> Vec<String> {
    let nombres = partes(bytes);
    let mut avisos = Vec::new();
    let hay = |prefijo: &str| nombres.iter().any(|n| n.starts_with(prefijo));
    if hay("xl/charts/") {
        avisos.push("gráficos".to_string());
    }
    if hay("xl/media/") {
        avisos.push("imágenes".to_string());
    }
    if hay("xl/drawings/") {
        avisos.push("dibujos".to_string());
    }
    if hay("xl/pivotTables/") {
        avisos.push("tablas dinámicas".to_string());
    }
    if hay("xl/vbaProject.bin") {
        avisos.push("macros".to_string());
    }
    for n in nombres
        .iter()
        .filter(|n| n.starts_with("xl/worksheets/sheet"))
    {
        if n.ends_with(".xml") && parte_texto(bytes, n).contains("<dataValidations") {
            avisos.push(format!("validación de datos en {n}"));
        }
    }
    avisos
}

// ---------------------------------------------------------------------------
// (11) paste_csv_string  y  (12) Clipboard.csv
// ---------------------------------------------------------------------------

fn area(fila: i32, col: i32) -> Area {
    Area {
        sheet: 0,
        row: fila,
        column: col,
        width: 1,
        height: 1,
    }
}

#[test]
fn h11_paste_csv_string_con_comillas_y_saltos_de_linea() {
    let mut um = nuevo("en", "en");
    // HALLAZGO: el pegado lee TSV con comillas RFC 4180: un campo entre comillas
    // puede llevar tabuladores, saltos de línea y comillas duplicadas; \r\n
    // como fin de fila se acepta. Una sola entrada de historial.
    let tsv = "a\t\"b\tc\"\t\"linea1\nlinea2\"\t\"dice \"\"hola\"\"\"\r\n1\t2\t3\t=1+1\r\n";
    um.paste_csv_string(&area(1, 1), tsv).unwrap();
    assert_eq!(contenido(&um, 0, 1, 1), "a");
    assert_eq!(contenido(&um, 0, 1, 2), "b\tc");
    assert_eq!(contenido(&um, 0, 1, 3), "linea1\nlinea2");
    assert_eq!(contenido(&um, 0, 1, 4), "dice \"hola\"");
    assert_eq!(contenido(&um, 0, 2, 1), "1");
    assert_eq!(contenido(&um, 0, 2, 3), "3");
    // HALLAZGO: lo pegado se interpreta como si se tecleara: `=1+1` es fórmula y se evalúa.
    assert_eq!(valor(&um, 0, 2, 4), "2");
    um.undo().unwrap();
    assert!(
        !um.can_undo(),
        "el pegado entero es UNA entrada de historial"
    );
    assert_eq!(contenido(&um, 0, 1, 2), "");
    assert_eq!(contenido(&um, 0, 2, 4), "");

    // HALLAZGO: las filas con distinto número de campos que la PRIMERA se descartan
    // EN SILENCIO (el lector `csv` es estricto y `paste_csv_string` hace
    // `records().flatten()`): una fila corta desaparece sin error y las siguientes
    // suben un lugar. El núcleo debe rellenar con tabuladores hasta un rectángulo
    // antes de pegar.
    let mut um = nuevo("en", "en");
    um.paste_csv_string(&area(1, 1), "a\tb\tc\n1\t2\nx\ty\tz\n")
        .unwrap();
    assert_eq!(contenido(&um, 0, 1, 3), "c");
    assert_eq!(
        contenido(&um, 0, 2, 1),
        "x",
        "la fila corta se perdió y la siguiente subió a su lugar"
    );
    assert_eq!(
        contenido(&um, 0, 3, 1),
        "",
        "la tabla pegada quedó con una fila menos"
    );
    let mut um = nuevo("en", "en");
    um.paste_csv_string(&area(1, 1), "a\tb\tc\n1\t2\t\nx\ty\tz\n")
        .unwrap();
    assert_eq!(contenido(&um, 0, 2, 2), "2");
    assert_eq!(contenido(&um, 0, 3, 3), "z");

    // HALLAZGO: la extensión del pegado es el ancho de la primera fila y los
    // campos vacíos BORRAN lo que hubiera: tabuladores finales de más
    // destruyen datos a la derecha (por eso el núcleo recorta la extensión final).
    let mut um = nuevo("en", "en");
    um.set_user_input(0, 1, 4, "conservar").unwrap();
    um.paste_csv_string(&area(1, 1), "a\tb\t\t\n").unwrap();
    assert_eq!(
        contenido(&um, 0, 1, 4),
        "",
        "D1 se borró por los tabuladores finales"
    );
    let mut um = nuevo("en", "en");
    um.set_user_input(0, 1, 4, "conservar").unwrap();
    um.paste_csv_string(&area(1, 1), "a\tb\n").unwrap();
    assert_eq!(contenido(&um, 0, 1, 4), "conservar");
    registrar(
        "h11_paste_csv_string",
        json!({
            "comillas_tab_salto_de_linea": "ok", "crlf": "ok", "una_entrada_de_historial": true,
            "filas_mas_cortas_que_la_primera": "se descartan en silencio",
            "campos_vacios_finales": "borran celdas a la derecha",
        }),
    );
}

#[test]
fn h12_clipboard_csv_no_es_accesible() {
    let mut um = nuevo("en", "en");
    um.set_user_input(0, 1, 1, "a").unwrap();
    um.set_user_input(0, 1, 2, "b\tc").unwrap();
    um.set_user_input(0, 2, 1, "1234.5").unwrap();
    um.set_selected_range(1, 1, 2, 2).unwrap();
    let portapapeles = um.copy_to_clipboard().unwrap();
    // HALLAZGO: `Clipboard` y su campo `csv` son `pub(crate)`: el tipo ni siquiera
    // se exporta (solo `ClipboardData`), así que `portapapeles.csv` NO compila desde
    // fuera. Eso no se puede afirmar con una aserción: se documenta aquí y en el
    // código de IronCalc (`user_model/clipboard.rs`). Lo único alcanzable es la
    // serialización con serde (el tipo deriva `Serialize`), que sí lo contiene:
    let v = serde_json::to_value(&portapapeles).unwrap();
    let csv = v["csv"].as_str().unwrap().to_string();
    // El `csv` usa tabulador, comillas RFC 4180 y los valores FORMATEADOS...
    assert_eq!(csv, "a\t\"b\tc\"\n1234.5");
    // ...pero aplica `trim()` al resultado entero: la última fila perdió el tabulador
    // de su celda vacía (B2) y una selección que acaba en celdas vacías pierde todos
    // los tabuladores finales: no es un TSV rectangular fiable. Por eso el núcleo
    // arma su propio TSV.
    um.set_selected_cell(2, 1).unwrap();
    um.set_selected_range(2, 1, 3, 2).unwrap();
    let v2 = serde_json::to_value(um.copy_to_clipboard().unwrap()).unwrap();
    assert_eq!(
        v2["csv"].as_str().unwrap(),
        "1234.5",
        "se perdieron los tabuladores finales"
    );
    registrar(
        "h12_clipboard_csv",
        json!({"campo_csv_accesible": false, "alcanzable_por_serde": true, "aplica_trim": true, "decision": "el núcleo arma su TSV"}),
    );
}

// ---------------------------------------------------------------------------
// Extras que afectan al diseño
// ---------------------------------------------------------------------------

#[test]
fn x01_la_carga_no_evalua_y_deja_los_valores_guardados() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("cache.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_number(0, 0, 2).unwrap();
    ws.write_number(0, 1, 3).unwrap();
    // Valor guardado falso (7) para una fórmula cuyo resultado real es 5.
    ws.write_formula(0, 2, Formula::new("=A1+B1").set_result("7"))
        .unwrap();
    wb.save(&ruta).unwrap();
    let mut um = abrir(&ruta, "en", "en");
    // HALLAZGO: `load_from_xlsx` + `from_model` NO evalúan: se ve el valor guardado
    // en el archivo (7). `tools/compat-check` (§5.9) debe llamar a `evaluate()` y
    // solo entonces comparar; el valor leído tras evaluar (5) viene del recálculo.
    assert_eq!(valor(&um, 0, 1, 3), "7");
    um.evaluate();
    assert_eq!(valor(&um, 0, 1, 3), "5");
    registrar(
        "x01_carga_sin_evaluar",
        json!({"valor_tras_cargar": "7 (guardado)", "valor_tras_evaluate": "5 (recalculado)"}),
    );
}

#[test]
fn x02_guardar_save_to_xlsx_no_sobrescribe_y_entra_en_panico() {
    let dir = tempfile::tempdir().unwrap();
    let um = nuevo("en", "en");
    let ruta = dir.path().join("salida.xlsx");
    // HALLAZGO: `save_to_xlsx` se niega a escribir sobre un archivo que ya existe.
    save_to_xlsx(um.get_model(), ruta.to_str().unwrap()).unwrap();
    let err = save_to_xlsx(um.get_model(), ruta.to_str().unwrap()).unwrap_err();
    assert!(format!("{err:?}").contains("already exists"), "{err:?}");
    // HALLAZGO: si la carpeta no existe (o no hay permiso) PANICA (`File::create(..).unwrap()`
    // dentro de la biblioteca) en lugar de devolver `Err`.
    let inexistente = dir.path().join("no_existe").join("salida.xlsx");
    let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = save_to_xlsx(um.get_model(), inexistente.to_str().unwrap());
    }));
    assert!(
        resultado.is_err(),
        "save_to_xlsx debería entrar en pánico con carpeta inexistente"
    );
    // HALLAZGO: el camino que sirve para el guardado atómico del núcleo (§5.9) es
    // `save_xlsx_to_writer` con un `File` propio: errores como `Result` (el
    // `File::create` lo hace el núcleo), `tmp` + `rename`, sobrescribe el destino.
    let tmp = dir.path().join("salida.xlsx.tmp-1");
    let f = std::fs::File::create(&tmp).unwrap();
    save_xlsx_to_writer(um.get_model(), std::io::BufWriter::new(f)).unwrap();
    std::fs::rename(&tmp, &ruta).unwrap(); // sobrescribe el existente
    let um2 = abrir(&ruta, "en", "en");
    assert_eq!(um2.get_worksheets_properties()[0].name, "Sheet1");
    registrar(
        "x02_guardado",
        json!({"save_to_xlsx_sobrescribe": false, "carpeta_inexistente": "panic", "camino_recomendado": "save_xlsx_to_writer + File propio + rename"}),
    );
}

#[test]
fn x03_user_model_static_es_send_y_sync() {
    // HALLAZGO: `UserModel<'static>` es `Send + Sync` (se comprueba al compilar).
    fn exige_send_sync<T: Send + Sync>() {}
    exige_send_sync::<UserModel<'static>>();
    registrar(
        "x03_send_sync",
        json!({"user_model_static_send_sync": true}),
    );
}

#[test]
fn x04_el_desplazamiento_de_user_model_es_por_celda() {
    let mut um = nuevo("en", "en");
    um.set_top_left_visible_cell(10, 5).unwrap();
    // HALLAZGO: `UserModel` solo conoce la celda superior izquierda visible; su
    // `get_scroll_x/y` se DERIVA sumando los tamaños de las filas/columnas
    // anteriores (cuantizado por celda, O(n) en cada llamada). No hay
    // desplazamiento por píxel: el `View` del núcleo lo lleva aparte y solo
    // sincroniza `set_top_left_visible_cell(row_at(y), col_at(x))`.
    assert_eq!(um.get_scroll_y().unwrap(), 9.0 * 25.0);
    assert_eq!(um.get_scroll_x().unwrap(), 4.0 * 90.0);
    // HALLAZGO: RePág/AvPág dependen de `set_window_height`: con 500 de alto y filas
    // de 25 la página son 20 filas (de la fila 1 a la 21) y la selección sigue a la ventana.
    let mut um = nuevo("en", "en");
    um.set_window_height(500.0);
    um.on_page_down().unwrap();
    assert_eq!(um.get_selected_cell(), (0, 21, 1));
    assert_eq!(um.get_selected_view().top_row, 21);
    um.on_page_up().unwrap();
    assert_eq!(um.get_selected_cell(), (0, 1, 1));
    registrar(
        "x04_scroll_por_celda",
        json!({"scroll_por_pixel": false, "pagina_con_500px_y_filas_de_25": 20}),
    );
}
