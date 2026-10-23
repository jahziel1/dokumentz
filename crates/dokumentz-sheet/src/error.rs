//! Errores tipados del núcleo.
//!
//! Ningún camino que dependa de la entrada del usuario (archivos, texto, bytes de la
//! interfaz) entra en pánico: todo se devuelve como `CoreError` o `CodecError`
//! (`plan.md` §5.9).

use std::path::PathBuf;

use thiserror::Error;

/// Error de una operación del núcleo.
#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    /// La ruta no existe.
    #[error("no existe el archivo: {}", .0.display())]
    NoExiste(PathBuf),
    /// El sistema operativo negó el acceso a la ruta.
    #[error("sin permiso para leer: {}", .0.display())]
    SinPermiso(PathBuf),
    /// El contenido no es un libro `.xlsx` legible (archivo corrupto, truncado o de otro
    /// formato; también una ruta que es una carpeta). El texto explica la causa.
    #[error("no es un archivo .xlsx válido: {0}")]
    NoEsXlsx(String),
    /// Otro error de entrada/salida (la ruta, o el sistema, en el texto).
    #[error("error de entrada/salida: {0}")]
    Io(String),
    /// Índice de hoja fuera de rango.
    #[error("la hoja {0} no existe")]
    HojaInvalida(u32),
    /// El motor (IronCalc) rechazó la operación; el texto es el suyo.
    #[error("el motor rechazó la operación: {0}")]
    Motor(String),
    /// Error al decodificar una lista de dibujo.
    #[error(transparent)]
    Codec(#[from] CodecError),
}

/// Error al decodificar el formato binario v1 de la lista de dibujo (`plan.md` §5.4).
///
/// Cada variante describe qué falló sin asignar memoria proporcional a lo que el
/// encabezado *dice*: los tamaños declarados se validan contra los bytes recibidos.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodecError {
    /// Faltan bytes: hacen falta `necesarios` y hay `hay`.
    #[error("datos truncados: se necesitan {necesarios} bytes y hay {hay}")]
    Truncado {
        /// Bytes que exige lo declarado hasta el punto donde falló la lectura.
        necesarios: u64,
        /// Bytes recibidos.
        hay: usize,
    },
    /// Los primeros cuatro bytes no son la magia «PDL1».
    #[error("magia incorrecta: {0:#010x}")]
    MagiaIncorrecta(u32),
    /// Versión del formato que este lector no entiende.
    #[error("versión no soportada: {0}")]
    VersionNoSoportada(u16),
    /// Un campo reservado (banderas del encabezado o bytes reservados de un estilo) no es cero.
    #[error("campo reservado distinto de cero ({0})")]
    Reservado(&'static str),
    /// Los tamaños declarados no cuadran (desbordan o no coinciden con la longitud recibida).
    #[error("tamaños inconsistentes: {0}")]
    Tamano(&'static str),
    /// Un valor de enumeración, un índice o una cadena fuera de rango.
    #[error("valor inválido: {0}")]
    Valor(&'static str),
    /// La sección de cadenas no es UTF-8 válido.
    #[error("las cadenas no son UTF-8 válido")]
    Utf8,
}
