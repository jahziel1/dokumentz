//! Geometría de una hoja: posición y tamaño de filas y columnas (`plan.md` §5.3).
//!
//! Todo está en **píxeles lógicos de hoja** (los de IronCalc: ancho de columna =
//! caracteres × 9, alto de fila = puntos × 1,5625; por defecto 90 × 25). La estructura
//! guarda solo las filas y columnas cuyo tamaño difiere del valor por defecto, ordenadas,
//! con la suma acumulada de sus diferencias. Con `k` excepciones, `col_x`/`row_y` y
//! `col_at`/`row_at` cuestan `O(log k)`; con cero excepciones, `O(1)`.
//!
//! Las posiciones usan `f64`: con 1 048 576 filas de 25 px el borde inferior pasa de 26 millones
//! de píxeles, más de lo que un `f32` representa sin perder la fracción.
//!
//! Hallazgo 9 de T006: se construye leyendo directamente `rows`/`cols` de la hoja y **no**
//! llamando a `get_row_height` fila por fila (esa función es lineal en el número de filas con
//! entrada, así que hacerlo por cada fila sería cuadrático).

use ironcalc_base::types::Worksheet;
use ironcalc_base::{COLUMN_WIDTH_FACTOR, ROW_HEIGHT_FACTOR};

use crate::view::View;

/// Ancho, en píxeles lógicos, de la cabecera de filas (la columna de números `1, 2, 3…`).
pub const ROW_HEADER_W: f32 = 46.0;
/// Alto, en píxeles lógicos, de la cabecera de columnas (la fila `A, B, C…`).
pub const COL_HEADER_H: f32 = 25.0;
/// Número de filas del motor (base 1: la última es `MAX_ROWS`).
pub const MAX_ROWS: i32 = 1_048_576;
/// Número de columnas del motor (base 1: la última es `MAX_COLS`).
pub const MAX_COLS: i32 = 16_384;
/// Ancho por defecto de una columna no definida en el archivo (`constants.rs` del motor).
pub const DEFAULT_COL_W: f64 = 90.0;
/// Alto por defecto de una fila no definida en el archivo.
pub const DEFAULT_ROW_H: f64 = 25.0;

/// Tolerancia para que `at(start(i)) == i` aunque la suma de tamaños tenga error de redondeo.
const EPS: f64 = 1e-9;

/// Un eje (filas o columnas): tamaños por defecto más excepciones ordenadas.
#[derive(Debug, Clone)]
struct Axis {
    default: f64,
    /// Número de elementos del eje (`MAX_ROWS` o `MAX_COLS`).
    count: i32,
    /// Índices (base 1) con tamaño propio, ordenados y sin repetir.
    idx: Vec<i32>,
    /// Tamaño de cada excepción, en el mismo orden que `idx`.
    size: Vec<f64>,
    /// `cum[k]` = suma de `(size[j] - default)` para `j < k`. Tiene `idx.len() + 1` elementos.
    cum: Vec<f64>,
}

impl Axis {
    /// `overrides` puede venir desordenado y con repetidos: gana la primera aparición de cada
    /// índice (como en `Worksheet::row_height`, que devuelve la primera fila que coincide).
    /// Se descartan los índices fuera de `1..=count`, los tamaños no válidos y los que
    /// coinciden con el valor por defecto.
    fn new(default: f64, count: i32, mut overrides: Vec<(i32, f64)>) -> Axis {
        let default = if default.is_finite() && default > 0.0 {
            default
        } else {
            1.0
        };
        // Orden estable por índice: la primera aparición queda primero.
        overrides.sort_by_key(|(i, _)| *i);
        overrides.dedup_by_key(|(i, _)| *i);
        let mut idx = Vec::with_capacity(overrides.len());
        let mut size = Vec::with_capacity(overrides.len());
        for (i, s) in overrides {
            if !(1..=count).contains(&i) || !s.is_finite() || s < 0.0 || s == default {
                continue;
            }
            idx.push(i);
            size.push(s);
        }
        let mut cum = Vec::with_capacity(idx.len() + 1);
        let mut acc = 0.0;
        cum.push(acc);
        for s in &size {
            acc += s - default;
            cum.push(acc);
        }
        Axis {
            default,
            count,
            idx,
            size,
            cum,
        }
    }

    /// Posición de inicio del elemento `i` (base 1). `i = count + 1` da el tamaño total.
    fn start(&self, i: i32) -> f64 {
        let i = i.clamp(1, self.count + 1);
        let k = self.idx.partition_point(|&j| j < i);
        f64::from(i - 1) * self.default + self.cum[k]
    }

    /// Tamaño del elemento `i` (0 para ocultos).
    fn size_of(&self, i: i32) -> f64 {
        match self.idx.binary_search(&i) {
            Ok(k) => self.size[k],
            Err(_) => self.default,
        }
    }

    fn total(&self) -> f64 {
        self.start(self.count + 1)
    }

    /// Elemento (base 1) que contiene la posición `p` (intervalos cerrados por la izquierda).
    /// `p <= 0` da 1 y `p >= total` da el último. Un elemento de tamaño 0 (oculto) nunca se
    /// devuelve: se salta al siguiente visible.
    fn at(&self, p: f64) -> i32 {
        if p.is_nan() || p <= 0.0 {
            return 1;
        }
        let k = self.exceptions_started_by(p);
        let found = if k == 0 {
            (p / self.default + EPS).floor() + 1.0
        } else {
            let j = k - 1;
            let start = f64::from(self.idx[j] - 1) * self.default + self.cum[j];
            let end = start + self.size[j];
            if p < end {
                f64::from(self.idx[j])
            } else {
                f64::from(self.idx[j]) + 1.0 + ((p - end) / self.default + EPS).floor()
            }
        };
        if found >= f64::from(self.count) {
            self.count
        } else {
            (found as i32).max(1)
        }
    }

    /// Número de excepciones cuyo inicio es `<= p` (búsqueda binaria; el inicio de cada
    /// excepción crece con su posición porque los tamaños no son negativos).
    fn exceptions_started_by(&self, p: f64) -> usize {
        let (mut lo, mut hi) = (0, self.idx.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            let start = f64::from(self.idx[mid] - 1) * self.default + self.cum[mid];
            if start <= p {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }
}

/// Geometría de una hoja.
#[derive(Debug, Clone)]
pub struct SheetGeometry {
    cols: Axis,
    rows: Axis,
    used_rows: i32,
    used_cols: i32,
}

impl Default for SheetGeometry {
    /// Hoja vacía con tamaños por defecto.
    fn default() -> Self {
        SheetGeometry::new(DEFAULT_COL_W, DEFAULT_ROW_H, Vec::new(), Vec::new(), 0, 0)
    }
}

impl SheetGeometry {
    /// Construye desde tamaños por defecto y excepciones `(índice base 1, tamaño en px
    /// lógicos)`. Las excepciones pueden venir desordenadas y repetidas (gana la primera).
    /// `used_rows`/`used_cols` son la última fila y columna con dato (0 si la hoja está vacía).
    pub fn new(
        default_w: f64,
        default_h: f64,
        col_overrides: Vec<(i32, f64)>,
        row_overrides: Vec<(i32, f64)>,
        used_rows: i32,
        used_cols: i32,
    ) -> SheetGeometry {
        SheetGeometry {
            cols: Axis::new(default_w, MAX_COLS, col_overrides),
            rows: Axis::new(default_h, MAX_ROWS, row_overrides),
            used_rows: used_rows.clamp(0, MAX_ROWS),
            used_cols: used_cols.clamp(0, MAX_COLS),
        }
    }

    /// Construye leyendo `rows`, `cols` y los datos de la hoja del motor. Mismas reglas que
    /// `Worksheet::row_height` / `get_column_width`: oculto = 0, fila con entrada =
    /// puntos × `ROW_HEIGHT_FACTOR`, columna con `custom_width` = caracteres ×
    /// `COLUMN_WIDTH_FACTOR`; gana la primera entrada que contiene la fila o columna.
    pub fn from_worksheet(ws: &Worksheet) -> SheetGeometry {
        let row_overrides: Vec<(i32, f64)> = ws
            .rows
            .iter()
            .map(|r| {
                let h = if r.hidden {
                    0.0
                } else {
                    r.height * ROW_HEIGHT_FACTOR
                };
                (r.r, h)
            })
            .collect();

        // Las columnas vienen en rangos [min, max]: se expanden (hay como mucho 16 384) y gana
        // el primer rango que contiene cada columna, aunque dé el valor por defecto.
        let mut col_sizes: Vec<Option<f64>> = Vec::new();
        for c in &ws.cols {
            let lo = c.min.max(1);
            let hi = c.max.min(MAX_COLS);
            if lo > hi {
                continue;
            }
            if col_sizes.len() < hi as usize {
                col_sizes.resize(hi as usize, None);
            }
            let size = if c.hidden {
                0.0
            } else if c.custom_width {
                c.width * COLUMN_WIDTH_FACTOR
            } else {
                DEFAULT_COL_W
            };
            for slot in &mut col_sizes[(lo - 1) as usize..hi as usize] {
                if slot.is_none() {
                    *slot = Some(size);
                }
            }
        }
        let col_overrides: Vec<(i32, f64)> = col_sizes
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i as i32 + 1, s)))
            .collect();

        let (used_rows, used_cols) = used_range(ws);
        SheetGeometry::new(
            DEFAULT_COL_W,
            DEFAULT_ROW_H,
            col_overrides,
            row_overrides,
            used_rows,
            used_cols,
        )
    }

    /// Posición x (px de hoja) del borde izquierdo de la columna `col` (base 1).
    /// `col_x(1) == 0`. Con `col = MAX_COLS + 1` da el ancho total de la hoja.
    pub fn col_x(&self, col: i32) -> f64 {
        self.cols.start(col)
    }

    /// Posición y (px de hoja) del borde superior de la fila `row` (base 1).
    pub fn row_y(&self, row: i32) -> f64 {
        self.rows.start(row)
    }

    /// Ancho de la columna `col` (0 si está oculta).
    pub fn col_w(&self, col: i32) -> f64 {
        self.cols.size_of(col)
    }

    /// Alto de la fila `row` (0 si está oculta).
    pub fn row_h(&self, row: i32) -> f64 {
        self.rows.size_of(row)
    }

    /// Columna (base 1) que contiene la coordenada x de hoja. Antes de 0 da 1; pasado el
    /// borde derecho de la hoja da `MAX_COLS`. Las ocultas se saltan.
    pub fn col_at(&self, x: f64) -> i32 {
        self.cols.at(x)
    }

    /// Fila (base 1) que contiene la coordenada y de hoja. Ver [`SheetGeometry::col_at`].
    pub fn row_at(&self, y: f64) -> i32 {
        self.rows.at(y)
    }

    /// Ancho total de todas las columnas de la hoja.
    pub fn total_w(&self) -> f64 {
        self.cols.total()
    }

    /// Alto total de todas las filas de la hoja.
    pub fn total_h(&self) -> f64 {
        self.rows.total()
    }

    /// Última fila con dato (0 si la hoja está vacía).
    pub fn used_rows(&self) -> i32 {
        self.used_rows
    }

    /// Última columna con dato (0 si la hoja está vacía).
    pub fn used_cols(&self) -> i32 {
        self.used_cols
    }

    /// Número de columnas con ancho distinto del valor por defecto.
    pub fn col_exceptions(&self) -> usize {
        self.cols.idx.len()
    }

    /// Número de filas con alto distinto del valor por defecto.
    pub fn row_exceptions(&self) -> usize {
        self.rows.idx.len()
    }

    /// Extensión desplazable `(ancho, alto)` en px de hoja: el rango usado más una pantalla
    /// de holgura (el área de celdas de `view`), sin pasar del tamaño total de la hoja.
    pub fn extent(&self, view: &View) -> (f64, f64) {
        let used_w = if self.used_cols > 0 {
            self.col_x(self.used_cols + 1)
        } else {
            0.0
        };
        let used_h = if self.used_rows > 0 {
            self.row_y(self.used_rows + 1)
        } else {
            0.0
        };
        (
            (used_w + view.cells_w()).min(self.total_w()),
            (used_h + view.cells_h()).min(self.total_h()),
        )
    }
}

/// Última fila y columna con dato de la hoja (`(0, 0)` si no tiene celdas).
fn used_range(ws: &Worksheet) -> (i32, i32) {
    if ws.sheet_data.is_empty() {
        return (0, 0);
    }
    let dim = ws.dimension();
    (dim.max_row, dim.max_column)
}
