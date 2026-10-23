//! Direcciones A1 mínimas para validar y comparar guiones. No depende del núcleo: solo hace
//! falta saber si el texto es una celda o un rango válidos y llevarlo a una forma canónica.

/// Última columna de una hoja (`XFD`).
pub const MAX_COL: u32 = 16_384;
/// Última fila de una hoja.
pub const MAX_ROW: u32 = 1_048_576;

/// Celda `A1` (con o sin `$`, en cualquier caja) -> `(columna, fila)` desde 1.
pub fn parse_cell(texto: &str) -> Option<(u32, u32)> {
    let limpio: String = texto.chars().filter(|&c| c != '$').collect();
    let corte = limpio.find(|c: char| c.is_ascii_digit())?;
    let (letras, digitos) = limpio.split_at(corte);
    if letras.is_empty() || !letras.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if digitos.is_empty()
        || !digitos.chars().all(|c| c.is_ascii_digit())
        || digitos.starts_with('0')
    {
        return None;
    }
    let mut col: u32 = 0;
    for c in letras.chars() {
        col = col.checked_mul(26)? + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
        if col > MAX_COL {
            return None;
        }
    }
    let fila: u32 = digitos.parse().ok()?;
    (fila <= MAX_ROW).then_some((col, fila))
}

fn letras(mut col: u32) -> String {
    let mut s = Vec::new();
    while col > 0 {
        col -= 1;
        s.push(b'A' + (col % 26) as u8);
        col /= 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Celda canónica: mayúsculas y sin `$` (`$b$2` -> `B2`). `None` si no es una celda válida.
pub fn normalize_cell(texto: &str) -> Option<String> {
    let (c, f) = parse_cell(texto)?;
    Some(format!("{}{f}", letras(c)))
}

/// Rango canónico: esquinas ordenadas (superior izquierda : inferior derecha) y una sola celda
/// sin `:` (`d5:b2` -> `B2:D5`, `b2:b2` -> `B2`). `None` si no es una celda ni un rango válidos.
pub fn normalize_range(texto: &str) -> Option<String> {
    let mut partes = texto.split(':');
    let a = parse_cell(partes.next()?)?;
    let Some(b) = partes.next() else {
        return normalize_cell(texto);
    };
    let b = parse_cell(b)?;
    if partes.next().is_some() {
        return None;
    }
    let (c0, c1) = (a.0.min(b.0), a.0.max(b.0));
    let (f0, f1) = (a.1.min(b.1), a.1.max(b.1));
    if (c0, f0) == (c1, f1) {
        return Some(format!("{}{f0}", letras(c0)));
    }
    Some(format!("{}{f0}:{}{f1}", letras(c0), letras(c1)))
}
