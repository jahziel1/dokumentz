//! `check` y el binario `dokumentz-script-check` (T014).

use std::process::Command;

use dokumentz_script::result::{Observed, StepResult};
use dokumentz_script::{Script, ScriptResult, StepKind, check};

fn fixture(nombre: &str) -> String {
    format!("{}/tests/fixtures/{nombre}", env!("CARGO_MANIFEST_DIR"))
}

fn ok() -> (Script, ScriptResult) {
    (
        Script::from_path(fixture("script_ok.json")).unwrap(),
        ScriptResult::from_path(fixture("result_ok.json")).unwrap(),
    )
}

fn salida(esperado: &str, resultado: &str) -> (Option<i32>, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_dokumentz-script-check"))
        .args([fixture(esperado), fixture(resultado)])
        .output()
        .expect("no se pudo ejecutar dokumentz-script-check");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

// ---- binario -------------------------------------------------------------------------------

#[test]
fn el_binario_sale_con_0_si_coincide() {
    let (codigo, out, _) = salida("script_ok.json", "result_ok.json");
    assert_eq!(codigo, Some(0));
    assert!(out.contains("OK: 5 comprobaciones"), "{out}");
}

#[test]
fn el_binario_sale_con_1_si_difiere_y_dice_donde() {
    let (codigo, _, err) = salida("script_ok.json", "result_bad.json");
    assert_eq!(codigo, Some(1));
    assert!(err.contains("paso 9") && err.contains("adiós"), "{err}");
    assert!(err.contains("paso 12") && err.contains("B2:D4"), "{err}");
    assert!(err.contains("paso 13"), "{err}");
    assert!(err.contains("3 diferencias"), "{err}");
}

#[test]
fn el_binario_sale_con_2_si_no_puede_leer() {
    let (codigo, _, err) = salida("script_ok.json", "no_existe.json");
    assert_eq!(codigo, Some(2));
    assert!(err.contains("resultado"), "{err}");
    // También con argumentos de menos.
    let o = Command::new(env!("CARGO_BIN_EXE_dokumentz-script-check"))
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
}

// ---- biblioteca ----------------------------------------------------------------------------

#[test]
fn coincide_el_resultado_de_ejemplo() {
    let (s, r) = ok();
    let informe = check(&s, &r);
    assert!(informe.is_ok(), "{:?}", informe.mismatches);
    assert_eq!(informe.compared, 5);
}

#[test]
fn no_se_fia_del_ok_de_la_interfaz() {
    // El resultado dice ok pero lo observado es distinto: gana lo observado.
    let (s, mut r) = ok();
    let Some(Observed::Cell { value, .. }) = r.steps[9].observed.as_mut() else {
        panic!()
    };
    *value = "otro".to_string();
    assert!(r.steps[9].ok && r.ok);
    let informe = check(&s, &r);
    assert_eq!(informe.mismatches.len(), 1);
    assert_eq!(informe.mismatches[0].step, Some(9));
}

#[test]
fn un_paso_fallido_o_un_guion_abortado_no_pasa() {
    let (s, mut r) = ok();
    r.steps[0] = StepResult::failed(0, StepKind::Open, "no existe");
    r.ok = false;
    let m = check(&s, &r).mismatches;
    assert!(
        m.iter()
            .any(|d| d.step.is_none() && d.message.contains("no terminó bien")),
        "{m:?}"
    );
    assert!(
        m.iter()
            .any(|d| d.step == Some(0) && d.message.contains("no existe")),
        "{m:?}"
    );

    let (s, mut r) = ok();
    r.abort("se agotó wait_idle");
    assert!(
        check(&s, &r).mismatches[0]
            .message
            .contains("se agotó wait_idle")
    );
}

#[test]
fn un_resultado_truncado_no_pasa() {
    let (s, mut r) = ok();
    r.steps.truncate(10); // la interfaz se detuvo antes de terminar
    let m = check(&s, &r).mismatches;
    assert!(
        m.iter()
            .any(|d| d.message.contains("tiene 18 pasos y el resultado 10")),
        "{m:?}"
    );
    assert!(
        m.iter()
            .any(|d| d.message.contains("5 comprobaciones y el resultado 3")),
        "{m:?}"
    );
}

#[test]
fn las_clases_de_paso_deben_coincidir() {
    let (s, mut r) = ok();
    r.steps[4].step = StepKind::Text;
    let m = check(&s, &r).mismatches;
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].message.contains("se esperaba `key`"), "{m:?}");
}

#[test]
fn compara_rangos_en_forma_canonica_y_celdas_sin_distinguir_caja() {
    let (s, mut r) = ok();
    r.steps[6].observed = Some(Observed::Selection {
        range: "b3:a3".to_string(), // esquinas al revés
        active: Some("$A$3".to_string()),
    });
    r.steps[12].observed = Some(Observed::Selection {
        range: "B2:D5".to_string(),
        active: Some("b2".to_string()),
    });
    assert!(check(&s, &r).is_ok());
}

#[test]
fn la_celda_activa_y_la_formula_se_comparan_si_el_esperado_las_da() {
    let (s, mut r) = ok();
    r.steps[6].observed = Some(Observed::Selection {
        range: "A3:B3".to_string(),
        active: Some("B3".to_string()),
    });
    let Some(Observed::Cell { formula, .. }) = r.steps[9].observed.as_mut() else {
        panic!()
    };
    *formula = None;
    let m = check(&s, &r).mismatches;
    assert_eq!(m.len(), 2, "{m:?}");
}

#[test]
fn un_assert_sin_valor_observado_falla() {
    let (s, mut r) = ok();
    r.steps[13].observed = None;
    let m = check(&s, &r).mismatches;
    assert!(m[0].message.contains("no trae el valor observado"), "{m:?}");

    let (s, mut r) = ok();
    r.steps[13].observed = Some(Observed::Selection {
        range: "A1".into(),
        active: None,
    });
    assert!(
        check(&s, &r).mismatches[0]
            .message
            .contains("no corresponde")
    );
}

#[test]
fn el_esperado_reducido_a_asserts_se_compara_por_orden() {
    let (s, r) = ok();
    let reducido = Script::new(
        s.steps
            .iter()
            .filter(|p| p.kind().is_assert())
            .cloned()
            .collect(),
    );
    assert_eq!(reducido.steps.len(), 5);
    let informe = check(&reducido, &r);
    assert!(informe.is_ok(), "{:?}", informe.mismatches);

    // Si el resultado trae una comprobación de más, se nota.
    let mut r2 = r.clone();
    r2.push(StepResult::observed(
        99,
        StepKind::AssertFormulaBar,
        Observed::FormulaBar { text: "x".into() },
    ));
    assert!(!check(&reducido, &r2).is_ok());
}

#[test]
fn same_outcome_ignora_solo_la_aplicacion() {
    let (_, a) = ok();
    let mut b = a.clone();
    b.app = Some("dokumentz-app-tauri".to_string());
    assert!(a.same_outcome(&b));
    let Some(Observed::FormulaBar { text }) = b.steps[13].observed.as_mut() else {
        panic!()
    };
    text.push('!');
    assert!(!a.same_outcome(&b));
}

#[test]
fn el_resultado_va_y_vuelve_por_json_y_por_archivo() {
    let (_, r) = ok();
    let json = r.to_json_string().unwrap();
    assert_eq!(ScriptResult::from_json_str(&json).unwrap(), r);

    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("a").join("b").join("out.json");
    r.write_to(&ruta).unwrap();
    assert_eq!(ScriptResult::from_path(&ruta).unwrap(), r);
}

#[test]
fn construccion_de_resultados() {
    let mut r = ScriptResult::new(Some("g".into()), Some("app".into()));
    r.push(StepResult::done(0, StepKind::Open));
    assert!(r.ok);
    r.push(StepResult::failed(1, StepKind::Save, "disco lleno"));
    assert!(!r.ok);
    assert!(ScriptResult::from_json_str(r#"{"schema":"x","ok":true,"steps":[]}"#).is_err());
    assert!(ScriptResult::from_json_str(r#"{"ok":true,"steps":[],"sobra":1}"#).is_err());
}
