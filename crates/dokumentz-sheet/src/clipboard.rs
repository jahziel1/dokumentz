//! Portapapeles de texto tabulado (FR-010, `plan.md` §5.8).
//!
//! **Copiar** ([`Core::copy_text`]): el texto de las celdas seleccionadas tal como se ven
//! (`get_formatted_cell_value`), columnas separadas por tabulador y filas por `\n`; los campos
//! con tabulador, salto de línea, retorno de carro o comillas van entre comillas dobles, con las
//! comillas internas duplicadas. No se usa `copy_to_clipboard()` de IronCalc: el campo `csv`
//! de su `Clipboard` es inaccesible (hallazgo 12 de T006).
//!
//! **Pegar** ([`Core::paste_text`]): el texto se lee como tabulado con comillas (RFC 4180) y
//! se vuelve a escribir normalizado antes de llamar a `paste_csv_string`, que es *una* entrada
//! de historial y evalúa una sola vez. La normalización corrige dos defectos de IronCalc 0.9.1
//! (hallazgo 11 de T006):
//!
//! * `paste_csv_string` descarta en silencio las filas con distinto número de campos que la
//!   primera: aquí se rellena a un rectángulo (las celdas que faltan quedan vacías).
//! * Los campos vacíos finales **borran** las celdas de la derecha: aquí se recortan las columnas
//!   finales que están vacías en todo el rectángulo. Las filas vacías finales también se
//!   descartan. Un salto de línea final no crea una fila.
//!
//! Las líneas vacías del interior del texto se conservan como filas vacías (el lector de
//! IronCalc las salta, y las filas de después subirían un lugar). Lo pegado se interpreta como
//! si se tecleara (`=1+1` es una fórmula) pero no pasa por la política de idioma de fórmulas, y
//! no ajusta la altura de las filas con saltos de línea (teclear sí lo hace).
//! Si el texto no cabe desde la celda de destino (más allá de la última fila o columna), no se
//! pega nada y se devuelve un error.

use ironcalc_base::expressions::types::Area;

use crate::core::Core;
use crate::error::CoreError;
use crate::geometry::{MAX_COLS, MAX_ROWS};

/// Añade un campo al texto tabulado, entre comillas si lo necesita. Con `forzar_vacio`, un
/// campo vacío va como `""` (para que una fila de un solo campo vacío no sea una línea vacía,
/// que el lector de IronCalc salta).
fn añadir_campo(salida: &mut String, valor: &str, forzar_vacio: bool) {
    let necesita = valor.contains(['\t', '\n', '\r', '"']) || (forzar_vacio && valor.is_empty());
    if necesita {
        salida.push('"');
        salida.push_str(&valor.replace('"', "\"\""));
        salida.push('"');
    } else {
        salida.push_str(valor);
    }
}

/// Lee texto tabulado con comillas. Una comilla solo abre un campo si está al principio de él
/// (en mitad de un campo es un carácter más). Termina fila `\n`, `\r\n` o `\r` fuera de comillas;
/// una línea vacía es una fila con un campo vacío. No hay fila tras un salto de línea final.
fn leer_tabulado(texto: &str) -> Vec<Vec<String>> {
    #[derive(PartialEq)]
    enum Estado {
        Inicio,
        Campo,
        Comillas,
        TrasComillas,
    }
    let mut filas: Vec<Vec<String>> = Vec::new();
    let mut fila: Vec<String> = Vec::new();
    let mut campo = String::new();
    let mut estado = Estado::Inicio;
    let mut saltar_lf = false;
    for c in texto.chars() {
        if saltar_lf {
            saltar_lf = false;
            if c == '\n' {
                continue;
            }
        }
        match (&estado, c) {
            (Estado::Inicio, '"') => estado = Estado::Comillas,
            (Estado::Comillas, '"') => estado = Estado::TrasComillas,
            (Estado::TrasComillas, '"') => {
                campo.push('"');
                estado = Estado::Comillas;
            }
            (Estado::Comillas, otro) => campo.push(otro),
            (_, '\t') => {
                fila.push(std::mem::take(&mut campo));
                estado = Estado::Inicio;
            }
            (_, '\n' | '\r') => {
                fila.push(std::mem::take(&mut campo));
                filas.push(std::mem::take(&mut fila));
                estado = Estado::Inicio;
                saltar_lf = c == '\r';
            }
            (_, otro) => {
                campo.push(otro);
                estado = Estado::Campo;
            }
        }
    }
    // Último registro, si no terminó con salto de línea.
    if estado != Estado::Inicio || !fila.is_empty() || !campo.is_empty() {
        fila.push(campo);
        filas.push(fila);
    }
    filas
}

/// Texto tabulado normalizado listo para `paste_csv_string`, con su tamaño.
struct Bloque {
    texto: String,
    filas: i32,
    cols: i32,
}

/// Rellena a rectángulo, recorta lo vacío del final y vuelve a escribir (ver el módulo).
/// `None` si no queda ninguna celda con contenido.
fn normalizar(texto: &str) -> Option<Bloque> {
    let mut filas = leer_tabulado(texto);
    while filas.last().is_some_and(|f| f.iter().all(String::is_empty)) {
        filas.pop();
    }
    let ancho = filas.iter().map(Vec::len).max()?;
    for f in &mut filas {
        f.resize(ancho, String::new());
    }
    let mut ancho_util = ancho;
    while ancho_util > 1 && filas.iter().all(|f| f[ancho_util - 1].is_empty()) {
        ancho_util -= 1;
    }
    let mut salida = String::new();
    for (i, f) in filas.iter().enumerate() {
        if i > 0 {
            salida.push('\n');
        }
        for (j, valor) in f[..ancho_util].iter().enumerate() {
            if j > 0 {
                salida.push('\t');
            }
            añadir_campo(&mut salida, valor, ancho_util == 1);
        }
    }
    Some(Bloque {
        texto: salida,
        filas: i32::try_from(filas.len()).unwrap_or(i32::MAX),
        cols: i32::try_from(ancho_util).unwrap_or(i32::MAX),
    })
}

impl Core {
    /// Texto tabulado de la selección, para el portapapeles del sistema (ver el módulo). Con
    /// filas o columnas enteras seleccionadas, se limita al rango con datos de la hoja (sin
    /// ello, una columna entera daría un millón de líneas vacías). Un rango normal se copia
    /// entero, aunque tenga celdas vacías al final.
    pub fn copy_text(&mut self) -> Result<String, CoreError> {
        self.sync_view_sheet();
        let sel = self.selection();
        let (f0, c0, mut f1, mut c1) = sel.range;
        if sel.is_entire_cols() || sel.is_entire_rows() {
            let g = self.geometry(sel.sheet)?;
            if sel.is_entire_cols() {
                f1 = f1.min(g.used_rows().max(f0));
            }
            if sel.is_entire_rows() {
                c1 = c1.min(g.used_cols().max(c0));
            }
        }
        let mut salida = String::new();
        for fila in f0..=f1 {
            if fila > f0 {
                salida.push('\n');
            }
            for col in c0..=c1 {
                if col > c0 {
                    salida.push('\t');
                }
                let valor = self
                    .um
                    .get_formatted_cell_value(sel.sheet, fila, col)
                    .map_err(CoreError::Motor)?;
                añadir_campo(&mut salida, &valor, false);
            }
        }
        Ok(salida)
    }

    /// Pega texto tabulado con la esquina superior izquierda del rango seleccionado como
    /// destino (`PasteText`). Es una sola entrada de historial; deja seleccionado lo pegado,
    /// con la celda activa en su esquina superior izquierda. Un texto sin contenido no hace
    /// nada. Si no cabe en la hoja desde el destino, no pega nada y falla con
    /// [`CoreError::Motor`].
    pub fn paste_text(&mut self, text: &str) -> Result<(), CoreError> {
        let Some(bloque) = normalizar(text) else {
            return Ok(());
        };
        self.sync_view_sheet();
        let sel = self.selection();
        let (fila, col) = (sel.range.0, sel.range.1);
        if i64::from(fila) - 1 + i64::from(bloque.filas) > i64::from(MAX_ROWS)
            || i64::from(col) - 1 + i64::from(bloque.cols) > i64::from(MAX_COLS)
        {
            return Err(CoreError::Motor(format!(
                "el texto pegado ({} filas × {} columnas) no cabe en la hoja desde la fila {fila}, columna {col}",
                bloque.filas, bloque.cols
            )));
        }
        // `paste_csv_string` termina seleccionando lo pegado y falla (después de escribir) si la
        // celda activa no está dentro: se deja la selección en una sola celda, la del destino.
        self.um
            .set_selected_cell(fila, col)
            .map_err(CoreError::Motor)?;
        // Si el destino cae dentro de una celda combinada, el motor ajusta la activa a su ancla.
        let (fila, col) = self.selection().active;
        let area = Area {
            sheet: sel.sheet,
            row: fila,
            column: col,
            width: 1,
            height: 1,
        };
        self.um
            .paste_csv_string(&area, &bloque.texto)
            .map_err(CoreError::Motor)?;
        // Pudo ampliar el rango usado. El motor deja seleccionado lo pegado, con la celda activa
        // en la esquina superior izquierda.
        self.invalidate_geometry();
        self.mark_changed();
        self.ensure_cell_visible(fila, col);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_comillas_tabuladores_y_saltos_de_linea() {
        let filas = leer_tabulado("a\t\"b\tc\"\t\"x\ny\"\t\"dice \"\"hola\"\"\"\r\n1\t2\n");
        assert_eq!(
            filas,
            vec![vec!["a", "b\tc", "x\ny", "dice \"hola\""], vec!["1", "2"],]
        );
    }

    #[test]
    fn una_comilla_en_mitad_de_un_campo_es_un_caracter() {
        assert_eq!(leer_tabulado("ab\"c\td"), vec![vec!["ab\"c", "d"]]);
    }

    #[test]
    fn terminadores_y_lineas_vacias() {
        assert_eq!(
            leer_tabulado("a\rb\r\nc\n\nd"),
            vec![vec!["a"], vec!["b"], vec!["c"], vec![""], vec!["d"],]
        );
        assert!(leer_tabulado("").is_empty());
        assert!(leer_tabulado("\n").len() == 1);
        // Un tabulador final es un campo vacío final.
        assert_eq!(leer_tabulado("a\t"), vec![vec!["a", ""]]);
    }

    #[test]
    fn normaliza_a_rectangulo_y_recorta_columnas_vacias() {
        let b = normalizar("a\tb\tc\n1\t2\nx\ty\tz\n").unwrap();
        assert_eq!((b.filas, b.cols), (3, 3));
        assert_eq!(b.texto, "a\tb\tc\n1\t2\t\nx\ty\tz");
        let b = normalizar("a\tb\t\t\n1\t2\t\t\n").unwrap();
        assert_eq!((b.filas, b.cols), (2, 2));
        assert_eq!(b.texto, "a\tb\n1\t2");
    }

    #[test]
    fn filas_vacias_finales_no_cuentan_y_las_internas_se_conservan() {
        let b = normalizar("a\n\nb\n\n\n").unwrap();
        assert_eq!((b.filas, b.cols), (3, 1));
        assert_eq!(b.texto, "a\n\"\"\nb");
        assert!(normalizar("").is_none());
        assert!(normalizar("\t\t\n\n").is_none());
    }
}
