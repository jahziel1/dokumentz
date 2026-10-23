//! Lista de dibujo de un bloque de la cuadrícula (`plan.md` §5.4).
//!
//! El núcleo decide *qué* se ve (texto ya formateado, estilo ya resuelto, geometría,
//! etiquetas de cabecera, desborde del texto) y las interfaces solo pintan. Este módulo
//! define los tipos y construye un bloque desde el motor; `codec.rs` lo pasa a bytes.
//!
//! Decisiones que el plan deja abiertas:
//! * Tamaño de letra: `font_px` = puntos del archivo × `ROW_HEIGHT_FACTOR` (la misma conversión
//!   que el motor aplica a las alturas), así una letra de 16 pt llena la fila por defecto.
//! * `family` guarda la familia genérica del archivo (0 = sin indicar, 1 = con serifas,
//!   2 = sin serifas, 3 = monoespaciada…); el formato v1 no lleva nombres de fuente.
//! * Una celda con relleno pero sin texto se emite como `Kind::SoloEstilo`. También las que
//!   heredan relleno de su fila o columna entera (formato de fila/columna).
//! * Las celdas combinadas, los bordes y los formatos condicionales no entran en la v1.

use std::borrow::Cow;
use std::collections::HashMap;

use ironcalc_base::ROW_HEIGHT_FACTOR;
use ironcalc_base::UserModel;
use ironcalc_base::types::{
    Alignment, Cell, CellType, HorizontalAlignment, Style, VerticalAlignment, Worksheet,
};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::geometry::{MAX_COLS, MAX_ROWS, SheetGeometry};

/// Tope de filas de un bloque (protege de peticiones absurdas; una pantalla tiene cientos).
pub const MAX_BLOCK_ROWS: i32 = 20_000;
/// Tope de columnas de un bloque.
pub const MAX_BLOCK_COLS: i32 = 2_048;
/// Tope de columnas vacías que un texto largo puede invadir a su derecha.
pub const MAX_SPILL: u8 = 8;
/// Tope de bytes de cadenas de un bloque (queda muy por debajo de `u32::MAX`).
const MAX_STRINGS: usize = 1 << 30;

/// Color RGBA empaquetado como `0xRRGGBBAA` (rojo en el byte más alto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba(pub u32);

impl Rgba {
    /// Color opaco.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF)
    }

    /// Negro opaco, el color de letra por defecto.
    pub const NEGRO: Rgba = Rgba::rgb(0, 0, 0);

    /// Rojo, verde y azul (0..=255).
    pub const fn componentes(self) -> (u8, u8, u8) {
        (
            (self.0 >> 24) as u8,
            (self.0 >> 16) as u8,
            (self.0 >> 8) as u8,
        )
    }

    /// Lee `#RRGGBB` (lo que devuelve `resolve_color`). Cadena vacía u otro formato: `None`.
    pub fn from_hex(hex: &str) -> Option<Rgba> {
        let digitos = hex.strip_prefix('#')?;
        if digitos.len() != 6 || !digitos.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let v = u32::from_str_radix(digitos, 16).ok()?;
        Some(Rgba((v << 8) | 0xFF))
    }
}

/// Referencia a una cadena dentro de [`DrawList::strings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StrRef {
    /// Desplazamiento en bytes.
    pub off: u32,
    /// Longitud en bytes.
    pub len: u32,
}

/// Alineación horizontal ya resuelta (sin «General»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Align {
    /// A la izquierda.
    #[default]
    Izq = 0,
    /// Centrado.
    Centro = 1,
    /// A la derecha.
    Der = 2,
}

/// Alineación vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VAlign {
    /// Arriba.
    Arriba = 0,
    /// Centrado.
    Centro = 1,
    /// Abajo (el valor por defecto de Office).
    #[default]
    Abajo = 2,
}

/// Qué contiene una celda de la lista.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kind {
    /// Texto.
    Texto = 0,
    /// Número (el texto ya viene con el formato de número aplicado).
    Numero = 1,
    /// Valor lógico.
    Bool = 2,
    /// Error de fórmula (`#DIV/0!`…).
    Error = 3,
    /// Sin texto; solo tiene relleno.
    SoloEstilo = 4,
}

/// Estilo de una celda **ya resuelto**: sin temas, sin «General», sin índices del motor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CellStyle {
    /// Color de la letra.
    pub fg: Rgba,
    /// Color de relleno; `None` = sin relleno.
    pub fill: Option<Rgba>,
    /// Tamaño de la letra en px lógicos de hoja.
    pub font_px: f32,
    /// Familia genérica de la fuente (ver el comentario del módulo).
    pub family: u16,
    /// Negrita.
    pub bold: bool,
    /// Cursiva.
    pub italic: bool,
    /// Subrayado.
    pub underline: bool,
    /// Tachado.
    pub strike: bool,
    /// Ajustar texto a varias líneas.
    pub wrap: bool,
    /// Alineación horizontal resuelta.
    pub h_align: Align,
    /// Alineación vertical.
    pub v_align: VAlign,
}

/// Una celda dispersa del bloque: solo las que tienen texto o relleno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCell {
    /// Fila relativa al bloque (0 = `first_row`).
    pub row_off: u32,
    /// Columna relativa al bloque (0 = `first_col`).
    pub col_off: u32,
    /// Texto ya formateado.
    pub text: StrRef,
    /// Índice en [`DrawList::styles`].
    pub style: u16,
    /// Tipo de contenido.
    pub kind: Kind,
    /// Columnas vacías contiguas a la derecha que el texto puede invadir (solo texto alineado
    /// a la izquierda y sin ajuste de línea; tope [`MAX_SPILL`]).
    pub spill_cols: u8,
}

/// Qué área de la hoja pinta la interfaz. Todo en px de hoja.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawRequest {
    /// Hoja a dibujar.
    pub sheet: u32,
    /// Borde izquierdo del área visible.
    pub x0: f64,
    /// Borde superior del área visible.
    pub y0: f64,
    /// Ancho del área visible.
    pub w: f32,
    /// Alto del área visible.
    pub h: f32,
    /// Margen horizontal que se añade a cada lado para precargar.
    pub margin_x: f32,
    /// Margen vertical que se añade arriba y abajo.
    pub margin_y: f32,
}

/// Bloque de la cuadrícula listo para pintar.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawList {
    /// Hoja a la que pertenece.
    pub sheet: u32,
    /// Versión de datos con la que se construyó (para invalidar cachés).
    pub data_version: u64,
    /// Primera fila del bloque (base 1).
    pub first_row: i32,
    /// Primera columna del bloque (base 1).
    pub first_col: i32,
    /// Número de filas.
    pub n_rows: u32,
    /// Número de columnas.
    pub n_cols: u32,
    /// Posición de hoja (x, y) de la esquina superior izquierda del bloque.
    pub origin: (f64, f64),
    /// `n_cols + 1` aristas verticales, relativas a `origin.0`.
    pub col_edges: Vec<f32>,
    /// `n_rows + 1` aristas horizontales, relativas a `origin.1`.
    pub row_edges: Vec<f32>,
    /// Etiquetas de columna («A», «B»…), una por columna del bloque.
    pub col_labels: Vec<StrRef>,
    /// Etiquetas de fila («1», «2»…), una por fila del bloque.
    pub row_labels: Vec<StrRef>,
    /// Estilos sin repetir.
    pub styles: Vec<CellStyle>,
    /// Celdas con texto o relleno, ordenadas por fila y luego por columna.
    pub cells: Vec<DrawCell>,
    /// Todas las cadenas, concatenadas.
    pub strings: String,
}

impl DrawList {
    /// Texto al que apunta una referencia (vacío si queda fuera de rango o parte un carácter).
    pub fn text(&self, r: StrRef) -> &str {
        let ini = r.off as usize;
        let fin = ini.saturating_add(r.len as usize);
        self.strings.get(ini..fin).unwrap_or("")
    }

    /// Texto de una celda.
    pub fn cell_text(&self, c: &DrawCell) -> &str {
        self.text(c.text)
    }

    /// Etiqueta de la columna `i` del bloque.
    pub fn col_label(&self, i: usize) -> &str {
        self.col_labels.get(i).map_or("", |r| self.text(*r))
    }

    /// Etiqueta de la fila `i` del bloque.
    pub fn row_label(&self, i: usize) -> &str {
        self.row_labels.get(i).map_or("", |r| self.text(*r))
    }

    /// Última columna del bloque (base 1).
    pub fn last_col(&self) -> i32 {
        self.first_col + self.n_cols as i32 - 1
    }

    /// Última fila del bloque (base 1).
    pub fn last_row(&self) -> i32 {
        self.first_row + self.n_rows as i32 - 1
    }

    /// Celda de la lista en la posición de hoja `(row, col)` (base 1), si tiene texto o relleno.
    pub fn find_cell(&self, row: i32, col: i32) -> Option<&DrawCell> {
        let ro = u32::try_from(row.checked_sub(self.first_row)?).ok()?;
        let co = u32::try_from(col.checked_sub(self.first_col)?).ok()?;
        let i = self
            .cells
            .binary_search_by_key(&(ro, co), |c| (c.row_off, c.col_off))
            .ok()?;
        self.cells.get(i)
    }

    /// ¿Este bloque contiene toda el área visible de `req`? Un bloque que llega a un borde de
    /// la hoja (fila 1, última fila, columna 1, última columna) cubre «hasta el infinito» en
    /// esa dirección. Los márgenes de `req` no cuentan: solo el área visible.
    pub fn covers(&self, req: &DrawRequest) -> bool {
        if req.sheet != self.sheet || self.n_rows == 0 || self.n_cols == 0 {
            return false;
        }
        let x_lo = req.x0.max(0.0);
        let y_lo = req.y0.max(0.0);
        let x_hi = req.x0 + f64::from(req.w);
        let y_hi = req.y0 + f64::from(req.h);
        let bloque_x0 = self.origin.0;
        let bloque_y0 = self.origin.1;
        let bloque_x1 = bloque_x0 + f64::from(self.col_edges.last().copied().unwrap_or(0.0));
        let bloque_y1 = bloque_y0 + f64::from(self.row_edges.last().copied().unwrap_or(0.0));
        let izq = self.first_col == 1 || x_lo >= bloque_x0;
        let arr = self.first_row == 1 || y_lo >= bloque_y0;
        let der = self.last_col() >= MAX_COLS || x_hi <= bloque_x1;
        let aba = self.last_row() >= MAX_ROWS || y_hi <= bloque_y1;
        izq && arr && der && aba
    }
}

/// Etiqueta de columna estilo hoja de cálculo: 1 → «A», 26 → «Z», 27 → «AA», 16384 → «XFD».
pub fn col_label(col: i32) -> String {
    let mut n = col.max(0);
    let mut letras = Vec::new();
    while n > 0 {
        n -= 1;
        letras.push(b'A' + (n % 26) as u8);
        n /= 26;
    }
    letras.reverse();
    String::from_utf8(letras).unwrap_or_default()
}

/// Construye el bloque que pide `req` sobre el motor. `geom` es la geometría de la hoja.
///
/// El bloque cubre el área visible más los márgenes y se recorta a la hoja: si `req` queda
/// fuera por la derecha o por abajo, devuelve las últimas fila y columna. Las filas y
/// columnas ocultas entran en las aristas con ancho 0 pero no llevan celdas.
pub fn build_block(
    um: &UserModel<'static>,
    geom: &SheetGeometry,
    req: &DrawRequest,
    data_version: u64,
) -> Result<DrawList, CoreError> {
    let ws = um
        .get_model()
        .workbook
        .worksheets
        .get(req.sheet as usize)
        .ok_or(CoreError::HojaInvalida(req.sheet))?;

    // Área del bloque en px de hoja.
    let x_lo = (req.x0 - f64::from(req.margin_x)).max(0.0);
    let y_lo = (req.y0 - f64::from(req.margin_y)).max(0.0);
    let x_hi = (req.x0 + f64::from(req.w) + f64::from(req.margin_x)).max(x_lo);
    let y_hi = (req.y0 + f64::from(req.h) + f64::from(req.margin_y)).max(y_lo);

    let first_col = geom.col_at(x_lo);
    let first_row = geom.row_at(y_lo);
    let last_col = geom
        .col_at(x_hi)
        .max(first_col)
        .min(first_col + MAX_BLOCK_COLS - 1);
    let last_row = geom
        .row_at(y_hi)
        .max(first_row)
        .min(first_row + MAX_BLOCK_ROWS - 1);
    let n_cols = (last_col - first_col + 1) as usize;
    let n_rows = (last_row - first_row + 1) as usize;
    let origin = (geom.col_x(first_col), geom.row_y(first_row));

    // Aristas relativas al origen y anchos/altos (0 = oculto).
    let mut col_edges = Vec::with_capacity(n_cols + 1);
    let mut col_w = Vec::with_capacity(n_cols);
    let mut acc = 0.0_f64;
    col_edges.push(0.0_f32);
    for c in first_col..=last_col {
        let w = geom.col_w(c);
        col_w.push(w);
        acc += w;
        col_edges.push(acc as f32);
    }
    let mut row_edges = Vec::with_capacity(n_rows + 1);
    let mut row_h = Vec::with_capacity(n_rows);
    acc = 0.0;
    row_edges.push(0.0_f32);
    for r in first_row..=last_row {
        let h = geom.row_h(r);
        row_h.push(h);
        acc += h;
        row_edges.push(acc as f32);
    }

    let mut b = Builder {
        um,
        ws,
        sheet: req.sheet,
        styles: Vec::new(),
        por_clave: HashMap::new(),
        strings: String::new(),
    };

    // Etiquetas de cabecera.
    let mut col_labels = Vec::with_capacity(n_cols);
    for c in first_col..=last_col {
        col_labels.push(b.cadena(&col_label(c)));
    }
    let mut row_labels = Vec::with_capacity(n_rows);
    for r in first_row..=last_row {
        row_labels.push(b.cadena(&r.to_string()));
    }

    // Formato de fila o columna entera (celdas sin dato pero con relleno heredado).
    let implicitos = Implicitos::new(ws, first_row, last_row, first_col, last_col);
    let sin_implicitos = implicitos.vacio();

    let mut cells: Vec<DrawCell> = Vec::new();
    for (ro, r) in (first_row..=last_row).enumerate() {
        if row_h[ro] <= 0.0 {
            continue;
        }
        let existentes: Vec<(i32, Cow<'_, Cell>)> = ws
            .sheet_data
            .cells_in_row_between(r, first_col, last_col)
            .collect();
        if sin_implicitos {
            for (c, cell) in &existentes {
                let co = (c - first_col) as usize;
                if col_w[co] > 0.0 {
                    b.celda(
                        &mut cells,
                        (r, *c),
                        (ro as u32, co as u32),
                        Some(&**cell),
                        0,
                    );
                }
            }
        } else {
            let mut siguiente = existentes.iter().peekable();
            for (co, c) in (first_col..=last_col).enumerate() {
                if col_w[co] <= 0.0 {
                    while siguiente.next_if(|(cc, _)| *cc <= c).is_some() {}
                    continue;
                }
                let cell = siguiente.next_if(|(cc, _)| *cc == c).map(|(_, cell)| cell);
                let implicito = if cell.is_none() {
                    implicitos.estilo(r, c)
                } else {
                    0
                };
                if cell.is_none() && implicito == 0 {
                    continue;
                }
                let cell = cell.map(|c| &**c);
                b.celda(&mut cells, (r, c), (ro as u32, co as u32), cell, implicito);
            }
        }
        if b.strings.len() > MAX_STRINGS {
            break;
        }
    }

    Ok(DrawList {
        sheet: req.sheet,
        data_version,
        first_row,
        first_col,
        n_rows: n_rows as u32,
        n_cols: n_cols as u32,
        origin,
        col_edges,
        row_edges,
        col_labels,
        row_labels,
        styles: b.styles,
        cells,
        strings: b.strings,
    })
}

/// Estado mientras se arma un bloque.
struct Builder<'a> {
    um: &'a UserModel<'static>,
    ws: &'a Worksheet,
    sheet: u32,
    styles: Vec<CellStyle>,
    /// (índice de estilo del motor, tipo) → índice en `styles`. El tipo entra en la clave
    /// porque la alineación «General» depende de él.
    por_clave: HashMap<(i32, Kind), u16>,
    strings: String,
}

impl Builder<'_> {
    fn cadena(&mut self, s: &str) -> StrRef {
        let off = u32::try_from(self.strings.len()).unwrap_or(u32::MAX);
        self.strings.push_str(s);
        StrRef {
            off,
            len: u32::try_from(s.len()).unwrap_or(u32::MAX),
        }
    }

    /// Agrega la celda `(r, c)` si tiene texto o relleno. `cell` = `None` para celdas sin
    /// dato que heredan estilo de su fila o columna.
    fn celda(
        &mut self,
        out: &mut Vec<DrawCell>,
        pos: (i32, i32),
        off: (u32, u32),
        cell: Option<&Cell>,
        estilo_implicito: i32,
    ) {
        let (r, c) = pos;
        let (row_off, col_off) = off;
        let mut kind = match cell {
            None | Some(Cell::EmptyCell { .. }) => Kind::SoloEstilo,
            Some(cell) => match cell.get_type() {
                CellType::Number => Kind::Numero,
                CellType::LogicalValue => Kind::Bool,
                CellType::ErrorValue => Kind::Error,
                _ => Kind::Texto,
            },
        };
        let texto = if kind == Kind::SoloEstilo {
            String::new()
        } else {
            self.um
                .get_formatted_cell_value(self.sheet, r, c)
                .unwrap_or_default()
        };
        if texto.is_empty() {
            kind = Kind::SoloEstilo;
        }
        let estilo_motor = cell.map_or(estilo_implicito, Cell::get_style);
        let style = self.estilo(estilo_motor, kind, r, c);
        if kind == Kind::SoloEstilo && self.styles[style as usize].fill.is_none() {
            return; // sin texto ni relleno: no se dibuja
        }
        let spill_cols = {
            let st = &self.styles[style as usize];
            if kind == Kind::Texto && st.h_align == Align::Izq && !st.wrap {
                self.columnas_libres(r, c)
            } else {
                0
            }
        };
        let text = if texto.is_empty() {
            StrRef::default()
        } else {
            self.cadena(&texto)
        };
        out.push(DrawCell {
            row_off,
            col_off,
            text,
            style,
            kind,
            spill_cols,
        });
    }

    /// Columnas vacías contiguas a la derecha de `(r, c)`, hasta [`MAX_SPILL`].
    fn columnas_libres(&self, r: i32, c: i32) -> u8 {
        let mut n = 0;
        while n < MAX_SPILL && c + i32::from(n) < MAX_COLS {
            let vacia = match self.ws.sheet_data.cell(r, c + i32::from(n) + 1) {
                None => true,
                Some(cell) => matches!(&*cell, Cell::EmptyCell { .. }),
            };
            if !vacia {
                break;
            }
            n += 1;
        }
        n
    }

    /// Índice en `styles` del estilo del motor `idx` para una celda de tipo `kind`. La primera
    /// vez resuelve el estilo desde la posición `(r, c)`, que tiene ese índice.
    fn estilo(&mut self, idx: i32, kind: Kind, r: i32, c: i32) -> u16 {
        if let Some(&i) = self.por_clave.get(&(idx, kind)) {
            return i;
        }
        let motor = self
            .um
            .get_model()
            .get_style_for_cell(self.sheet, r, c)
            .unwrap_or_default();
        let resuelto = resolver_estilo(self.um, &motor, kind);
        let i = match self.styles.iter().position(|s| *s == resuelto) {
            Some(i) => i,
            None => {
                self.styles.push(resuelto);
                self.styles.len() - 1
            }
        };
        let i = u16::try_from(i).unwrap_or(0);
        self.por_clave.insert((idx, kind), i);
        i
    }
}

/// Convierte un estilo del motor en el estilo resuelto del bloque.
fn resolver_estilo(um: &UserModel<'static>, s: &Style, kind: Kind) -> CellStyle {
    let fg = Rgba::from_hex(&um.resolve_color(&s.font.color)).unwrap_or(Rgba::NEGRO);
    let fill = Rgba::from_hex(&um.resolve_color(&s.fill.color));
    let pt = if s.font.sz > 0 { s.font.sz } else { 11 };
    CellStyle {
        fg,
        fill,
        font_px: (f64::from(pt) * ROW_HEIGHT_FACTOR) as f32,
        family: u16::try_from(s.font.family.max(0)).unwrap_or(0),
        bold: s.font.b,
        italic: s.font.i,
        underline: s.font.u,
        strike: s.font.strike,
        wrap: s.alignment.as_ref().is_some_and(|a| a.wrap_text),
        h_align: alineacion_horizontal(s.alignment.as_ref(), kind),
        v_align: alineacion_vertical(s.alignment.as_ref()),
    }
}

/// Alineación horizontal. «General» se resuelve por tipo, como en Office: número a la derecha,
/// texto a la izquierda, lógico y error centrados.
fn alineacion_horizontal(a: Option<&Alignment>, kind: Kind) -> Align {
    match a.map_or(&HorizontalAlignment::General, |a| &a.horizontal) {
        HorizontalAlignment::Left
        | HorizontalAlignment::Justify
        | HorizontalAlignment::Distributed
        | HorizontalAlignment::Fill => Align::Izq,
        HorizontalAlignment::Right => Align::Der,
        HorizontalAlignment::Center | HorizontalAlignment::CenterContinuous => Align::Centro,
        HorizontalAlignment::General => match kind {
            Kind::Numero => Align::Der,
            Kind::Bool | Kind::Error => Align::Centro,
            Kind::Texto | Kind::SoloEstilo => Align::Izq,
        },
    }
}

fn alineacion_vertical(a: Option<&Alignment>) -> VAlign {
    match a.map_or(&VerticalAlignment::Bottom, |a| &a.vertical) {
        VerticalAlignment::Top => VAlign::Arriba,
        VerticalAlignment::Bottom => VAlign::Abajo,
        VerticalAlignment::Center | VerticalAlignment::Distributed | VerticalAlignment::Justify => {
            VAlign::Centro
        }
    }
}

/// Formatos de fila y de columna entera que afectan a celdas sin dato del bloque.
struct Implicitos {
    /// `(fila, índice de estilo)` de las filas con `custom_format` dentro del bloque.
    filas: Vec<(i32, i32)>,
    /// `(min, max, estilo)` de las columnas con estilo que tocan el bloque.
    columnas: Vec<(i32, i32, i32)>,
}

impl Implicitos {
    fn new(ws: &Worksheet, r0: i32, r1: i32, c0: i32, c1: i32) -> Implicitos {
        let mut filas: Vec<(i32, i32)> = ws
            .rows
            .iter()
            .filter(|f| f.custom_format && (r0..=r1).contains(&f.r))
            .map(|f| (f.r, f.s))
            .collect();
        filas.sort_by_key(|(r, _)| *r);
        let columnas = ws
            .cols
            .iter()
            .filter(|c| c.min <= c1 && c.max >= c0)
            .map(|c| (c.min, c.max, c.style.unwrap_or(0)))
            .collect();
        Implicitos { filas, columnas }
    }

    /// ¿Ninguna fila ni columna del bloque trae formato propio (distinto de 0)?
    fn vacio(&self) -> bool {
        self.filas.iter().all(|(_, s)| *s == 0) && self.columnas.iter().all(|(_, _, s)| *s == 0)
    }

    /// Índice de estilo de una celda sin dato: el de su fila si tiene formato, si no el de la
    /// primera columna que la contiene (la misma regla que `get_cell_style_index` del motor).
    fn estilo(&self, r: i32, c: i32) -> i32 {
        if let Ok(i) = self.filas.binary_search_by_key(&r, |(f, _)| *f) {
            return self.filas[i].1;
        }
        self.columnas
            .iter()
            .find(|(min, max, _)| (*min..=*max).contains(&c))
            .map_or(0, |(_, _, s)| *s)
    }
}
