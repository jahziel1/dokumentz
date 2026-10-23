//! E08 (T024): fórmulas en español e inglés (FR-006, `plan.md` §5.7).
//!
//! * El conjunto fijo del plan (`SUMA/SUM`, `BUSCARV/VLOOKUP`, `BUSCARX/XLOOKUP`,
//!   `SI.ERROR/IFERROR`, `CONTAR.SI/COUNTIF`, `SI/IF`) da el mismo valor tecleado en cualquiera
//!   de los dos idiomas.
//! * Una función inexistente deja el error y conserva el texto; el historial neto es una
//!   entrada y Rehacer funciona; el idioma activo no se toca.
//! * A2 se abre en español. `SetLang` cambia cómo se ven las fórmulas guardadas.
//! * La tabla región → `locale` (`locale_for_region`) y su efecto al teclear.
//! * El defecto de IronCalc 0.9.1 que contamina fórmulas con el mismo texto interno, y por qué
//!   se descartó el reintento con `undo` (rompe Rehacer).
//! * `medicion_t024_*` (solo `--release`, `#[ignore]` en `debug`): tiempos de una edición normal,
//!   de teclear una fórmula en el otro idioma con (A) reintento con `undo` (con y sin pausa) y
//!   con (B) traducción, sobre un A2 sintético de 100 000 × 20 con 10 000 fórmulas. Escribe
//!   `results/dev/T024.json`.

mod common;

use std::time::Instant;

use dokumentz_sheet::{Core, Lang, OpenOptions, locale_for_region};
use ironcalc_base::UserModel;
use ironcalc_base::types::CellType;
use serde_json::json;

use common::{crear_a2, mediana, redondear, registrar_medicion};

fn opciones(lang: Lang, locale: &'static str) -> OpenOptions {
    OpenOptions {
        lang,
        locale,
        tz: "UTC",
    }
}

/// Libro nuevo con datos: A1:A3 = 1, 2, 3 y B1:B3 = «a», «b», «c».
fn con_datos(lang: Lang, locale: &'static str) -> Core {
    let mut core = Core::new_empty(opciones(lang, locale)).unwrap();
    for (i, (n, t)) in [("1", "a"), ("2", "b"), ("3", "c")].into_iter().enumerate() {
        let fila = i as i32 + 1;
        core.set_cell_input(0, fila, 1, n).unwrap();
        core.set_cell_input(0, fila, 2, t).unwrap();
    }
    core
}

fn valor(core: &Core, fila: i32, col: i32) -> String {
    core.cell_text(0, fila, col).unwrap()
}

fn contenido(core: &Core, fila: i32, col: i32) -> String {
    core.cell_input(0, fila, col).unwrap()
}

/// Pares (español, inglés, valor esperado) del conjunto fijo del plan, con `,` como separador
/// (`locale` `en`).
const PARES: [(&str, &str, &str); 6] = [
    ("=SUMA(A1:A3)", "=SUM(A1:A3)", "6"),
    ("=BUSCARV(2,A1:B3,2,0)", "=VLOOKUP(2,A1:B3,2,0)", "b"),
    ("=BUSCARX(2,A1:A3,B1:B3)", "=XLOOKUP(2,A1:A3,B1:B3)", "b"),
    ("=SI.ERROR(1/0,\"x\")", "=IFERROR(1/0,\"x\")", "x"),
    ("=CONTAR.SI(A1:A3,\">1\")", "=COUNTIF(A1:A3,\">1\")", "2"),
    ("=SI(A1>0,\"si\",\"no\")", "=IF(A1>0,\"si\",\"no\")", "si"),
];

// ----------------------------------------------------------------------------------------
// El conjunto fijo, en ambos idiomas y con todas las estrategias
// ----------------------------------------------------------------------------------------

fn comprobar_pares(lang: Lang) {
    let mut core = con_datos(lang, "en");
    for (i, (es, en, esperado)) in PARES.into_iter().enumerate() {
        let fila = 5 + i as i32;
        core.set_cell_input(0, fila, 1, es).unwrap();
        core.set_cell_input(0, fila, 2, en).unwrap();
        assert_eq!(valor(&core, fila, 1), esperado, "{es} en {lang:?}");
        assert_eq!(valor(&core, fila, 2), esperado, "{en} en {lang:?}");
        // La fórmula se guarda en inglés y se muestra en el idioma activo: las dos celdas
        // enseñan exactamente el mismo texto, el del idioma activo.
        let esperado_texto = if lang == Lang::Es { es } else { en };
        assert_eq!(contenido(&core, fila, 1), esperado_texto);
        assert_eq!(contenido(&core, fila, 2), esperado_texto);
    }
    assert_eq!(core.lang(), lang, "el idioma activo no se toca");
}

#[test]
fn el_conjunto_fijo_da_el_mismo_valor_en_espanol_e_ingles_en_sesion_es() {
    comprobar_pares(Lang::Es);
}

#[test]
fn el_conjunto_fijo_da_el_mismo_valor_en_espanol_e_ingles_en_sesion_en() {
    comprobar_pares(Lang::En);
}

#[test]
fn con_locale_es_el_conjunto_fijo_usa_punto_y_coma() {
    // España: argumentos con `;` y coma decimal. Mismas funciones, otro convenio de teclado.
    let mut core = con_datos(Lang::Es, "es");
    for texto in ["=SUMA(A1:A3;0,5)", "=SUM(A1:A3;0,5)"] {
        core.set_cell_input(0, 5, 1, texto).unwrap();
        assert_eq!(valor(&core, 5, 1), "6,5", "{texto}");
        assert_eq!(contenido(&core, 5, 1), "=SUMA(A1:A3;0,5)");
    }
}

// ----------------------------------------------------------------------------------------
// Historial neto, función inexistente
// ----------------------------------------------------------------------------------------

/// Teclea `texto` en D1 y comprueba que el historial neto es **una** entrada: un `undo` deja
/// D1 vacía sin tocar los datos, y `redo` la devuelve con su valor.
fn comprobar_una_entrada(lang: Lang, texto: &str, esperado: &str) {
    let mut core = con_datos(lang, "en");
    core.set_cell_input(0, 1, 4, texto).unwrap();
    assert_eq!(valor(&core, 1, 4), esperado, "{texto}");
    assert!(!core.can_redo(), "no queda nada que rehacer");
    core.undo().unwrap();
    assert_eq!(contenido(&core, 1, 4), "", "un solo undo quita la fórmula");
    assert_eq!(valor(&core, 3, 1), "3", "y no toca la entrada anterior");
    assert!(core.can_redo());
    core.redo().unwrap();
    assert_eq!(valor(&core, 1, 4), esperado, "redo la devuelve");
    assert_eq!(core.lang(), lang);
}

#[test]
fn teclear_en_el_otro_idioma_deja_una_sola_entrada_de_historial() {
    comprobar_una_entrada(Lang::Es, "=SUM(A1:A3)", "6");
    comprobar_una_entrada(Lang::En, "=SUMA(A1:A3)", "6");
    // Y el texto de la entrada queda en el idioma activo: Rehacer funciona.
    let mut core = con_datos(Lang::Es, "en");
    core.set_cell_input(0, 1, 4, "=IFERROR(VLOOKUP(2,A1:B3,2,0),\"x\")")
        .unwrap();
    assert_eq!(
        contenido(&core, 1, 4),
        "=SI.ERROR(BUSCARV(2,A1:B3,2,0),\"x\")"
    );
    core.undo().unwrap();
    core.redo().unwrap();
    assert_eq!(valor(&core, 1, 4), "b");
}

#[test]
fn una_funcion_inexistente_deja_el_error_y_conserva_el_texto() {
    for (lang, error) in [(Lang::Es, "#¿NOMBRE?"), (Lang::En, "#NAME?")] {
        let mut core = con_datos(lang, "en");
        core.set_cell_input(0, 1, 4, "=NOEXISTE(A1:A3)").unwrap();
        assert_eq!(valor(&core, 1, 4), error, "{lang:?}");
        // El texto no se pierde (el nombre desconocido queda en minúsculas) y se corrige.
        assert_eq!(contenido(&core, 1, 4), "=noexiste(A1:A3)");
        assert_eq!(core.lang(), lang);
        // Una sola entrada de historial.
        core.undo().unwrap();
        assert_eq!(contenido(&core, 1, 4), "");
        assert_eq!(valor(&core, 3, 1), "3");
        core.redo().unwrap();
        assert_eq!(valor(&core, 1, 4), error);
        // Y se puede corregir tecleando la función buena.
        core.set_cell_input(0, 1, 4, "=SUMA(A1:A3)").unwrap();
        assert_eq!(valor(&core, 1, 4), "6");
    }
}

#[test]
fn una_mezcla_de_nombres_conocidos_y_desconocidos_conserva_el_texto() {
    // `SUMA` se entiende en español; `FOO` no existe en ningún idioma: no se traduce nada.
    let mut core = con_datos(Lang::Es, "en");
    core.set_cell_input(0, 1, 4, "=SUMA(A1:A3)+FOO(1)").unwrap();
    assert_eq!(valor(&core, 1, 4), "#¿NOMBRE?");
    assert_eq!(contenido(&core, 1, 4), "=SUMA(A1:A3)+foo(1)");
}

#[test]
fn las_formulas_que_no_empiezan_por_igual_no_pasan_por_la_politica_de_idioma() {
    let mut core = con_datos(Lang::Es, "en");
    core.set_cell_input(0, 1, 4, "SUM(A1:A3)").unwrap();
    assert_eq!(valor(&core, 1, 4), "SUM(A1:A3)", "es texto");
    core.set_cell_input(0, 2, 4, "=A1+A2").unwrap();
    assert_eq!(valor(&core, 2, 4), "3");
}

#[test]
fn la_formula_valida_en_el_idioma_activo_no_se_traduce_ni_se_reintenta() {
    // Un nombre que solo existe en el idioma activo: no se toca (una sola evaluación).
    let mut core = con_datos(Lang::Es, "en");
    core.set_cell_input(0, 1, 4, "=SUMA(A1:A3)").unwrap();
    assert_eq!(contenido(&core, 1, 4), "=SUMA(A1:A3)");
    // Y un error de verdad (no de nombre) tampoco dispara nada raro.
    core.set_cell_input(0, 2, 4, "=1/0").unwrap();
    assert_eq!(
        valor(&core, 2, 4),
        "#¡DIV/0!",
        "los errores se rotulan en el idioma activo"
    );
    assert_eq!(contenido(&core, 2, 4), "=1/0");
}

#[test]
fn las_constantes_logicas_en_ingles_se_aceptan_en_una_sesion_en_espanol() {
    // Con IronCalc 0.9.1, `TRUE`/`FALSE` sueltos en una sesión `es` dan `#ERROR!` (no error de
    // nombre): la traducción lo reconoce; el reintento con `undo` (solo ante el error de
    // nombre) no lo habría hecho.
    let mut core = con_datos(Lang::Es, "en");
    core.set_cell_input(0, 1, 4, "=IF(A1>0,TRUE,FALSE)")
        .unwrap();
    assert_eq!(valor(&core, 1, 4), "VERDADERO");
    assert_eq!(contenido(&core, 1, 4), "=SI(A1>0,VERDADERO,FALSO)");
}

#[test]
fn una_formula_con_un_nombre_definido_del_libro_tambien_se_traduce() {
    // `Ventas` es un nombre del libro, no una función: no debe confundirse con un nombre del
    // otro idioma, y la traducción no puede quedar bloqueada por él.
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("nombres.xlsx");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    for (i, n) in [4, 5, 6].into_iter().enumerate() {
        ws.write_number(i as u32, 0, n).unwrap();
    }
    wb.define_name("Ventas", "=Sheet1!$A$1:$A$3").unwrap();
    wb.save(&ruta).unwrap();
    let mut core = Core::open_xlsx(&ruta, opciones(Lang::Es, "en")).unwrap();
    core.recalculate();
    core.set_cell_input(0, 1, 3, "=SUMA(Ventas)").unwrap();
    core.set_cell_input(0, 2, 3, "=SUM(Ventas)").unwrap();
    core.set_cell_input(0, 3, 3, "=MAX(Ventas)+SUM(Ventas)")
        .unwrap();
    assert_eq!(valor(&core, 1, 3), "15");
    assert_eq!(
        valor(&core, 2, 3),
        "15",
        "SUM se traduce aunque haya un nombre definido"
    );
    assert_eq!(contenido(&core, 2, 3), "=SUMA(Ventas)");
    assert_eq!(valor(&core, 3, 3), "21", "MAX existe en los dos idiomas");
    // Una mezcla de los dos idiomas no se traduce a medias: la parte válida no se estropea
    // (sigue en mayúsculas) y la otra queda como error de nombre.
    core.set_cell_input(0, 4, 3, "=SUMA(Ventas)+SUM(Ventas)")
        .unwrap();
    assert_eq!(valor(&core, 4, 3), "#¿NOMBRE?");
    assert_eq!(contenido(&core, 4, 3), "=SUMA(Ventas)+sum(Ventas)");
}

// ----------------------------------------------------------------------------------------
// SetLang y A2 en español
// ----------------------------------------------------------------------------------------

#[test]
fn set_lang_cambia_como_se_ven_las_formulas_guardadas() {
    let mut core = con_datos(Lang::En, "en");
    core.set_cell_input(0, 1, 4, "=SUM(A1:A3)").unwrap();
    core.set_cell_input(0, 2, 4, "=1/0").unwrap();
    let version = core.data_version();
    let sucio_antes = core.is_dirty();

    core.set_lang(Lang::Es).unwrap();
    assert_eq!(core.lang(), Lang::Es);
    assert_eq!(contenido(&core, 1, 4), "=SUMA(A1:A3)");
    assert_eq!(valor(&core, 1, 4), "6", "el valor no cambia");
    assert!(
        core.data_version() > version,
        "los textos de error cambian de idioma"
    );
    assert_eq!(
        core.is_dirty(),
        sucio_antes,
        "el idioma no es un cambio del documento"
    );

    // Ahora se teclea en español y también se acepta el inglés.
    core.set_cell_input(0, 3, 4, "=SUMA(A1:A2)").unwrap();
    core.set_cell_input(0, 4, 4, "=SUM(A1:A2)").unwrap();
    assert_eq!(valor(&core, 3, 4), "3");
    assert_eq!(valor(&core, 4, 4), "3");

    core.set_lang(Lang::En).unwrap();
    assert_eq!(contenido(&core, 3, 4), "=SUM(A1:A2)");
    assert_eq!(contenido(&core, 4, 4), "=SUM(A1:A2)");
}

#[test]
fn a2_se_abre_con_los_nombres_de_funcion_en_espanol() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_a2(dir.path(), 50, 20, 10);
    let mut core = Core::open_xlsx(&ruta, opciones(Lang::Es, "en")).unwrap();
    core.recalculate();
    // Columna W (23): las fórmulas con nombres de función.
    assert_eq!(contenido(&core, 1, 23), "=SUMA(A2:A11)");
    assert_eq!(contenido(&core, 2, 23), "=SI(A2>0,\"si\",\"no\")");
    assert_eq!(
        contenido(&core, 3, 23),
        "=SI.ERROR(BUSCARV(7,A2:B11,2,FALSO),0)"
    );
    // Se calculan sin errores.
    for fila in 1..=3 {
        let v = valor(&core, fila, 23);
        assert!(!v.starts_with('#'), "W{fila} = {v}");
    }
    assert_eq!(valor(&core, 2, 23), "si");
    // Las dependientes de U1 también.
    assert_eq!(
        valor(&core, 2, 22),
        (2 * 7).to_string(),
        "V2 = U1 * A2 = 2 * 7"
    );
}

// ----------------------------------------------------------------------------------------
// locale ≠ idioma: la tabla región → locale
// ----------------------------------------------------------------------------------------

#[test]
fn la_tabla_region_locale() {
    let tabla = [
        // El Mac del proyecto: coma para separar argumentos y punto decimal.
        ("es_MX", "en"),
        ("es-MX", "en"),
        ("es_MX.UTF-8", "en"),
        ("es_MX@currency=MXN", "en"),
        ("ES_mx", "en"),
        ("es_US", "en"),
        ("es_PE", "en"),
        ("es_419", "en"),
        // Español con coma decimal.
        ("es_ES", "es"),
        ("es_ES.UTF-8", "es"),
        ("es_AR", "es"),
        ("es_CL", "es"),
        ("es_CO", "es"),
        ("es_UY", "es"),
        ("es_VE", "es"),
        ("es", "es"),
        // Alemán, francés e italiano.
        ("de_DE", "de"),
        ("de_AT", "de"),
        ("de_CH", "en"),
        ("fr_FR", "fr"),
        ("fr_CA", "fr"),
        ("fr_CH", "fr"),
        ("it_IT", "it"),
        ("it_CH", "en"),
        // Inglés.
        ("en_US", "en"),
        ("en_GB", "en-GB"),
        ("en-GB", "en-GB"),
        ("en_AU", "en"),
        ("en", "en"),
        // Otros idiomas con coma decimal, al locale más cercano que tiene IronCalc.
        ("pt_BR", "de"),
        ("ca_ES", "es"),
        // Punto decimal, vacío o desconocido.
        ("ja_JP", "en"),
        ("zh-Hans-CN", "en"),
        ("", "en"),
        ("xx_YY", "en"),
        ("C", "en"),
        ("POSIX", "en"),
    ];
    for (region, locale) in tabla {
        assert_eq!(locale_for_region(region), locale, "región «{region}»");
    }
}

#[test]
fn todos_los_locales_de_la_tabla_existen_en_ironcalc() {
    let soportados = ironcalc_base::get_supported_locales();
    for region in [
        "es_MX", "es_ES", "de_DE", "fr_FR", "it_IT", "en_US", "en_GB", "pt_BR", "ca_ES", "",
    ] {
        let locale = locale_for_region(region);
        assert!(
            soportados.iter().any(|s| s == locale),
            "IronCalc no tiene el locale «{locale}» (región «{region}»)"
        );
        // Y abre un libro con él.
        Core::new_empty(OpenOptions::from_region(Lang::Es, region)).unwrap();
    }
}

#[test]
fn el_locale_decide_el_separador_y_el_decimal_no_el_idioma() {
    // México: idioma español, locale `en` → `=SUMA(1,2)` y punto decimal.
    let mut mx = Core::new_empty(OpenOptions::from_region(Lang::Es, "es_MX")).unwrap();
    mx.set_cell_input(0, 1, 1, "=SUMA(1,2)").unwrap();
    mx.set_cell_input(0, 2, 1, "1.5").unwrap();
    mx.set_cell_input(0, 3, 1, "=SUM(1,2)").unwrap();
    assert_eq!(valor(&mx, 1, 1), "3");
    assert_eq!(valor(&mx, 2, 1), "1.5");
    assert_eq!(valor(&mx, 3, 1), "3", "y el inglés también se acepta");
    assert_eq!(contenido(&mx, 3, 1), "=SUMA(1,2)");

    // España: argumentos con `;` y coma decimal; `1,5` es un número.
    let mut es = Core::new_empty(OpenOptions::from_region(Lang::Es, "es_ES")).unwrap();
    es.set_cell_input(0, 1, 1, "=SUMA(1;2)").unwrap();
    es.set_cell_input(0, 2, 1, "1,5").unwrap();
    es.set_cell_input(0, 3, 1, "=SUM(1;2)").unwrap();
    assert_eq!(valor(&es, 1, 1), "3");
    assert_eq!(valor(&es, 2, 1), "1,5");
    es.set_cell_input(0, 4, 1, "=A2*2").unwrap();
    assert_eq!(valor(&es, 4, 1), "3", "1,5 se guardó como número");
    assert_eq!(valor(&es, 3, 1), "3");
}

// ----------------------------------------------------------------------------------------
// Defecto de IronCalc 0.9.1 y por qué se descartó el reintento con `undo`
// ----------------------------------------------------------------------------------------

#[test]
fn la_traduccion_no_crea_entradas_fallidas_en_el_libro_real() {
    // locale `es` (España): `,` es el decimal. El texto en inglés con `;` se entiende al
    // traducirlo; en el libro real solo entra la fórmula válida, así que no hay entrada
    // fallida que contamine a las siguientes con el mismo texto interno.
    let mut core = Core::new_empty(opciones(Lang::Es, "es")).unwrap();
    core.set_cell_input(0, 1, 1, "=IF(1>0;2;3)").unwrap();
    core.set_cell_input(0, 2, 1, "=SI(1>0;2;3)").unwrap();
    core.set_cell_input(0, 3, 1, "=IF(1>0;2;3)").unwrap();
    for fila in 1..=3 {
        assert_eq!(valor(&core, fila, 1), "2", "fila {fila}");
        assert_eq!(contenido(&core, fila, 1), "=SI(1>0;2;3)");
    }
}

#[test]
fn defecto_de_0_9_1_un_texto_que_no_se_analiza_contamina_al_valido_con_el_mismo_texto_interno() {
    // locale `es`: `,` es el decimal, así que `=IF(1>0,2,3)` no se puede analizar y IronCalc
    // guarda su texto tal cual. `=SI(1>0;2;3)` tiene el mismo texto interno (`IF(1>0,2,3)`) y
    // reutiliza esa entrada fallida: sale con `#ERROR!`. Ni los paréntesis ni el `+` unario lo
    // esquivan. El núcleo no puede corregirlo (la tabla de fórmulas no es accesible) y esta
    // aserción documenta el defecto: si una versión posterior de IronCalc lo arregla, falla y
    // hay que revisar `edit.rs` (módulo) y `plan.md` §5.7.
    let mut core = Core::new_empty(opciones(Lang::Es, "es")).unwrap();
    core.set_cell_input(0, 1, 1, "=IF(1>0,2,3)").unwrap();
    assert_eq!(valor(&core, 1, 1), "#ERROR!");
    assert_eq!(
        contenido(&core, 1, 1),
        "=IF(1>0,2,3)",
        "el texto tecleado se conserva"
    );
    core.set_cell_input(0, 2, 1, "=SI(1>0;2;3)").unwrap();
    assert_eq!(
        valor(&core, 2, 1),
        "#ERROR!",
        "contaminada por la entrada fallida"
    );
    assert_eq!(contenido(&core, 2, 1), "=IF(1>0,2,3)");
    // Con otro texto interno no pasa nada.
    core.set_cell_input(0, 3, 1, "=SI(2>1;2;3)").unwrap();
    assert_eq!(valor(&core, 3, 1), "2");
}

#[test]
fn un_error_de_sintaxis_de_verdad_se_deja_como_se_tecleo() {
    let mut core = Core::new_empty(opciones(Lang::Es, "en")).unwrap();
    core.set_cell_input(0, 1, 1, "=2+*3").unwrap();
    assert_eq!(contenido(&core, 1, 1), "=2+*3");
    assert!(valor(&core, 1, 1).starts_with('#'));
    core.undo().unwrap();
    assert_eq!(contenido(&core, 1, 1), "", "una sola entrada de historial");
}

/// Estrategia (A) del plan (§5.7) sobre el motor en bruto: escribir en el idioma activo y, si
/// queda el error de nombre, `undo`, cambiar de idioma, escribir de nuevo y volver al activo.
/// Con `pausa`, la evaluación se pausa y solo se hace explícita donde hace falta.
fn reintento_con_undo(um: &mut UserModel<'_>, fila: i32, col: i32, texto: &str, pausa: bool) {
    let es_error_de_nombre = |um: &UserModel<'_>| {
        matches!(um.get_cell_type(0, fila, col), Ok(CellType::ErrorValue))
            && um.get_formatted_cell_value(0, fila, col).unwrap() == "#¿NOMBRE?"
    };
    if pausa {
        um.pause_evaluation();
    }
    um.set_user_input(0, fila, col, texto).unwrap();
    if pausa {
        um.evaluate();
    }
    if es_error_de_nombre(um) {
        um.undo().unwrap();
        um.set_language("en").unwrap();
        um.set_user_input(0, fila, col, texto).unwrap();
        um.set_language("es").unwrap();
        if pausa {
            um.evaluate();
        }
    }
    if pausa {
        um.resume_evaluation();
    }
}

#[test]
fn el_reintento_con_undo_rompe_rehacer() {
    // Por qué (A) se descartó: tras el reintento la entrada de historial guarda el texto tecleado
    // en inglés y `redo` lo analiza con el idioma *activo* (español): vuelve el error de nombre.
    // La traducción (B) escribe el texto ya en el idioma activo y su Rehacer funciona (ver
    // `teclear_en_el_otro_idioma_deja_una_sola_entrada_de_historial`).
    for pausa in [false, true] {
        let mut um = UserModel::new_empty("borrador", "en", "UTC", "es").unwrap();
        um.set_user_input(0, 1, 1, "5").unwrap();
        um.set_user_input(0, 2, 1, "7").unwrap();
        reintento_con_undo(&mut um, 3, 1, "=SUM(A1:A2)", pausa);
        assert_eq!(um.get_formatted_cell_value(0, 3, 1).unwrap(), "12");
        um.undo().unwrap();
        assert_eq!(um.get_cell_content(0, 3, 1).unwrap(), "");
        um.redo().unwrap();
        assert_eq!(
            um.get_formatted_cell_value(0, 3, 1).unwrap(),
            "#¿NOMBRE?",
            "Rehacer re-analiza el texto en inglés con el idioma activo (pausa = {pausa})"
        );
    }
}

// ----------------------------------------------------------------------------------------
// Medición (solo --release): ¿cuánto cuesta teclear en el otro idioma sobre A2?
// ----------------------------------------------------------------------------------------

#[test]
#[cfg_attr(debug_assertions, ignore = "medición: solo en --release")]
fn medicion_t024_costes_de_teclear_en_el_otro_idioma_sobre_a2() {
    const REPETICIONES: usize = 7;
    // Celda de prueba: W50 (columna 23); fuera de las dependientes de U1.
    let (fila, col) = (50, 23);
    let ms = |t: Instant| t.elapsed().as_secs_f64() * 1000.0;

    let dir = tempfile::tempdir().unwrap();
    let t = Instant::now();
    let ruta = crear_a2(dir.path(), 100_000, 20, 10_000);
    let generar_ms = ms(t);

    // --- Núcleo: edición normal y (B) traducción ---
    let t = Instant::now();
    let mut core = Core::open_xlsx(&ruta, opciones(Lang::Es, "en")).unwrap();
    let abrir_ms = ms(t);
    core.recalculate();
    core.set_cell_input(0, fila, col, "=SUMA(A2:A11)").unwrap(); // calentamiento
    let (mut normal, mut traduccion, mut u1) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..REPETICIONES {
        // Edición normal: fórmula en el idioma activo (una evaluación completa).
        let t = Instant::now();
        core.set_cell_input(0, fila, col, "=SUMA(A2:A11)").unwrap();
        normal.push(ms(t));
        // Teclear en el otro idioma: el camino real del núcleo.
        let t = Instant::now();
        core.set_cell_input(0, fila, col, "=SUM(A2:A11)").unwrap();
        traduccion.push(ms(t));
        assert_eq!(contenido(&core, fila, col), "=SUMA(A2:A11)");
        // Editar la celda de la que dependen las 10 000 fórmulas (M-05).
        let t = Instant::now();
        core.set_cell_input(0, 1, 21, &format!("{}", 3 + i))
            .unwrap();
        u1.push(ms(t));
    }
    assert_eq!(
        valor(&core, 2, 22),
        (7 * (3 + REPETICIONES - 1)).to_string(),
        "V2 = U1 * A2"
    );
    drop(core);

    // --- Motor en bruto: (A) reintento con undo, con y sin pausa ---
    let modelo = ironcalc::import::load_from_xlsx(ruta.to_str().unwrap(), "en", "UTC", "es")
        .expect("abrir A2");
    let mut um = UserModel::from_model(modelo);
    um.evaluate();
    reintento_con_undo(&mut um, fila, col, "=SUM(A2:A11)", false); // calentamiento
    let (mut normal_bruto, mut reintento, mut reintento_pausa) =
        (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..REPETICIONES {
        let t = Instant::now();
        um.set_user_input(0, fila, col, "=SUMA(A2:A11)").unwrap();
        normal_bruto.push(ms(t));
        let t = Instant::now();
        reintento_con_undo(&mut um, fila, col, "=SUM(A2:A11)", false);
        reintento.push(ms(t));
        let t = Instant::now();
        reintento_con_undo(&mut um, fila, col, "=SUM(A2:A11)", true);
        reintento_pausa.push(ms(t));
    }
    assert_eq!(um.get_cell_content(0, fila, col).unwrap(), "=SUMA(A2:A11)");

    let (normal_ms, normal_bruto_ms) = (mediana(normal.clone()), mediana(normal_bruto.clone()));
    let serie = |v: &[f64], base: f64| {
        let m = mediana(v.to_vec());
        json!({"mediana_ms": redondear(m), "razon_sobre_normal": redondear(m / base),
               "muestras_ms": v.iter().map(|x| redondear(*x)).collect::<Vec<_>>()})
    };
    let (m_trad, m_a, m_ap) = (
        mediana(traduccion.clone()),
        mediana(reintento.clone()),
        mediana(reintento_pausa.clone()),
    );
    println!(
        "T024 — edición normal {normal_ms:.1} ms (núcleo) / {normal_bruto_ms:.1} ms (motor en bruto); \
         (A) reintento {m_a:.1} ms = {:.2}x; (A) con pausa {m_ap:.1} ms = {:.2}x; \
         (B) traducción {m_trad:.1} ms = {:.2}x; editar U1 {:.1} ms",
        m_a / normal_bruto_ms,
        m_ap / normal_bruto_ms,
        m_trad / normal_ms,
        mediana(u1.clone()),
    );
    registrar_medicion(
        "T024",
        json!({
            "libro": {"filas": 100_000, "columnas": 20, "formulas_dependientes": 10_000,
                      "generar_ms": redondear(generar_ms), "abrir_ms": redondear(abrir_ms),
                      "idioma": "es", "locale": "en"},
            "edicion_normal_nucleo": serie(&normal, normal_ms),
            "edicion_normal_motor_en_bruto": serie(&normal_bruto, normal_bruto_ms),
            "A_reintento_con_undo": serie(&reintento, normal_bruto_ms),
            "A_reintento_con_undo_y_pausa": serie(&reintento_pausa, normal_bruto_ms),
            "B_traduccion": serie(&traduccion, normal_ms),
            "editar_u1_con_10000_dependientes": serie(&u1, normal_ms),
            "criterio": "plan.md §5.7: (A) si teclear en el otro idioma cuesta <= 2x una edición normal; si no, (B)",
            "elegida": "B (traduccion): (A) rompe Rehacer, ver la prueba el_reintento_con_undo_rompe_rehacer",
        }),
    );

    // Lo que usa el camino normal cumple la regla de plan.md §5.7.
    assert!(
        m_trad <= 2.0 * normal_ms,
        "teclear en el otro idioma cuesta {m_trad:.1} ms, más de 2x una edición normal ({normal_ms:.1} ms)"
    );
}
