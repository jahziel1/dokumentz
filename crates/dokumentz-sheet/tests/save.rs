//! E08 (T028): guardado atómico (FR-011, `plan.md` §5.9; US3.5 y US3.6).
//!
//! * Guardar y reabrir conserva valores y fórmulas.
//! * Tras un fallo (disco lleno simulado, carpeta de solo lectura, archivo de solo lectura,
//!   carpeta inexistente) el archivo original queda idéntico (hash) y no queda ningún temporal.
//! * «Guardar» sobrescribe el archivo abierto, «Guardar como» cambia la ruta del documento y
//!   `is_dirty()` vuelve a `false`.

mod common;

use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, BufWriter, ErrorKind, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use dokumentz_sheet::{Command, Core, CoreError, Lang, OpenOptions};
use rust_xlsxwriter::Workbook;

use common::{abrir, crear_a2, crear_empty, crear_small, opciones};

/// Hash del contenido de un archivo (compara «idéntico» antes y después de un fallo).
fn hash(ruta: &Path) -> u64 {
    let mut h = DefaultHasher::new();
    h.write(&std::fs::read(ruta).expect("leer el archivo"));
    h.finish()
}

/// Archivos de `dir` cuyo nombre contiene `.tmp-` (los temporales del guardado atómico).
fn temporales(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".tmp-"))
        .collect()
}

/// Volcado de todas las hojas: `(hoja, fila, col, texto visible, contenido editable)` de cada
/// celda del rango usado. Dos libros con el mismo volcado tienen los mismos valores y fórmulas.
fn volcado(core: &mut Core) -> Vec<(u32, i32, i32, String, String)> {
    let mut v = Vec::new();
    for hoja in 0..core.sheet_names().len() as u32 {
        let g = core.geometry(hoja).unwrap();
        for fila in 1..=g.used_rows() {
            for col in 1..=g.used_cols() {
                v.push((
                    hoja,
                    fila,
                    col,
                    core.cell_text(hoja, fila, col).unwrap(),
                    core.cell_input(hoja, fila, col).unwrap(),
                ));
            }
        }
    }
    v
}

/// Abre `small.xlsx` copiado a un directorio temporal y lo deja editable.
fn small() -> (tempfile::TempDir, PathBuf, Core) {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_small(dir.path());
    let mut core = abrir(&ruta);
    core.recalculate();
    (dir, ruta, core)
}

// ----------------------------------------------------------------------------------------
// Guardar y reabrir
// ----------------------------------------------------------------------------------------

#[test]
fn guardar_como_y_reabrir_conserva_valores_y_formulas() {
    let (dir, _ruta, mut core) = small();
    core.set_cell_input(0, 8, 1, "nuevo ñandú").unwrap();
    core.set_cell_input(0, 8, 2, "=B2+B3").unwrap();
    core.set_cell_input(0, 9, 1, "texto\ncon salto").unwrap();
    assert!(core.is_dirty());
    let version = core.data_version();

    let destino = dir.path().join("copia.xlsx");
    core.save(Some(&destino)).unwrap();
    assert!(!core.is_dirty(), "tras guardar no hay cambios pendientes");
    assert_eq!(
        core.path(),
        Some(destino.as_path()),
        "Guardar como cambia la ruta"
    );
    assert_eq!(core.data_version(), version, "guardar no cambia los datos");
    assert!(temporales(dir.path()).is_empty());

    let mut otra = abrir(&destino);
    otra.recalculate();
    assert_eq!(volcado(&mut otra), volcado(&mut core));
    assert_eq!(
        otra.cell_input(0, 2, 4).unwrap(),
        "=B2*2",
        "las fórmulas se conservan"
    );
    assert_eq!(otra.cell_text(0, 2, 4).unwrap(), "2469");
    assert_eq!(otra.cell_input(0, 8, 2).unwrap(), "=B2+B3");
}

#[test]
fn guardar_sobrescribe_el_archivo_abierto() {
    let (dir, ruta, mut core) = small();
    let antes = hash(&ruta);
    core.set_cell_input(0, 1, 1, "Cambiado").unwrap();
    core.execute(Command::Save { path: None }).unwrap();
    assert_ne!(hash(&ruta), antes, "el archivo cambió");
    assert_eq!(core.path(), Some(ruta.as_path()));
    assert!(!core.is_dirty());
    assert!(temporales(dir.path()).is_empty());
    let otra = abrir(&ruta);
    assert_eq!(otra.cell_text(0, 1, 1).unwrap(), "Cambiado");
    // Y se puede seguir editando y guardando.
    core.set_cell_input(0, 1, 2, "Otra vez").unwrap();
    assert!(core.is_dirty());
    core.save(None).unwrap();
    assert!(!core.is_dirty());
    assert_eq!(abrir(&ruta).cell_text(0, 1, 2).unwrap(), "Otra vez");
}

#[test]
fn guardar_sin_ruta_pide_guardar_como() {
    let mut core = Core::new_empty(opciones()).unwrap();
    core.set_cell_input(0, 1, 1, "x").unwrap();
    let err = core.save(None).unwrap_err();
    assert!(
        matches!(&err, CoreError::Io(m) if m.contains("Guardar como")),
        "{err:?}"
    );
    assert!(core.is_dirty(), "el documento sigue con cambios pendientes");
    assert_eq!(core.path(), None);
}

#[test]
fn un_libro_vacio_se_guarda_y_se_reabre_vacio() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = abrir(&crear_empty(dir.path()));
    let destino = dir.path().join("vacio2.xlsx");
    core.save(Some(&destino)).unwrap();
    let mut otra = abrir(&destino);
    assert_eq!(otra.sheet_names().len(), 1);
    assert!(volcado(&mut otra).is_empty());
}

#[test]
fn el_texto_largo_con_saltos_de_linea_no_se_pierde_al_guardar() {
    let mut core = Core::new_empty(opciones()).unwrap();
    let largo = format!("{}\nsegunda\tlínea ñ", "x".repeat(20_000));
    core.set_cell_input(0, 3, 2, &largo).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("largo.xlsx");
    core.save(Some(&destino)).unwrap();
    assert_eq!(abrir(&destino).cell_input(0, 3, 2).unwrap(), largo);
}

#[test]
fn el_idioma_no_cambia_lo_guardado_las_formulas_van_en_ingles() {
    let mut core = Core::new_empty(OpenOptions {
        lang: Lang::Es,
        locale: "en",
        tz: "UTC",
    })
    .unwrap();
    core.set_cell_input(0, 1, 1, "5").unwrap();
    core.set_cell_input(0, 1, 2, "=SUMA(A1,1)").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("es.xlsx");
    core.save(Some(&destino)).unwrap();
    let en = Core::open_xlsx(&destino, opciones()).unwrap();
    assert_eq!(en.cell_input(0, 1, 2).unwrap(), "=SUM(A1,1)");
    let es = Core::open_xlsx(
        &destino,
        OpenOptions {
            lang: Lang::Es,
            locale: "en",
            tz: "UTC",
        },
    )
    .unwrap();
    assert_eq!(es.cell_input(0, 1, 2).unwrap(), "=SUMA(A1,1)");
}

#[test]
fn ida_y_vuelta_de_un_a2_pequeno_no_cambia_ni_valores_ni_formulas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = crear_a2(dir.path(), 60, 20, 30);
    let mut core = Core::open_xlsx(&ruta, opciones()).unwrap();
    core.recalculate();
    core.set_cell_input(0, 1, 21, "5").unwrap();
    let destino = dir.path().join("a2_guardado.xlsx");
    core.save(Some(&destino)).unwrap();
    let mut otra = Core::open_xlsx(&destino, opciones()).unwrap();
    otra.recalculate();
    assert_eq!(volcado(&mut otra), volcado(&mut core));
    assert_eq!(
        otra.cell_text(0, 2, 22).unwrap(),
        "35",
        "V2 = U1 * A2 = 5 * 7"
    );
}

// ----------------------------------------------------------------------------------------
// Fallos: el original no cambia
// ----------------------------------------------------------------------------------------

/// Escritor que falla con «disco lleno» tras `restante` bytes (y, si `al_cerrar`, al vaciar el
/// búfer). Es el que usan las pruebas para inyectar fallos con `Core::save_with_writer`.
struct FallaTras {
    interno: BufWriter<File>,
    restante: usize,
    al_cerrar: bool,
}

impl FallaTras {
    fn new(f: File, restante: usize, al_cerrar: bool) -> FallaTras {
        FallaTras {
            interno: BufWriter::new(f),
            restante,
            al_cerrar,
        }
    }
}

impl Write for FallaTras {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.restante == 0 {
            return Err(io::Error::new(
                ErrorKind::StorageFull,
                "disco lleno simulado",
            ));
        }
        let n = buf.len().min(self.restante);
        let escrito = self.interno.write(&buf[..n])?;
        self.restante -= escrito;
        Ok(escrito)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.al_cerrar {
            return Err(io::Error::new(
                ErrorKind::StorageFull,
                "disco lleno simulado",
            ));
        }
        self.interno.flush()
    }
}

impl Seek for FallaTras {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.interno.seek(pos)
    }
}

#[test]
fn tras_un_fallo_inyectado_el_archivo_original_sigue_identico() {
    let (dir, ruta, mut core) = small();
    let hash_antes = hash(&ruta);
    let tamano = core.to_xlsx_bytes().unwrap().len();
    core.set_cell_input(0, 1, 1, "cambio que no se guarda")
        .unwrap();

    // Fallo al principio, a mitad y casi al final del archivo, y al vaciar el búfer.
    for (restante, al_cerrar) in [
        (0, false),
        (100, false),
        (tamano / 2, false),
        (tamano - 1, false),
        (usize::MAX, true),
    ] {
        let err = core
            .save_with_writer(None, |f| FallaTras::new(f, restante, al_cerrar))
            .unwrap_err();
        assert!(
            matches!(&err, CoreError::Io(m) if m.contains("small.xlsx")
                && (m.contains("disco lleno simulado") || m.contains("no hay espacio en disco"))),
            "error claro con la causa y el archivo (restante {restante}): {err:?}"
        );
        assert_eq!(
            hash(&ruta),
            hash_antes,
            "el original no cambia (restante {restante})"
        );
        assert!(
            temporales(dir.path()).is_empty(),
            "no queda el temporal (restante {restante})"
        );
        assert!(core.is_dirty(), "el documento sigue con cambios pendientes");
        assert_eq!(core.path(), Some(ruta.as_path()));
    }
    // Y tras los fallos, un guardado normal funciona.
    core.save(None).unwrap();
    assert_ne!(hash(&ruta), hash_antes);
    assert_eq!(
        abrir(&ruta).cell_text(0, 1, 1).unwrap(),
        "cambio que no se guarda"
    );
}

#[test]
fn guardar_como_con_fallo_no_cambia_la_ruta_ni_deja_un_archivo_a_medias() {
    let (dir, ruta, mut core) = small();
    let destino = dir.path().join("nuevo.xlsx");
    let err = core
        .save_with_writer(Some(&destino), |f| FallaTras::new(f, 500, false))
        .unwrap_err();
    assert!(matches!(err, CoreError::Io(_)), "{err:?}");
    assert!(!destino.exists(), "no se crea el destino a medias");
    assert!(temporales(dir.path()).is_empty());
    assert_eq!(
        core.path(),
        Some(ruta.as_path()),
        "la ruta del documento no cambia"
    );
}

#[cfg(unix)]
mod permisos {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// ¿Se ejecuta como administrador? Entonces los permisos no impiden escribir.
    fn es_root() -> bool {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("p");
        std::fs::write(&f, b"x").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o444)).unwrap();
        let abre = std::fs::OpenOptions::new().write(true).open(&f).is_ok();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        abre
    }

    #[test]
    fn carpeta_de_solo_lectura_es_sin_permiso_y_el_original_no_cambia() {
        if es_root() {
            return;
        }
        let (dir, ruta, mut core) = small();
        let hash_antes = hash(&ruta);
        core.set_cell_input(0, 1, 1, "x").unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let resultado = core.save(None);
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(resultado, Err(CoreError::SinPermiso(ruta.clone())));
        assert_eq!(hash(&ruta), hash_antes);
        assert!(temporales(dir.path()).is_empty());
        assert!(core.is_dirty());
        // Con el mensaje legible.
        let msg = CoreError::SinPermiso(ruta.clone()).to_string();
        assert!(msg.contains("sin permiso"), "{msg}");
    }

    #[test]
    fn archivo_de_solo_lectura_se_rechaza_aunque_el_rename_pudiera_reemplazarlo() {
        let (dir, ruta, mut core) = small();
        let hash_antes = hash(&ruta);
        core.set_cell_input(0, 1, 1, "x").unwrap();
        std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(0o444)).unwrap();
        let resultado = core.save(None);
        std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(resultado, Err(CoreError::SinPermiso(ruta.clone())));
        assert_eq!(hash(&ruta), hash_antes);
        assert!(temporales(dir.path()).is_empty());
    }
}

#[test]
fn carpeta_inexistente_es_un_error_claro_no_un_panico() {
    // `save_to_xlsx` de IronCalc entra en pánico en este caso (hallazgo de T006).
    let mut core = Core::new_empty(opciones()).unwrap();
    core.set_cell_input(0, 1, 1, "x").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("no_existe").join("libro.xlsx");
    let err = core.save(Some(&destino)).unwrap_err();
    assert!(
        matches!(&err, CoreError::Io(m) if m.contains("no existe la carpeta")),
        "{err:?}"
    );
    assert!(core.is_dirty());
    assert_eq!(core.path(), None);
}

#[test]
fn guardar_sobre_una_carpeta_es_un_error_y_la_carpeta_no_se_toca() {
    let mut core = Core::new_empty(opciones()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let carpeta = dir.path().join("carpeta.xlsx");
    std::fs::create_dir(&carpeta).unwrap();
    std::fs::write(carpeta.join("dentro.txt"), b"x").unwrap();
    let err = core.save(Some(&carpeta)).unwrap_err();
    assert!(
        matches!(&err, CoreError::Io(m) if m.contains("carpeta")),
        "{err:?}"
    );
    assert!(carpeta.join("dentro.txt").exists());
    assert!(temporales(dir.path()).is_empty());
}

#[test]
fn una_ruta_sin_nombre_de_archivo_es_un_error() {
    let mut core = Core::new_empty(opciones()).unwrap();
    assert!(matches!(
        core.save(Some(Path::new("/"))),
        Err(CoreError::Io(_))
    ));
    assert!(matches!(
        core.save(Some(Path::new(""))),
        Err(CoreError::Io(_))
    ));
}

#[test]
fn un_temporal_viejo_con_el_mismo_pid_no_impide_guardar() {
    let (dir, ruta, mut core) = small();
    let viejo = dir
        .path()
        .join(format!("small.xlsx.tmp-{}", std::process::id()));
    std::fs::write(&viejo, b"basura de un guardado anterior").unwrap();
    core.set_cell_input(0, 1, 1, "ok").unwrap();
    core.save(None).unwrap();
    assert_eq!(abrir(&ruta).cell_text(0, 1, 1).unwrap(), "ok");
    assert!(
        temporales(dir.path()).is_empty(),
        "el temporal se reemplazó y se renombró"
    );
}

#[test]
fn guardar_no_cambia_la_seleccion_ni_la_edicion_abierta() {
    let (dir, _ruta, mut core) = small();
    core.set_viewport(800.0, 400.0, 1.0);
    core.execute(Command::Move {
        dir: dokumentz_sheet::Dir::Down,
        extend: false,
    })
    .unwrap();
    core.execute(Command::BeginEdit {
        mode: dokumentz_sheet::EditMode::Replace,
        seed: Some("a medias".into()),
    })
    .unwrap();
    let sel = core.selection();
    core.save(Some(&dir.path().join("x.xlsx"))).unwrap();
    assert_eq!(core.selection(), sel);
    assert_eq!(core.edit_state().unwrap().text, "a medias");
}

/// Un archivo guardado puede crearse de nuevo con `rust_xlsxwriter`: la prueba de que el
/// directorio temporal de las pruebas permite crear libros externos sin interferir.
#[test]
fn el_guardado_funciona_en_un_directorio_con_otros_archivos() {
    let dir = tempfile::tempdir().unwrap();
    let otro = dir.path().join("otro.xlsx");
    Workbook::new().save(&otro).unwrap();
    let hash_otro = hash(&otro);
    let mut core = Core::new_empty(opciones()).unwrap();
    core.set_cell_input(0, 1, 1, "x").unwrap();
    core.save(Some(&dir.path().join("mio.xlsx"))).unwrap();
    assert_eq!(hash(&otro), hash_otro);
}
