//! Formato binario v1 de la lista de dibujo (`plan.md` §5.4).
//!
//! Es el contrato entre el núcleo y `drawlist.ts` (interfaz de Tauri); se congela con el
//! archivo dorado `tests/golden/drawlist_v1.bin`. Todo es *little-endian*.
//!
//! | Sección | Contenido |
//! |---|---|
//! | Cabecera (64 B) | `u32` magia `0x31444C50` («PLD1» en memoria), `u16` versión = 1, `u16` banderas = 0, `u64` `data_version`, `u32` hoja, `i32` `first_row`, `i32` `first_col`, `u32` `n_rows`, `u32` `n_cols`, `u32` `n_styles`, `u32` `n_cells`, `u32` `strings_len`, `f64` `origin_x`, `f64` `origin_y` |
//! | Aristas de columnas | `f32 × (n_cols + 1)`, relativas a `origin_x` |
//! | Aristas de filas | `f32 × (n_rows + 1)`, relativas a `origin_y` |
//! | Etiquetas | `n_cols` y luego `n_rows` pares `(u32 str_off, u32 str_len)` |
//! | Estilos | `n_styles × 20 B`: `u32 fg`, `u32 fill` (0 = sin relleno), `f32 font_px`, `u16 family`, `u8 flags` (negrita 1, cursiva 2, subrayado 4, tachado 8, ajuste 16), `u8 h_align`, `u8 v_align`, `u8[3]` reservado = 0 |
//! | Celdas | `n_cells × 20 B`: `u32 row_off`, `u32 col_off`, `u32 str_off`, `u32 str_len`, `u16 style`, `u8 kind`, `u8 spill_cols` |
//! | Cadenas | `strings_len` bytes UTF-8 |
//!
//! Alineación: cada sección empieza en un desplazamiento múltiplo de 4 sin necesidad de
//! relleno, porque los elementos de todas las anteriores miden múltiplos de 4 bytes; la sección
//! de cadenas va al final y puede tener cualquier longitud. No hay bytes después de ella.
//!
//! Colores: `u32` `0xRRGGBBAA` (rojo en el byte alto del número; en memoria el orden es
//! `AA BB GG RR`, porque es *little-endian*). Un relleno con valor 0 significa «sin relleno».
//!
//! **El lector es estricto**, para que modificar cualquier byte del dorado se note: rechaza
//! banderas o bytes reservados distintos de cero, valores de enumeración fuera de rango,
//! índices fuera de rango, aristas que no crecen, celdas desordenadas y cadenas que no son
//! UTF-8. Los tamaños declarados se comprueban contra la longitud recibida **antes** de
//! reservar memoria, así que una cabecera mentirosa no puede provocar una reserva enorme.

use crate::drawlist::{
    Align, CellStyle, DrawCell, DrawList, Kind, MAX_SPILL, Rgba, StrRef, VAlign,
};
use crate::error::CodecError;
use crate::geometry::{MAX_COLS, MAX_ROWS};

/// Magia del formato: «PLD1» en memoria (`0x31444C50` leído como `u32` *little-endian*).
pub const MAGIC: u32 = 0x3144_4C50;
/// Versión del formato que escribe y lee este módulo.
pub const VERSION: u16 = 1;
/// Tamaño de la cabecera.
pub const HEADER_LEN: usize = 64;
/// Tamaño de un estilo.
pub const STYLE_LEN: usize = 20;
/// Tamaño de una celda.
pub const CELL_LEN: usize = 20;

const F_NEGRITA: u8 = 1;
const F_CURSIVA: u8 = 2;
const F_SUBRAYADO: u8 = 4;
const F_TACHADO: u8 = 8;
const F_AJUSTE: u8 = 16;
const F_TODAS: u8 = F_NEGRITA | F_CURSIVA | F_SUBRAYADO | F_TACHADO | F_AJUSTE;

/// Longitud exacta que ocupará la lista codificada.
pub fn encoded_len(dl: &DrawList) -> usize {
    HEADER_LEN
        + 4 * dl.col_edges.len()
        + 4 * dl.row_edges.len()
        + 8 * (dl.col_labels.len() + dl.row_labels.len())
        + STYLE_LEN * dl.styles.len()
        + CELL_LEN * dl.cells.len()
        + dl.strings.len()
}

/// Codifica la lista al formato v1. `dl` debe ser coherente (`col_edges.len() == n_cols + 1`,
/// etc.); toda lista construida por el núcleo o devuelta por [`decode`] lo es.
pub fn encode(dl: &DrawList) -> Vec<u8> {
    debug_assert_eq!(dl.col_edges.len(), dl.n_cols as usize + 1);
    debug_assert_eq!(dl.row_edges.len(), dl.n_rows as usize + 1);
    debug_assert_eq!(dl.col_labels.len(), dl.n_cols as usize);
    debug_assert_eq!(dl.row_labels.len(), dl.n_rows as usize);

    let mut out = Vec::with_capacity(encoded_len(dl));
    // Cabecera.
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&dl.data_version.to_le_bytes());
    out.extend_from_slice(&dl.sheet.to_le_bytes());
    out.extend_from_slice(&dl.first_row.to_le_bytes());
    out.extend_from_slice(&dl.first_col.to_le_bytes());
    out.extend_from_slice(&dl.n_rows.to_le_bytes());
    out.extend_from_slice(&dl.n_cols.to_le_bytes());
    out.extend_from_slice(&u32_de(dl.styles.len()).to_le_bytes());
    out.extend_from_slice(&u32_de(dl.cells.len()).to_le_bytes());
    out.extend_from_slice(&u32_de(dl.strings.len()).to_le_bytes());
    out.extend_from_slice(&dl.origin.0.to_le_bytes());
    out.extend_from_slice(&dl.origin.1.to_le_bytes());
    debug_assert_eq!(out.len(), HEADER_LEN);
    // Aristas.
    for e in dl.col_edges.iter().chain(&dl.row_edges) {
        out.extend_from_slice(&e.to_le_bytes());
    }
    // Etiquetas.
    for s in dl.col_labels.iter().chain(&dl.row_labels) {
        out.extend_from_slice(&s.off.to_le_bytes());
        out.extend_from_slice(&s.len.to_le_bytes());
    }
    // Estilos.
    for st in &dl.styles {
        out.extend_from_slice(&st.fg.0.to_le_bytes());
        out.extend_from_slice(&st.fill.map_or(0, |f| f.0).to_le_bytes());
        out.extend_from_slice(&st.font_px.to_le_bytes());
        out.extend_from_slice(&st.family.to_le_bytes());
        let mut flags = 0;
        if st.bold {
            flags |= F_NEGRITA;
        }
        if st.italic {
            flags |= F_CURSIVA;
        }
        if st.underline {
            flags |= F_SUBRAYADO;
        }
        if st.strike {
            flags |= F_TACHADO;
        }
        if st.wrap {
            flags |= F_AJUSTE;
        }
        out.push(flags);
        out.push(st.h_align as u8);
        out.push(st.v_align as u8);
        out.extend_from_slice(&[0, 0, 0]);
    }
    // Celdas.
    for c in &dl.cells {
        out.extend_from_slice(&c.row_off.to_le_bytes());
        out.extend_from_slice(&c.col_off.to_le_bytes());
        out.extend_from_slice(&c.text.off.to_le_bytes());
        out.extend_from_slice(&c.text.len.to_le_bytes());
        out.extend_from_slice(&c.style.to_le_bytes());
        out.push(c.kind as u8);
        out.push(c.spill_cols);
    }
    // Cadenas.
    out.extend_from_slice(dl.strings.as_bytes());
    out
}

fn u32_de(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Lector secuencial con comprobación de límites.
struct Lector<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Lector<'a> {
    fn tomar(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        let fin = self
            .pos
            .checked_add(n)
            .ok_or(CodecError::Tamano("desborde"))?;
        let trozo = self.buf.get(self.pos..fin).ok_or(CodecError::Truncado {
            necesarios: fin as u64,
            hay: self.buf.len(),
        })?;
        self.pos = fin;
        Ok(trozo)
    }

    fn arr<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let t = self.tomar(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(t);
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.arr::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.arr()?))
    }
    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.arr()?))
    }
    fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.arr()?))
    }
    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.arr()?))
    }
    fn f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_le_bytes(self.arr()?))
    }
    fn f64(&mut self) -> Result<f64, CodecError> {
        Ok(f64::from_le_bytes(self.arr()?))
    }
}

/// Decodifica una lista en el formato v1. No entra en pánico con ninguna entrada.
pub fn decode(bytes: &[u8]) -> Result<DrawList, CodecError> {
    let mut l = Lector { buf: bytes, pos: 0 };
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::Truncado {
            necesarios: HEADER_LEN as u64,
            hay: bytes.len(),
        });
    }
    let magia = l.u32()?;
    if magia != MAGIC {
        return Err(CodecError::MagiaIncorrecta(magia));
    }
    let version = l.u16()?;
    if version != VERSION {
        return Err(CodecError::VersionNoSoportada(version));
    }
    if l.u16()? != 0 {
        return Err(CodecError::Reservado("banderas del encabezado"));
    }
    let data_version = l.u64()?;
    let sheet = l.u32()?;
    let first_row = l.i32()?;
    let first_col = l.i32()?;
    let n_rows = l.u32()?;
    let n_cols = l.u32()?;
    let n_styles = l.u32()?;
    let n_cells = l.u32()?;
    let strings_len = l.u32()?;
    let origin = (l.f64()?, l.f64()?);

    // El bloque debe caber en la hoja y no estar vacío.
    if n_rows == 0 || n_cols == 0 {
        return Err(CodecError::Valor("bloque vacío"));
    }
    if n_rows > MAX_ROWS as u32 || n_cols > MAX_COLS as u32 {
        return Err(CodecError::Valor("más filas o columnas que la hoja"));
    }
    if first_row < 1
        || first_col < 1
        || i64::from(first_row) + i64::from(n_rows) - 1 > i64::from(MAX_ROWS)
        || i64::from(first_col) + i64::from(n_cols) - 1 > i64::from(MAX_COLS)
    {
        return Err(CodecError::Valor("el bloque se sale de la hoja"));
    }
    if !origin.0.is_finite()
        || !origin.1.is_finite()
        || origin.0.is_sign_negative()
        || origin.1.is_sign_negative()
    {
        return Err(CodecError::Valor("origen"));
    }
    if u64::from(n_cells) > u64::from(n_rows) * u64::from(n_cols) {
        return Err(CodecError::Tamano("más celdas que posiciones"));
    }

    // Longitud total declarada, en u64 (no puede desbordar: cada término cabe en ~2^37).
    let (nr, nc) = (u64::from(n_rows), u64::from(n_cols));
    let total = HEADER_LEN as u64
        + 4 * (nc + 1)
        + 4 * (nr + 1)
        + 8 * (nc + nr)
        + STYLE_LEN as u64 * u64::from(n_styles)
        + CELL_LEN as u64 * u64::from(n_cells)
        + u64::from(strings_len);
    let hay = bytes.len() as u64;
    if total > hay {
        return Err(CodecError::Truncado {
            necesarios: total,
            hay: bytes.len(),
        });
    }
    if total < hay {
        return Err(CodecError::Tamano("bytes después de la sección de cadenas"));
    }

    // A partir de aquí todo lo que se reserva está acotado por `bytes.len()`.
    let col_edges = aristas(&mut l, n_cols as usize + 1)?;
    let row_edges = aristas(&mut l, n_rows as usize + 1)?;

    let mut col_labels = Vec::with_capacity(n_cols as usize);
    for _ in 0..n_cols {
        col_labels.push(StrRef {
            off: l.u32()?,
            len: l.u32()?,
        });
    }
    let mut row_labels = Vec::with_capacity(n_rows as usize);
    for _ in 0..n_rows {
        row_labels.push(StrRef {
            off: l.u32()?,
            len: l.u32()?,
        });
    }

    let mut styles = Vec::with_capacity(n_styles as usize);
    for _ in 0..n_styles {
        let fg = l.u32()?;
        let fill = l.u32()?;
        let font_px = l.f32()?;
        let family = l.u16()?;
        let flags = l.u8()?;
        let h = l.u8()?;
        let v = l.u8()?;
        if l.arr::<3>()? != [0, 0, 0] {
            return Err(CodecError::Reservado("bytes reservados de un estilo"));
        }
        if flags & !F_TODAS != 0 {
            return Err(CodecError::Reservado("banderas de estilo desconocidas"));
        }
        if !font_px.is_finite() || font_px.is_sign_negative() {
            return Err(CodecError::Valor("tamaño de letra"));
        }
        styles.push(CellStyle {
            fg: Rgba(fg),
            fill: if fill == 0 { None } else { Some(Rgba(fill)) },
            font_px,
            family,
            bold: flags & F_NEGRITA != 0,
            italic: flags & F_CURSIVA != 0,
            underline: flags & F_SUBRAYADO != 0,
            strike: flags & F_TACHADO != 0,
            wrap: flags & F_AJUSTE != 0,
            h_align: match h {
                0 => Align::Izq,
                1 => Align::Centro,
                2 => Align::Der,
                _ => return Err(CodecError::Valor("alineación horizontal")),
            },
            v_align: match v {
                0 => VAlign::Arriba,
                1 => VAlign::Centro,
                2 => VAlign::Abajo,
                _ => return Err(CodecError::Valor("alineación vertical")),
            },
        });
    }

    let mut cells = Vec::with_capacity(n_cells as usize);
    let mut anterior: Option<(u32, u32)> = None;
    for _ in 0..n_cells {
        let row_off = l.u32()?;
        let col_off = l.u32()?;
        let text = StrRef {
            off: l.u32()?,
            len: l.u32()?,
        };
        let style = l.u16()?;
        let kind = match l.u8()? {
            0 => Kind::Texto,
            1 => Kind::Numero,
            2 => Kind::Bool,
            3 => Kind::Error,
            4 => Kind::SoloEstilo,
            _ => return Err(CodecError::Valor("tipo de celda")),
        };
        let spill_cols = l.u8()?;
        if row_off >= n_rows || col_off >= n_cols {
            return Err(CodecError::Valor("celda fuera del bloque"));
        }
        if u32::from(style) >= n_styles {
            return Err(CodecError::Valor("estilo de celda inexistente"));
        }
        if spill_cols > MAX_SPILL {
            return Err(CodecError::Valor("desborde mayor que el tope"));
        }
        if anterior.is_some_and(|a| a >= (row_off, col_off)) {
            return Err(CodecError::Valor("celdas desordenadas o repetidas"));
        }
        anterior = Some((row_off, col_off));
        cells.push(DrawCell {
            row_off,
            col_off,
            text,
            style,
            kind,
            spill_cols,
        });
    }

    let crudas = l.tomar(strings_len as usize)?;
    let strings = std::str::from_utf8(crudas)
        .map_err(|_| CodecError::Utf8)?
        .to_owned();
    for r in col_labels
        .iter()
        .chain(&row_labels)
        .chain(cells.iter().map(|c| &c.text))
    {
        let ini = r.off as usize;
        let fin = ini.saturating_add(r.len as usize);
        if fin > strings.len() || !strings.is_char_boundary(ini) || !strings.is_char_boundary(fin) {
            return Err(CodecError::Valor("referencia a cadena fuera de rango"));
        }
    }

    Ok(DrawList {
        sheet,
        data_version,
        first_row,
        first_col,
        n_rows,
        n_cols,
        origin,
        col_edges,
        row_edges,
        col_labels,
        row_labels,
        styles,
        cells,
        strings,
    })
}

/// Lee `n` aristas: finitas, la primera 0 y sin decrecer.
fn aristas(l: &mut Lector<'_>, n: usize) -> Result<Vec<f32>, CodecError> {
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let e = l.f32()?;
        // `is_sign_positive` descarta -0.0, que sería igual a 0.0 pero con otros bytes.
        let valida =
            e.is_finite() && e.is_sign_positive() && if i == 0 { e == 0.0 } else { e >= v[i - 1] };
        if !valida {
            return Err(CodecError::Valor("aristas"));
        }
        v.push(e);
    }
    Ok(v)
}
