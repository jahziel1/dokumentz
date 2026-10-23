//! Ayudantes comunes de las pruebas de la etapa E06: generan los `.xlsx` de prueba con
//! `rust_xlsxwriter` en un directorio temporal (no hay binarios versionados salvo el dorado
//! de `tests/golden/`). Cada prueba incluye este módulo con `mod common;`.
#![allow(dead_code)] // cada archivo de pruebas usa solo parte de los ayudantes

use std::path::{Path, PathBuf};

use dokumentz_sheet::{Core, OpenOptions};
use rust_xlsxwriter::{
    Color, ExcelDateTime, Format, FormatAlign, FormatUnderline, Formula, Workbook,
};

/// Opciones fijas de las pruebas: inglés, `locale` `en`, UTC.
pub fn opciones() -> OpenOptions {
    OpenOptions::default()
}

/// Abre un `.xlsx` con las opciones de las pruebas.
pub fn abrir(ruta: &Path) -> Core {
    Core::open_xlsx(ruta, opciones()).expect("abrir el fixture")
}

/// `empty.xlsx`: un libro con una hoja y ninguna celda.
pub fn crear_empty(dir: &Path) -> PathBuf {
    let ruta = dir.join("empty.xlsx");
    let mut wb = Workbook::new();
    wb.add_worksheet();
    wb.save(&ruta).expect("guardar empty.xlsx");
    ruta
}

/// `small.xlsx`: hoja «Datos» con todos los casos que mira T017. Posiciones (base 1):
///
/// | Celda | Contenido | Texto esperado | Particularidad |
/// |---|---|---|---|
/// | A1 | «Producto» | `Producto` | negrita |
/// | B1 | «Precio» | `Precio` | negrita, alineado a la derecha |
/// | C1 | «Fecha» | `Fecha` | negrita |
/// | D1 | «Doble» | `Doble` | negrita |
/// | A2 | «Manzana» | `Manzana` | texto, alineación general |
/// | B2 | 1234,5 | `1,234.50` | formato `#,##0.00` |
/// | C2 | 2024-03-05 | `2024-03-05` | fecha con formato `yyyy-mm-dd` |
/// | D2 | `=B2*2` | `2469` | fórmula con valor guardado correcto |
/// | A3 | «Pera» | `Pera` | cursiva, subrayado, letra roja `#FF0000` |
/// | B3 | 12 | `12` | número general |
/// | D3 | VERDADERO | `TRUE` | lógico |
/// | A4 | «Uva» | `Uva` | relleno amarillo `#FFFF00`, centrado |
/// | C4 | (vacía) | | solo relleno verde `#00FF00` |
/// | A5 | texto largo | el texto | desborda a la derecha (B5..E5 vacías) |
/// | A6 | texto largo con ajuste | el texto | ajuste de línea, tachado, tamaño 14 |
/// | B6 | 7,12345 | `7.12` | formato `0.00` |
///
/// El rango usado es A1:D6 (6 filas × 4 columnas).
pub fn crear_small(dir: &Path) -> PathBuf {
    let ruta = dir.join("small.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Datos").unwrap();
    let negrita = Format::new().set_bold();
    let negrita_der = Format::new().set_bold().set_align(FormatAlign::Right);
    ws.write_string_with_format(0, 0, "Producto", &negrita)
        .unwrap();
    ws.write_string_with_format(0, 1, "Precio", &negrita_der)
        .unwrap();
    ws.write_string_with_format(0, 2, "Fecha", &negrita)
        .unwrap();
    ws.write_string_with_format(0, 3, "Doble", &negrita)
        .unwrap();

    ws.write_string(1, 0, "Manzana").unwrap();
    ws.write_number_with_format(1, 1, 1234.5, &Format::new().set_num_format("#,##0.00"))
        .unwrap();
    let fecha = ExcelDateTime::from_ymd(2024, 3, 5).unwrap();
    ws.write_datetime_with_format(1, 2, &fecha, &Format::new().set_num_format("yyyy-mm-dd"))
        .unwrap();
    ws.write_formula(1, 3, Formula::new("=B2*2").set_result("2469"))
        .unwrap();

    let roja = Format::new()
        .set_italic()
        .set_underline(FormatUnderline::Single)
        .set_font_color(Color::RGB(0xFF0000));
    ws.write_string_with_format(2, 0, "Pera", &roja).unwrap();
    ws.write_number(2, 1, 12).unwrap();
    ws.write_boolean(2, 3, true).unwrap();

    let amarilla = Format::new()
        .set_background_color(Color::RGB(0xFFFF00))
        .set_align(FormatAlign::Center);
    ws.write_string_with_format(3, 0, "Uva", &amarilla).unwrap();
    ws.write_blank(
        3,
        2,
        &Format::new().set_background_color(Color::RGB(0x00FF00)),
    )
    .unwrap();

    ws.write_string(4, 0, TEXTO_LARGO).unwrap();
    let ajustada = Format::new()
        .set_text_wrap()
        .set_font_strikethrough()
        .set_font_size(14);
    ws.write_string_with_format(5, 0, TEXTO_LARGO, &ajustada)
        .unwrap();
    ws.write_number_with_format(5, 1, 7.12345, &Format::new().set_num_format("0.00"))
        .unwrap();
    wb.save(&ruta).expect("guardar small.xlsx");
    ruta
}

/// Texto de `small.xlsx` A5 y A6.
pub const TEXTO_LARGO: &str = "Texto largo que invade las celdas vacías";

/// `formatos.xlsx`: fila 3 y columna F con formato de fila/columna entera (relleno) y una
/// celda con dato dentro de ellas (F3 «x»), que conserva su propio estilo.
///
/// * Fila 3 completa: relleno azul `#0000FF` (celdas sin dato heredan; A3 tiene un dato).
/// * Columna F completa: relleno gris `#C0C0C0`.
pub fn crear_formatos(dir: &Path) -> PathBuf {
    let ruta = dir.join("formatos.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let azul = Format::new().set_background_color(Color::RGB(0x0000FF));
    let gris = Format::new().set_background_color(Color::RGB(0xC0C0C0));
    ws.set_row_format(2, &azul).unwrap();
    ws.set_column_format(5, &gris).unwrap();
    ws.write_string(0, 0, "dato").unwrap();
    ws.write_string_with_format(2, 0, "en fila azul", &azul)
        .unwrap();
    wb.save(&ruta).expect("guardar formatos.xlsx");
    ruta
}

/// `tamanos.xlsx`: tamaños personalizados.
///
/// * Columna B (índice 2 base 1) ancha: 20 caracteres → 180 px.
/// * Columna D oculta (0 px).
/// * Fila 2 alta: 30 puntos → 46,875 px.
/// * Fila 4 oculta (0 px).
/// * Datos en A1, B2, C3, E5 y A6 para tener algo que dibujar.
pub fn crear_tamanos(dir: &Path) -> PathBuf {
    let ruta = dir.join("tamanos.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_column_width(1, 20).unwrap();
    ws.set_column_hidden(3).unwrap();
    ws.set_row_height(1, 30).unwrap();
    ws.set_row_hidden(3).unwrap();
    ws.write_string(0, 0, "A1").unwrap();
    ws.write_string(1, 1, "B2").unwrap();
    ws.write_string(2, 2, "C3").unwrap();
    ws.write_string(3, 3, "D4 oculta").unwrap();
    ws.write_string(4, 4, "E5").unwrap();
    ws.write_string(5, 0, "A6").unwrap();
    wb.save(&ruta).expect("guardar tamanos.xlsx");
    ruta
}

/// Libro grande para pruebas de bloque y de caché: `filas` × `cols` con números enteros
/// (`fila * 1000 + col`, base 1) y la fila 1 con encabezados `Col{n}`.
pub fn crear_grande(dir: &Path, filas: u32, cols: u16) -> PathBuf {
    let ruta = dir.join("grande.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for c in 0..cols {
        ws.write_string(0, c, format!("Col{}", c + 1)).unwrap();
    }
    for r in 1..filas {
        for c in 0..cols {
            ws.write_number(r, c, f64::from((r + 1) * 1000 + u32::from(c) + 1))
                .unwrap();
        }
    }
    wb.save(&ruta).expect("guardar grande.xlsx");
    ruta
}

// ---------------------------------------------------------------------------------------------
// Ayudantes de E08 (edición, deshacer con dependientes, idioma)
// ---------------------------------------------------------------------------------------------

/// A2 sintético (`spec.md` §7): `filas` × `cols` con números, textos, fechas y estilos (la fila 1
/// son encabezados en negrita), más `formulas` fórmulas que dependen de **una** celda de entrada.
///
/// * La entrada es la celda `(1, cols + 1)` (U1 si `cols` = 20) y vale 2.
/// * Las dependientes están en la columna `cols + 2` (V si `cols` = 20), filas 2..=`formulas` + 1,
///   y cada una es `=$U$1*A{fila}`; su valor es `2 * A{fila}`.
/// * La columna `cols + 3` (W) lleva fórmulas con nombres de función (`SUM`, `IF`, `VLOOKUP`…)
///   para comprobar que al abrir con el idioma `es` se ven en español.
///
/// Hace falta `filas` > `formulas`. Se genera en un directorio temporal; no se versiona.
pub fn crear_a2(dir: &Path, filas: u32, cols: u16, formulas: u32) -> PathBuf {
    assert!(filas > formulas, "filas debe ser mayor que formulas");
    let ruta = dir.join("a2.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let negrita = Format::new().set_bold();
    let fecha = Format::new().set_num_format("yyyy-mm-dd");
    let moneda = Format::new().set_num_format("#,##0.00");
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
                    ws.write_number_with_format(r, c, f64::from(r) * 1.5, &moneda)
                        .unwrap();
                }
            }
        }
    }
    ws.write_number(0, cols, 2).unwrap();
    let entrada = format!("${}$1", columna_a1(cols));
    for r in 1..=formulas {
        ws.write_formula(
            r,
            cols + 1,
            Formula::new(format!("={entrada}*A{}", r + 1)).set_result("0"),
        )
        .unwrap();
    }
    let w = cols + 2;
    ws.write_formula(0, w, Formula::new("=SUM(A2:A11)").set_result("0"))
        .unwrap();
    ws.write_formula(
        1,
        w,
        Formula::new("=IF(A2>0,\"si\",\"no\")").set_result("si"),
    )
    .unwrap();
    ws.write_formula(
        2,
        w,
        Formula::new("=IFERROR(VLOOKUP(7,A2:B11,2,FALSE),0)").set_result("0"),
    )
    .unwrap();
    wb.save(&ruta).expect("guardar a2.xlsx");
    ruta
}

/// Letras de columna de una columna base 0 (0 → `A`, 20 → `U`, 26 → `AA`).
pub fn columna_a1(mut col: u16) -> String {
    let mut letras = Vec::new();
    loop {
        letras.push(b'A' + (col % 26) as u8);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    letras.reverse();
    String::from_utf8(letras).unwrap()
}

/// Mediana de una lista de tiempos en milisegundos.
pub fn mediana(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Redondea a dos decimales (para los JSON de medición).
pub fn redondear(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Fusiona `claves` en el JSON de medición `results/dev/<nombre>.json` (lo crea si no existe).
/// Solo se llama desde pruebas `--release`; con `debug` no se mide ni se guarda.
pub fn registrar_medicion(nombre: &str, claves: serde_json::Value) {
    use std::sync::Mutex;
    static CANDADO: Mutex<()> = Mutex::new(());
    let _guardia = CANDADO.lock().unwrap_or_else(|e| e.into_inner());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../results/dev");
    std::fs::create_dir_all(&dir).expect("crear results/dev");
    let ruta = dir.join(format!("{nombre}.json"));
    let mut raiz: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&ruta)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if let serde_json::Value::Object(nuevas) = claves {
        raiz.extend(nuevas);
    }
    std::fs::write(&ruta, serde_json::to_string_pretty(&raiz).unwrap())
        .expect("escribir la medición");
}
