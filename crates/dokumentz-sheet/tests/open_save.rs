//! T015: abrir `.xlsx`, errores tipados sin pánico e ida y vuelta en memoria.
//!
//! Los `.xlsx` se generan en un directorio temporal (`tests/common/mod.rs`); `corrupt` y
//! `truncated` se derivan aquí de `small` en lugar de versionar binarios.

mod common;

use std::io::Write;
use std::path::Path;

use dokumentz_sheet::{Core, CoreError, Lang, OpenOptions};
use rust_xlsxwriter::{Formula, Workbook};

use common::{abrir, crear_empty, crear_small, opciones};

/// Texto de todas las celdas del rango `filas × cols` de la hoja 0.
fn volcado(core: &Core, filas: i32, cols: i32) -> Vec<String> {
    let mut v = Vec::new();
    for f in 1..=filas {
        for c in 1..=cols {
            v.push(core.cell_text(0, f, c).expect("texto de celda"));
        }
    }
    v
}

#[test]
fn abre_small_y_lee_sus_datos() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let core = abrir(&ruta);
    assert_eq!(core.sheet_names(), vec!["Datos".to_string()]);
    assert_eq!(core.path(), Some(ruta.as_path()));
    assert!(!core.is_dirty());
    assert_eq!(core.data_version(), 0);
    // Texto formateado como se ve, no el valor crudo.
    assert_eq!(core.cell_text(0, 1, 1).unwrap(), "Producto");
    assert_eq!(core.cell_text(0, 2, 2).unwrap(), "1,234.50");
    assert_eq!(core.cell_text(0, 2, 3).unwrap(), "2024-03-05");
    assert_eq!(core.cell_text(0, 2, 4).unwrap(), "2469");
    assert_eq!(core.cell_text(0, 3, 4).unwrap(), "TRUE");
    assert_eq!(core.cell_text(0, 6, 2).unwrap(), "7.12");
    // Una celda sin dato es texto vacío, no error.
    assert_eq!(core.cell_text(0, 4, 2).unwrap(), "");
    // Una hoja inexistente sí es error.
    assert!(core.cell_text(7, 1, 1).is_err());
}

#[test]
fn abre_empty() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_empty(dir.path());
    let mut core = abrir(&ruta);
    assert_eq!(core.sheet_names().len(), 1);
    assert_eq!(core.cell_text(0, 1, 1).unwrap(), "");
    let m = core.sheet_metrics();
    assert_eq!((m.used_rows, m.used_cols), (0, 0));
}

#[test]
fn libro_nuevo_vacio() {
    let core = Core::new_empty(opciones()).unwrap();
    assert_eq!(core.sheet_names(), vec!["Sheet1".to_string()]);
    assert_eq!(core.path(), None);
    let es = Core::new_empty(OpenOptions {
        lang: Lang::Es,
        locale: "es",
        tz: "UTC",
    })
    .unwrap();
    assert_eq!(es.sheet_names(), vec!["Hoja1".to_string()]);
    assert_eq!(es.lang(), Lang::Es);
}

#[test]
fn ruta_inexistente_es_no_existe() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("no-hay-nada.xlsx");
    match Core::open_xlsx(&ruta, opciones()) {
        Err(CoreError::NoExiste(p)) => assert_eq!(p, ruta),
        otro => panic!("se esperaba NoExiste y salió {:?}", otro.err()),
    }
    // También cuando falta la carpeta.
    let ruta = dir.path().join("sub/no-hay.xlsx");
    assert!(matches!(
        Core::open_xlsx(&ruta, opciones()),
        Err(CoreError::NoExiste(_))
    ));
}

#[test]
fn una_carpeta_no_es_xlsx() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Core::open_xlsx(dir.path(), opciones()),
        Err(CoreError::NoEsXlsx(_))
    ));
}

#[cfg(unix)]
#[test]
fn archivo_sin_permiso_es_sin_permiso() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&ruta).is_ok() {
        // Se corre como superusuario: el sistema no niega nada. No hay qué comprobar.
        return;
    }
    match Core::open_xlsx(&ruta, opciones()) {
        Err(CoreError::SinPermiso(p)) => assert_eq!(p, ruta),
        otro => panic!("se esperaba SinPermiso y salió {:?}", otro.err()),
    }
    std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn truncado_es_no_es_xlsx_sin_panico() {
    let dir = tempfile::tempdir().unwrap();
    let completo = std::fs::read(crear_small(dir.path())).unwrap();
    // Cortes en distintos puntos: casi vacío, la mitad, y a falta de pocos bytes.
    for corte in [
        0,
        1,
        4,
        30,
        completo.len() / 2,
        completo.len() - 10,
        completo.len() - 1,
    ] {
        let ruta = dir.path().join(format!("truncated-{corte}.xlsx"));
        std::fs::write(&ruta, &completo[..corte]).unwrap();
        match Core::open_xlsx(&ruta, opciones()) {
            Err(CoreError::NoEsXlsx(_)) => {}
            otro => panic!(
                "corte en {corte}: se esperaba NoEsXlsx y salió {:?}",
                otro.err()
            ),
        }
    }
}

#[test]
fn corrupto_es_no_es_xlsx_sin_panico() {
    let dir = tempfile::tempdir().unwrap();
    let completo = std::fs::read(crear_small(dir.path())).unwrap();
    let mut casos: Vec<(&str, Vec<u8>)> = Vec::new();
    casos.push(("texto plano", b"esto no es un libro de calculo".to_vec()));
    // Directorio central destruido (el final del archivo).
    let mut sin_directorio = completo.clone();
    let n = sin_directorio.len();
    sin_directorio[n - 40..].fill(0);
    casos.push(("directorio central a cero", sin_directorio));
    // Un zip válido que no es un libro.
    let mut zip_ajeno = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_ajeno));
        z.start_file("hola.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"hola").unwrap();
        z.finish().unwrap();
    }
    casos.push(("zip sin libro", zip_ajeno));
    // Un libro con el XML de una hoja roto: se escribe un zip copiando todo salvo la hoja.
    casos.push(("hoja con XML roto", con_hoja_rota(&completo)));
    for (nombre, bytes) in casos {
        let ruta = dir.path().join("corrupt.xlsx");
        std::fs::write(&ruta, &bytes).unwrap();
        match Core::open_xlsx(&ruta, opciones()) {
            Err(CoreError::NoEsXlsx(msg)) => assert!(!msg.is_empty(), "{nombre}: mensaje vacío"),
            otro => panic!("{nombre}: se esperaba NoEsXlsx y salió {:?}", otro.err()),
        }
        // Y por el camino de bytes en memoria.
        assert!(
            matches!(
                Core::open_xlsx_bytes(&bytes, "corrupt", opciones()),
                Err(CoreError::NoEsXlsx(_))
            ),
            "{nombre}: en memoria"
        );
    }
}

/// Copia un `.xlsx` reemplazando el contenido de la parte `objetivo` por `nuevo`.
fn reemplazar_parte(xlsx: &[u8], objetivo: &str, nuevo: &[u8]) -> Vec<u8> {
    let mut origen = zip::ZipArchive::new(std::io::Cursor::new(xlsx)).unwrap();
    let mut salida = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut salida));
        for i in 0..origen.len() {
            let mut f = origen.by_index(i).unwrap();
            if f.is_dir() {
                continue;
            }
            let nombre = f.name().expect("nombre de entrada").to_string();
            let mut datos = Vec::new();
            std::io::Read::read_to_end(&mut f, &mut datos).unwrap();
            if nombre == objetivo {
                datos = nuevo.to_vec();
            }
            z.start_file(nombre, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(&datos).unwrap();
        }
        z.finish().unwrap();
    }
    salida
}

fn con_hoja_rota(xlsx: &[u8]) -> Vec<u8> {
    reemplazar_parte(
        xlsx,
        "xl/worksheets/sheet1.xml",
        b"<worksheet><sheetData><row r=\"1\"><c r=\"A1\"",
    )
}

/// Generador congruencial lineal para mutaciones reproducibles (sin dependencias).
struct Lcg(u64);
impl Lcg {
    fn siguiente(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn mutaciones_con_semilla_fija_nunca_entran_en_panico() {
    let dir = tempfile::tempdir().unwrap();
    let original = std::fs::read(crear_small(dir.path())).unwrap();
    let mut lcg = Lcg(0xD0C5);
    let (mut ok, mut err) = (0, 0);
    for _ in 0..400 {
        let mut bytes = original.clone();
        for _ in 0..1 + lcg.siguiente() % 4 {
            let pos = (lcg.siguiente() as usize) % bytes.len();
            bytes[pos] ^= 1 << (lcg.siguiente() % 8);
        }
        // Lo único que se exige es que vuelva (con Ok o con un error tipado).
        match Core::open_xlsx_bytes(&bytes, "mutado", opciones()) {
            Ok(_) => ok += 1,
            Err(_) => err += 1,
        }
    }
    println!("mutaciones: {ok} abren, {err} dan error tipado");
    assert_eq!(ok + err, 400);
    assert!(err > 0, "alguna mutación debería romper el archivo");
}

#[test]
fn locale_inexistente_es_error_del_motor_y_no_panico() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let opts = OpenOptions {
        locale: "xx-inexistente",
        ..opciones()
    };
    assert!(Core::open_xlsx(&ruta, opts).is_err());
    assert!(Core::new_empty(opts).is_err());
}

#[test]
fn abrir_no_evalua_y_recalculate_si() {
    // Decisión de T015: la carga conserva los valores guardados en el archivo (7 aunque
    // A1 + B1 = 5); `recalculate` los pone al día.
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("sin-evaluar.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_number(0, 0, 2).unwrap();
    ws.write_number(0, 1, 3).unwrap();
    ws.write_formula(0, 2, Formula::new("=A1+B1").set_result("7"))
        .unwrap();
    wb.save(&ruta).unwrap();

    let mut core = abrir(&ruta);
    assert_eq!(
        core.cell_text(0, 1, 3).unwrap(),
        "7",
        "abrir no debe evaluar"
    );
    let antes = core.data_version();
    core.recalculate();
    assert_eq!(core.cell_text(0, 1, 3).unwrap(), "5");
    assert!(core.data_version() > antes, "recalcular cambia los valores");
    assert!(
        !core.is_dirty(),
        "recalcular no es una modificación del usuario"
    );
}

#[test]
fn ida_y_vuelta_en_memoria_conserva_el_texto_de_todas_las_celdas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let original = abrir(&ruta);
    let bytes = original.to_xlsx_bytes().unwrap();
    // Un `.xlsx` es un zip.
    assert_eq!(&bytes[..2], b"PK");
    let vuelta = Core::open_xlsx_bytes(&bytes, "vuelta", opciones()).unwrap();
    assert_eq!(vuelta.sheet_names(), original.sheet_names());
    assert_eq!(volcado(&vuelta, 7, 5), volcado(&original, 7, 5));

    // Y por disco: escribir los bytes y abrir por ruta da lo mismo.
    let guardado = dir.path().join("vuelta.xlsx");
    std::fs::write(&guardado, &bytes).unwrap();
    let por_ruta = abrir(&guardado);
    assert_eq!(volcado(&por_ruta, 7, 5), volcado(&original, 7, 5));
}

#[test]
fn open_xlsx_y_open_xlsx_bytes_dan_el_mismo_libro() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let por_ruta = abrir(&ruta);
    let por_bytes =
        Core::open_xlsx_bytes(&std::fs::read(&ruta).unwrap(), "small", opciones()).unwrap();
    assert_eq!(volcado(&por_ruta, 7, 5), volcado(&por_bytes, 7, 5));
    assert_eq!(por_bytes.path(), None);
}

#[test]
fn ruta_con_espacios_y_acentos() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("le document ñandú");
    std::fs::create_dir(&sub).unwrap();
    let ruta = crear_small(&sub);
    let nueva = sub.join("año 2024 — ventas.xlsx");
    std::fs::rename(&ruta, &nueva).unwrap();
    assert!(Path::new(&nueva).exists());
    let core = abrir(&nueva);
    assert_eq!(core.cell_text(0, 1, 1).unwrap(), "Producto");
}

#[test]
fn escribir_una_celda_marca_modificado_y_sube_la_version() {
    let mut core = Core::new_empty(opciones()).unwrap();
    assert!(!core.is_dirty());
    let v0 = core.data_version();
    core.set_cell_input(0, 1, 1, "=1+2").unwrap();
    assert!(core.is_dirty());
    assert!(core.data_version() > v0);
    assert_eq!(core.cell_text(0, 1, 1).unwrap(), "3");
    assert!(core.set_cell_input(9, 1, 1, "x").is_err());
}

#[test]
fn un_panico_interno_del_motor_se_convierte_en_error_tipado() {
    // Defecto de IronCalc 0.9.1 (hallado por mutación de las partes XML): si la hoja que
    // declara `workbook.xml` no tiene su relación en `workbook.xml.rels`, el importador
    // indexa un mapa con `rels[&id]` y entra en pánico («no entry found for key»). `Core`
    // lo captura y lo devuelve como `NoEsXlsx`; una carga con el pánico suelto tiraría la
    // aplicación entera al abrir un archivo dañado.
    let dir = tempfile::tempdir().unwrap();
    let completo = std::fs::read(crear_small(dir.path())).unwrap();
    let sin_relaciones = reemplazar_parte(
        &completo,
        "xl/_rels/workbook.xml.rels",
        br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#,
    );
    let directo = std::panic::catch_unwind(|| {
        ironcalc::import::load_from_xlsx_bytes(&sin_relaciones, "x", "en", "UTC")
    });
    println!(
        "IronCalc directo con rels vacío: {}",
        if directo.is_err() {
            "PÁNICO"
        } else {
            "error normal"
        }
    );
    let ruta = dir.path().join("sin-relaciones.xlsx");
    std::fs::write(&ruta, &sin_relaciones).unwrap();
    assert!(matches!(
        Core::open_xlsx(&ruta, opciones()),
        Err(CoreError::NoEsXlsx(_))
    ));
    assert!(matches!(
        Core::open_xlsx_bytes(&sin_relaciones, "x", opciones()),
        Err(CoreError::NoEsXlsx(_))
    ));
}
