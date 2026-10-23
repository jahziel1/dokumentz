//! Guardado atómico de `.xlsx` (FR-011, `plan.md` §5.9).
//!
//! El libro se escribe en un archivo temporal junto al destino (`<nombre>.tmp-<pid>`) y, solo si
//! todo salió bien, se renombra sobre el destino. Así el archivo original queda intacto ante
//! cualquier fallo (carpeta de solo lectura, disco lleno, error del motor): el temporal se borra
//! y el error vuelve tipado. No se usa `save_to_xlsx` de IronCalc: se niega a escribir sobre un
//! archivo que ya existe y entra en pánico si la carpeta no existe (hallazgo de T006); se usa
//! `save_xlsx_to_writer` sobre un archivo propio, y un pánico del motor se convierte en error.
//!
//! «Guardar» es `Core::save(None)` (el archivo abierto); «Guardar como» es `Core::save(Some(ruta))`
//! y el documento pasa a apuntar a la nueva ruta. Tras guardar, el documento queda sin cambios
//! pendientes. Un archivo destino de solo lectura se rechaza con [`CoreError::SinPermiso`]
//! aunque el `rename` pudiera reemplazarlo.

use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Seek, Write};
use std::path::{Path, PathBuf};

use ironcalc::error::XlsxError;
use ironcalc::export::save_xlsx_to_writer;

use crate::core::{Core, capturar_panico, error_de_io};
use crate::error::CoreError;

/// Errores de escritura del motor (el zip o el flujo): siempre son de guardado, no de formato.
fn error_de_guardado(destino: &Path, e: &XlsxError) -> CoreError {
    CoreError::Io(format!("no se pudo guardar {}: {e}", destino.display()))
}

/// Traduce un error de escritura en disco a uno claro.
fn error_de_escritura(destino: &Path, e: &std::io::Error) -> CoreError {
    match e.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => {
            CoreError::SinPermiso(destino.to_path_buf())
        }
        ErrorKind::NotFound => CoreError::Io(format!(
            "no existe la carpeta de destino: {}",
            destino
                .parent()
                .map_or_else(|| ".".to_string(), |p| p.display().to_string())
        )),
        ErrorKind::StorageFull => CoreError::Io(format!(
            "no hay espacio en disco para guardar {}",
            destino.display()
        )),
        _ => CoreError::Io(format!("no se pudo guardar {}: {e}", destino.display())),
    }
}

/// Ruta del temporal: junto al destino, para que el `rename` sea dentro del mismo volumen.
fn ruta_temporal(destino: &Path) -> Result<PathBuf, CoreError> {
    let nombre = destino.file_name().ok_or_else(|| {
        CoreError::Io(format!(
            "la ruta no tiene nombre de archivo: {}",
            destino.display()
        ))
    })?;
    let mut temporal = nombre.to_os_string();
    temporal.push(format!(".tmp-{}", std::process::id()));
    Ok(destino.with_file_name(temporal))
}

/// Rechaza lo que no se puede sobrescribir: una carpeta o un archivo de solo lectura.
fn comprobar_destino(destino: &Path) -> Result<(), CoreError> {
    match fs::metadata(destino) {
        Ok(m) if m.is_dir() => Err(CoreError::Io(format!(
            "la ruta es una carpeta: {}",
            destino.display()
        ))),
        Ok(m) if m.permissions().readonly() => Err(CoreError::SinPermiso(destino.to_path_buf())),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error_de_io(destino, &e)),
    }
}

impl Core {
    /// Guarda el libro como `.xlsx` (`Save`). Con `None` sobrescribe el archivo abierto (falla si
    /// el documento no tiene ruta); con `Some(ruta)` guarda ahí («Guardar como») y el documento
    /// pasa a apuntar a esa ruta. Atómico: ver el módulo. Tras guardar, `is_dirty()` es `false`.
    pub fn save(&mut self, path: Option<&Path>) -> Result<(), CoreError> {
        self.save_with_writer(path, BufWriter::new)
    }

    /// Como [`Core::save`] pero `envolver` decide el escritor que recibe el archivo temporal.
    /// Existe para que las pruebas inyecten fallos de escritura (disco lleno) sin tocar la API
    /// normal; `save` pasa un `BufWriter`.
    #[doc(hidden)]
    pub fn save_with_writer<W: Write + Seek>(
        &mut self,
        path: Option<&Path>,
        envolver: impl FnOnce(File) -> W,
    ) -> Result<(), CoreError> {
        // Instrumentación (T030): `save_ms`, el guardado completo (serializar, escribir, renombrar).
        crate::core::tramo!(self, crate::core::nombres::SAVE_MS);
        let destino: PathBuf = match path {
            Some(p) => p.to_path_buf(),
            None => self
                .path()
                .ok_or_else(|| {
                    CoreError::Io(
                        "el documento no tiene ruta: use «Guardar como» y elija una".to_string(),
                    )
                })?
                .to_path_buf(),
        };
        comprobar_destino(&destino)?;
        let temporal = ruta_temporal(&destino)?;
        let resultado = self.escribir_y_reemplazar(&destino, &temporal, envolver);
        if resultado.is_err() {
            // El original no se tocó; solo hay que quitar el temporal a medias.
            let _ = fs::remove_file(&temporal);
        }
        resultado?;
        self.mark_saved(destino);
        Ok(())
    }

    fn escribir_y_reemplazar<W: Write + Seek>(
        &self,
        destino: &Path,
        temporal: &Path,
        envolver: impl FnOnce(File) -> W,
    ) -> Result<(), CoreError> {
        // `File::create` reemplaza un temporal viejo con el mismo nombre (de un proceso que
        // terminó sin limpiar y cuyo pid se reutilizó).
        let fichero = File::create(temporal).map_err(|e| error_de_escritura(destino, &e))?;
        let sincronizador = fichero
            .try_clone()
            .map_err(|e| error_de_escritura(destino, &e))?;
        let modelo = self.um.get_model();
        let mut escritor = capturar_panico(|| {
            save_xlsx_to_writer(modelo, envolver(fichero))
                .map_err(|e| error_de_guardado(destino, &e))
        })?;
        escritor
            .flush()
            .map_err(|e| error_de_escritura(destino, &e))?;
        drop(escritor);
        sincronizador
            .sync_all()
            .map_err(|e| error_de_escritura(destino, &e))?;
        drop(sincronizador);
        fs::rename(temporal, destino).map_err(|e| error_de_escritura(destino, &e))
    }
}
