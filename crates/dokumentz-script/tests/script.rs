//! Lectura, validación e ida y vuelta de `Script` (T014).

use dokumentz_script::cell::{normalize_cell, normalize_range, parse_cell};
use dokumentz_script::{Modifier, Pos, Script, ScriptError, Step, StepKind};

fn fixture(nombre: &str) -> String {
    format!("{}/tests/fixtures/{nombre}", env!("CARGO_MANIFEST_DIR"))
}

fn uno(json: &str) -> Step {
    let texto = format!(r#"{{"steps":[{json}]}}"#);
    Script::from_json_str(&texto)
        .expect("paso válido")
        .steps
        .remove(0)
}

fn invalido(json: &str) -> String {
    let texto = format!(r#"{{"steps":[{json}]}}"#);
    Script::from_json_str(&texto)
        .expect_err("debía rechazarlo")
        .to_string()
}

#[test]
fn el_guion_de_ejemplo_se_lee_y_es_valido() {
    let s = Script::from_path(fixture("script_ok.json")).unwrap();
    assert_eq!(s.name.as_deref(), Some("nav-basico"));
    assert_eq!(s.steps.len(), 18);
    assert_eq!(s.steps.last(), Some(&Step::Quit {}));
}

#[test]
fn los_trece_pasos_se_leen_con_sus_valores_por_defecto() {
    assert_eq!(
        uno(r#"{"step":"open","path":"a.xlsx"}"#),
        Step::Open {
            path: "a.xlsx".into()
        }
    );
    assert_eq!(
        uno(r#"{"step":"key","key":"Enter"}"#),
        Step::Key {
            key: "Enter".into(),
            mods: vec![],
            repeat: 1
        }
    );
    assert_eq!(
        uno(r#"{"step":"key","key":"a","mods":["command","shift"],"repeat":3}"#),
        Step::Key {
            key: "a".into(),
            mods: vec![Modifier::Command, Modifier::Shift],
            repeat: 3
        }
    );
    assert_eq!(
        uno(r#"{"step":"text","text":"x"}"#),
        Step::Text { text: "x".into() }
    );
    assert_eq!(
        uno(r#"{"step":"scroll","dy":120}"#),
        Step::Scroll {
            dx: 0.0,
            dy: 120.0,
            count: 1,
            every_ms: 0
        }
    );
    assert_eq!(
        uno(r#"{"step":"drag","from":{"x":10,"y":20.5},"to":{"cell":"b2"}}"#),
        Step::Drag {
            from: Pos::Point { x: 10.0, y: 20.5 },
            to: Pos::Cell { cell: "b2".into() },
            duration_ms: 0
        }
    );
    assert_eq!(uno(r#"{"step":"wait_ms","ms":5}"#), Step::WaitMs { ms: 5 });
    assert_eq!(
        uno(r#"{"step":"wait_idle"}"#),
        Step::WaitIdle { timeout_ms: 10_000 }
    );
    assert_eq!(
        uno(r#"{"step":"mark","name":"m"}"#),
        Step::Mark { name: "m".into() }
    );
    assert_eq!(
        uno(r#"{"step":"assert_cell","cell":"A1","value":"1"}"#),
        Step::AssertCell {
            cell: "A1".into(),
            sheet: None,
            value: Some("1".into()),
            formula: None
        }
    );
    assert_eq!(
        uno(r#"{"step":"assert_selection","range":"A1"}"#),
        Step::AssertSelection {
            range: "A1".into(),
            active: None
        }
    );
    assert_eq!(
        uno(r#"{"step":"assert_formula_bar","text":"=1+1"}"#),
        Step::AssertFormulaBar {
            text: "=1+1".into()
        }
    );
    assert_eq!(uno(r#"{"step":"save"}"#), Step::Save { path: None });
    assert_eq!(uno(r#"{"step":"quit"}"#), Step::Quit {});
}

#[test]
fn cada_paso_informa_su_clase() {
    let clases: Vec<StepKind> = Script::from_path(fixture("script_ok.json"))
        .unwrap()
        .steps
        .iter()
        .map(Step::kind)
        .collect();
    assert_eq!(clases[0], StepKind::Open);
    assert_eq!(clases[3], StepKind::AssertCell);
    assert!(clases[3].is_assert() && !clases[4].is_assert());
    assert_eq!(StepKind::AssertFormulaBar.to_string(), "assert_formula_bar");
    assert_eq!(StepKind::WaitMs.to_string(), "wait_ms");
}

#[test]
fn ida_y_vuelta_conserva_el_guion() {
    let s = Script::from_path(fixture("script_ok.json")).unwrap();
    let json = s.to_json_string().unwrap();
    assert!(json.ends_with("}\n"));
    assert_eq!(Script::from_json_str(&json).unwrap(), s);
    // Los valores por defecto no se escriben.
    assert!(!json.contains("\"repeat\": 1"));
    assert!(!json.contains("\"every_ms\": 0"));
}

#[test]
fn rechaza_claves_y_pasos_desconocidos() {
    assert!(Script::from_json_str(r#"{"steps":[{"step":"salta"}]}"#).is_err());
    assert!(Script::from_json_str(r#"{"steps":[{"step":"quit","extra":1}]}"#).is_err());
    assert!(Script::from_json_str(r#"{"steps":[],"otro":1}"#).is_err());
    assert!(
        Script::from_json_str(r#"{"steps":[{"step":"key","key":"a","mods":["hyper"]}]}"#).is_err()
    );
    assert!(Script::from_json_str("no es json").is_err());
}

#[test]
fn rechaza_un_esquema_distinto() {
    let e = Script::from_json_str(r#"{"schema":"otro/2","steps":[]}"#).unwrap_err();
    assert!(matches!(e, ScriptError::Schema { .. }), "{e}");
}

#[test]
fn valida_celdas_rangos_y_topes() {
    assert!(invalido(r#"{"step":"assert_cell","cell":"A0","value":"x"}"#).contains("paso 0"));
    assert!(invalido(r#"{"step":"assert_cell","cell":"XFE1","value":"x"}"#).contains("celda"));
    assert!(invalido(r#"{"step":"assert_cell","cell":"A1"}"#).contains("necesita"));
    assert!(invalido(r#"{"step":"assert_selection","range":"A1:B2:C3"}"#).contains("rango"));
    assert!(
        invalido(r#"{"step":"assert_selection","range":"A1","active":"1A"}"#).contains("celda")
    );
    assert!(
        invalido(r#"{"step":"drag","from":{"cell":"ZZZZ1"},"to":{"x":0,"y":0}}"#).contains("celda")
    );
    assert!(invalido(r#"{"step":"key","key":""}"#).contains("vacío"));
    assert!(invalido(r#"{"step":"key","key":"a","repeat":0}"#).contains("repeat"));
    assert!(invalido(r#"{"step":"scroll","dy":1,"count":0}"#).contains("count"));
    assert!(invalido(r#"{"step":"wait_ms","ms":99999999}"#).contains("ms"));
    assert!(invalido(r#"{"step":"open","path":""}"#).contains("vacío"));
    assert!(invalido(r#"{"step":"text","text":""}"#).contains("vacío"));
    assert!(invalido(r#"{"step":"mark","name":""}"#).contains("vacío"));
    assert!(invalido(r#"{"step":"save","path":""}"#).contains("vacío"));
}

#[test]
fn quit_solo_puede_ir_al_final() {
    let e = Script::from_json_str(r#"{"steps":[{"step":"quit"},{"step":"wait_ms","ms":1}]}"#)
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "paso 0: `quit` solo puede ser el último paso"
    );
}

#[test]
fn el_error_dice_el_indice_del_paso() {
    let e = Script::from_json_str(
        r#"{"steps":[{"step":"wait_ms","ms":1},{"step":"wait_ms","ms":1},{"step":"key","key":""}]}"#,
    )
    .unwrap_err();
    assert!(matches!(e, ScriptError::Invalid { step: 2, .. }), "{e}");
}

#[test]
fn un_archivo_inexistente_es_un_error_de_lectura() {
    let e = Script::from_path(fixture("no_existe.json")).unwrap_err();
    assert!(matches!(e, ScriptError::Io { .. }), "{e}");
}

#[test]
fn direcciones_a1() {
    assert_eq!(parse_cell("A1"), Some((1, 1)));
    assert_eq!(parse_cell("$b$2"), Some((2, 2)));
    assert_eq!(parse_cell("AA10"), Some((27, 10)));
    assert_eq!(parse_cell("XFD1048576"), Some((16_384, 1_048_576)));
    for malo in [
        "", "A", "1", "A0", "A01", "XFE1", "A1048577", "A-1", "A 1", "1A", "AÑ1",
    ] {
        assert_eq!(parse_cell(malo), None, "{malo:?}");
    }
    assert_eq!(normalize_cell("$ab$12").as_deref(), Some("AB12"));
    assert_eq!(normalize_range("d5:b2").as_deref(), Some("B2:D5"));
    assert_eq!(normalize_range("b2:b2").as_deref(), Some("B2"));
    assert_eq!(normalize_range("c3").as_deref(), Some("C3"));
    assert_eq!(normalize_range("A1:B2:C3"), None);
    assert_eq!(normalize_range("A1:"), None);
}
