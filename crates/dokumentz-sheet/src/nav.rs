//! Selección, navegación y puntería (`hit_test`) sobre `UserModel` (`plan.md` §5.5 y §5.6).
//!
//! La selección vive en `UserModel` (celda activa o «ancla», extremo móvil o «foco» y rango
//! normalizado); este módulo la expone como un tipo propio, [`Selection`], porque el
//! `SelectedView` de IronCalc no se puede nombrar fuera de su crate y no lleva el foco
//! (hallazgo 4 de T006). Las funciones de movimiento de `UserModel` hacen la parte de
//! selección; el desplazamiento lo decide el núcleo.
//!
//! # Dos desplazamientos que deben coincidir
//!
//! El núcleo se desplaza en **píxeles de hoja** (`View::scroll_x/y`); `UserModel` guarda solo la
//! **celda superior izquierda** visible (`top_row`, `left_column`) y la usa para RePág/AvPág y
//! para sus propias decisiones de desplazamiento. La regla es una sola: en todo momento,
//! `top_row == row_at(scroll_y)` y `left_column == col_at(scroll_x)`. `Core` la mantiene
//! (`sync_um_scroll`) cada vez que cambia el desplazamiento, y tras cada movimiento descarta lo
//! que `UserModel` haya decidido sobre el desplazamiento (salvo la fila superior de una página,
//! que es justo lo que RePág/AvPág deben mover) y aplica [`Core::ensure_visible`].
//!
//! # Desplazamiento por teclado
//!
//! Como en Office, al navegar con el teclado el desplazamiento avanza por filas y columnas
//! **enteras** y lo mínimo necesario: una celda que se sale por abajo o por la derecha deja
//! su borde inferior (derecho) pegado al de la ventana; una que se sale por arriba o por la
//! izquierda, su borde superior (izquierdo) pegado al de la ventana. Si la celda ya se ve
//! entera no se toca nada (así se conserva el desplazamiento fino del trackpad).
//! Con una selección extendida lo que se mantiene visible es el **foco**, no la celda activa
//! (en Office, Mayús+RePág desde A1 deja A1 fuera de la pantalla); con la selección de filas o
//! columnas enteras, solo el eje que no abarca por completo.

use ironcalc_base::worksheet::NavigationDirection;
use serde::{Deserialize, Serialize};

use crate::core::Core;
use crate::error::CoreError;
use crate::geometry::{COL_HEADER_H, MAX_COLS, MAX_ROWS, ROW_HEADER_W};

/// Tolerancia al comparar posiciones en píxeles.
const EPS: f64 = 1e-6;

/// Dirección de una flecha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dir {
    /// Hacia arriba.
    Up,
    /// Hacia abajo.
    Down,
    /// Hacia la izquierda.
    Left,
    /// Hacia la derecha.
    Right,
}

impl Dir {
    /// `KeyboardEvent.key` que espera `on_expand_selected_range` (hallazgo 3 de T006).
    fn key(self) -> &'static str {
        match self {
            Dir::Up => "ArrowUp",
            Dir::Down => "ArrowDown",
            Dir::Left => "ArrowLeft",
            Dir::Right => "ArrowRight",
        }
    }

    fn engine(self) -> NavigationDirection {
        match self {
            Dir::Up => NavigationDirection::Up,
            Dir::Down => NavigationDirection::Down,
            Dir::Left => NavigationDirection::Left,
            Dir::Right => NavigationDirection::Right,
        }
    }
}

/// Selección de la hoja visible. Filas y columnas son base 1 (A1 = `(1, 1)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Hoja (base 0).
    pub sheet: u32,
    /// Celda activa (fila, columna): el ancla de la selección, donde escribe el teclado.
    pub active: (i32, i32),
    /// Extremo móvil de la selección (fila, columna): lo que desplazan Mayús+flecha o el
    /// arrastre. Es igual a `active` si la selección es de una celda.
    pub focus: (i32, i32),
    /// Rango normalizado `(fila_ini, col_ini, fila_fin, col_fin)`, ambos extremos incluidos.
    pub range: (i32, i32, i32, i32),
}

impl Selection {
    /// ¿Es una sola celda?
    pub fn is_single_cell(&self) -> bool {
        self.range.0 == self.range.2 && self.range.1 == self.range.3
    }

    /// ¿Está `(fila, col)` dentro del rango?
    pub fn contains(&self, row: i32, col: i32) -> bool {
        (self.range.0..=self.range.2).contains(&row) && (self.range.1..=self.range.3).contains(&col)
    }

    /// ¿Abarca todas las columnas? (selección de fila o filas enteras).
    pub fn is_entire_rows(&self) -> bool {
        self.range.1 == 1 && self.range.3 == MAX_COLS
    }

    /// ¿Abarca todas las filas? (selección de columna o columnas enteras).
    pub fn is_entire_cols(&self) -> bool {
        self.range.0 == 1 && self.range.2 == MAX_ROWS
    }

    /// Fila y columna que deben quedar a la vista: las del foco, salvo en el eje que la
    /// selección abarca por completo (ahí no hay nada concreto que mostrar).
    pub(crate) fn visible_target(&self) -> (Option<i32>, Option<i32>) {
        (
            (!self.is_entire_cols()).then_some(self.focus.0),
            (!self.is_entire_rows()).then_some(self.focus.1),
        )
    }
}

/// Qué hay bajo un punto del control de la cuadrícula (`Core::hit_test`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hit {
    /// Una celda del área de datos.
    Cell {
        /// Fila (base 1).
        row: i32,
        /// Columna (base 1).
        col: i32,
    },
    /// La cabecera de una fila (la columna de números).
    RowHeader {
        /// Fila (base 1).
        row: i32,
    },
    /// La cabecera de una columna (la fila de letras).
    ColHeader {
        /// Columna (base 1).
        col: i32,
    },
    /// La esquina donde se cruzan las dos cabeceras (selecciona toda la hoja).
    Corner,
    /// Fuera del control, o más allá de la última fila o columna de la hoja.
    Outside,
}

/// Qué extiende `DragTo` según dónde empezó el clic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DragMode {
    /// Clic fuera de la cuadrícula o en la esquina: arrastrar no hace nada.
    None,
    /// Clic en una celda: el arrastre extiende un rango de celdas.
    #[default]
    Cells,
    /// Clic en una cabecera de fila: el arrastre extiende filas enteras.
    Rows,
    /// Clic en una cabecera de columna: el arrastre extiende columnas enteras.
    Cols,
}

/// Nuevo desplazamiento de un eje para que el elemento `idx` quepa en la ventana (`win`).
///
/// `pos(i)` es el borde inicial del elemento `i` y `at(p)` el elemento que contiene `p`. Si ya
/// cabe, devuelve `scroll` sin tocarlo. Si se sale por el principio, el desplazamiento pasa a
/// ser su borde inicial; si se sale por el final, al primer borde inicial que lo deja entero
/// dentro (o su propio borde inicial si es mayor que la ventana).
fn axis_scroll(
    scroll: f64,
    win: f64,
    idx: i32,
    pos: &dyn Fn(i32) -> f64,
    at: &dyn Fn(f64) -> i32,
) -> f64 {
    let (start, end) = (pos(idx), pos(idx + 1));
    if start < scroll - EPS {
        return start;
    }
    if end > scroll + win + EPS {
        let need = end - win;
        let mut first = at(need);
        if pos(first) < need - EPS {
            first += 1;
        }
        return pos(first.min(idx));
    }
    scroll
}

impl Core {
    /// Selección actual (de la hoja seleccionada en el motor).
    pub fn selection(&self) -> Selection {
        let vista = self.um.get_selected_view();
        let [r0, c0, r1, c1] = vista.range;
        // `SelectedView` no trae el foco: es la esquina del rango opuesta al ancla en cada eje.
        let focus = (
            if vista.row == r0 { r1 } else { r0 },
            if vista.column == c0 { c1 } else { c0 },
        );
        Selection {
            sheet: vista.sheet,
            active: (vista.row, vista.column),
            focus,
            range: (r0, c0, r1, c1),
        }
    }

    /// Celda superior izquierda que el motor considera visible `(fila, columna)`. Coincide con
    /// la que da el desplazamiento en píxeles (`row_at(scroll_y)`, `col_at(scroll_x)`).
    pub fn top_left_cell(&self) -> (i32, i32) {
        let vista = self.um.get_selected_view();
        (vista.top_row, vista.left_column)
    }

    /// ¿Qué hay bajo el punto `(x, y)` del control? Coordenadas en píxeles lógicos con origen
    /// en la esquina superior izquierda del control (las cabeceras cuentan). Un punto no
    /// finito, fuera del control o más allá de la última fila o columna de la hoja da
    /// [`Hit::Outside`].
    pub fn hit_test(&mut self, x: f32, y: f32) -> Hit {
        let v = *self.view();
        if !(x.is_finite() && y.is_finite())
            || x < 0.0
            || y < 0.0
            || x >= v.viewport_w
            || y >= v.viewport_h
        {
            return Hit::Outside;
        }
        let en_filas = x < ROW_HEADER_W;
        let en_cols = y < COL_HEADER_H;
        if en_filas && en_cols {
            return Hit::Corner;
        }
        let Ok(g) = self.geometry(v.sheet) else {
            return Hit::Outside;
        };
        let (sx, sy) = v.to_sheet(x, y);
        let col = if en_filas {
            None
        } else if sx >= g.total_w() {
            return Hit::Outside;
        } else {
            Some(g.col_at(sx))
        };
        let row = if en_cols {
            None
        } else if sy >= g.total_h() {
            return Hit::Outside;
        } else {
            Some(g.row_at(sy))
        };
        match (row, col) {
            (Some(row), Some(col)) => Hit::Cell { row, col },
            (Some(row), None) => Hit::RowHeader { row },
            (None, Some(col)) => Hit::ColHeader { col },
            (None, None) => Hit::Corner,
        }
    }

    /// Clic en `(x, y)` (mismas coordenadas que [`Core::hit_test`]); devuelve lo que había
    /// bajo el puntero. Con `extend` (Mayús+clic) el rango va del ancla al punto.
    ///
    /// * Celda: la selecciona (o extiende el rango hasta ella).
    /// * Cabecera de fila o de columna: selecciona la fila o columna entera. La celda activa es
    ///   la de esa fila (columna) en la primera columna (fila) visible.
    /// * Esquina: selecciona toda la hoja sin mover la celda activa.
    /// * Fuera: no hace nada.
    ///
    /// Un clic solo desplaza la vista lo justo para que se vea entera la celda pulsada.
    pub fn click(&mut self, x: f32, y: f32, extend: bool) -> Result<Hit, CoreError> {
        self.sync_state();
        let hit = self.hit_test(x, y);
        match hit {
            Hit::Cell { row, col } => {
                self.drag = DragMode::Cells;
                if extend {
                    self.um
                        .on_area_selecting(row, col)
                        .map_err(CoreError::Motor)?;
                } else {
                    self.um
                        .set_selected_cell(row, col)
                        .map_err(CoreError::Motor)?;
                }
            }
            Hit::RowHeader { row } => {
                self.drag = DragMode::Rows;
                let (_, izquierda) = self.top_left_cell();
                let ancla = if extend {
                    self.selection().active
                } else {
                    (row, izquierda)
                };
                self.select_rows(ancla, row, !extend)?;
            }
            Hit::ColHeader { col } => {
                self.drag = DragMode::Cols;
                let (arriba, _) = self.top_left_cell();
                let ancla = if extend {
                    self.selection().active
                } else {
                    (arriba, col)
                };
                self.select_cols(ancla, col, !extend)?;
            }
            Hit::Corner => {
                self.drag = DragMode::None;
                self.um
                    .set_selected_range(1, 1, MAX_ROWS, MAX_COLS)
                    .map_err(CoreError::Motor)?;
            }
            Hit::Outside => {
                self.drag = DragMode::None;
                return Ok(hit);
            }
        }
        self.ensure_visible();
        Ok(hit)
    }

    /// Arrastre del puntero hasta `(x, y)` tras un [`Core::click`]: extiende la selección de
    /// celdas, de filas o de columnas hasta lo que haya bajo el puntero. Un punto fuera de la
    /// cuadrícula no es un error: se toma la celda que le correspondería (la columna o fila
    /// de más allá del borde) y la vista se desplaza para mostrarla, como el autodesplazamiento
    /// de Office. Un punto no finito se ignora.
    pub fn drag_to(&mut self, x: f32, y: f32) -> Result<(), CoreError> {
        self.sync_state();
        if self.drag == DragMode::None || !(x.is_finite() && y.is_finite()) {
            return Ok(());
        }
        let Ok(g) = self.geometry(self.view.sheet) else {
            return Ok(());
        };
        let (sx, sy) = self.view.to_sheet(x, y);
        let (fila, col) = (g.row_at(sy), g.col_at(sx));
        let ancla = self.selection().active;
        match self.drag {
            DragMode::Cells => self
                .um
                .on_area_selecting(fila, col)
                .map_err(CoreError::Motor)?,
            DragMode::Rows => self.select_rows(ancla, fila, false)?,
            DragMode::Cols => self.select_cols(ancla, col, false)?,
            DragMode::None => {}
        }
        self.ensure_visible();
        Ok(())
    }

    /// Una flecha (`on_arrow_*`) o, con `extend` (Mayús+flecha), un paso del foco
    /// (`on_expand_selected_range`). Sin `extend` la selección queda en una celda aunque la
    /// flecha no pueda moverse (borde de la hoja).
    pub fn move_active(&mut self, dir: Dir, extend: bool) -> Result<(), CoreError> {
        self.prepare_key();
        let resultado = if extend {
            self.um.on_expand_selected_range(dir.key())
        } else {
            match dir {
                Dir::Up => self.um.on_arrow_up(),
                Dir::Down => self.um.on_arrow_down(),
                Dir::Left => self.um.on_arrow_left(),
                Dir::Right => self.um.on_arrow_right(),
            }
        };
        resultado.map_err(CoreError::Motor)?;
        if !extend {
            self.collapse_to_active()?;
        }
        self.ensure_visible();
        Ok(())
    }

    /// Ctrl/Cmd+flecha: salta al borde del bloque de datos (`on_navigate_to_edge_in_direction`);
    /// en una hoja vacía, al borde de la hoja. Con `extend` (Ctrl+Mayús+flecha) mueve el foco
    /// con la misma regla (`Worksheet::navigate_to_edge_in_direction` aplicada al foco; la API de
    /// `UserModel` no lo ofrece, §5.6).
    pub fn go_to_edge(&mut self, dir: Dir, extend: bool) -> Result<(), CoreError> {
        self.prepare_key();
        if extend {
            let sel = self.selection();
            let destino = self
                .um
                .get_model()
                .workbook
                .worksheet(sel.sheet)
                .and_then(|h| {
                    h.navigate_to_edge_in_direction(sel.focus.0, sel.focus.1, dir.engine())
                })
                .map_err(CoreError::Motor)?;
            self.um
                .set_selected_range(sel.active.0, sel.active.1, destino.0, destino.1)
                .map_err(CoreError::Motor)?;
        } else {
            self.um
                .on_navigate_to_edge_in_direction(dir.engine())
                .map_err(CoreError::Motor)?;
            self.collapse_to_active()?;
        }
        self.ensure_visible();
        Ok(())
    }

    /// RePág (`down = false`) o AvPág (`down = true`): mueve la celda activa y la vista una
    /// pantalla (`on_page_up/down`; dependen de `set_window_height`, que `set_viewport` fija).
    /// La celda activa conserva su posición relativa en la pantalla. Con `extend`
    /// (Mayús+RePág/AvPág) se mueve el foco la misma cantidad de filas. En el último tramo de
    /// la hoja, donde `UserModel` no puede dar una página entera, la selección va a la última
    /// (primera) fila.
    pub fn page(&mut self, down: bool, extend: bool) -> Result<(), CoreError> {
        self.prepare_key();
        let antes = self.selection();
        let superior_antes = self.top_left_cell().0;
        let resultado = if down {
            self.um.on_page_down()
        } else {
            self.um.on_page_up()
        };
        // De `UserModel` solo se aprovecha la fila superior de la página. Su selección no se
        // usa: no valida la fila destino (junto al final de la hoja, o con la celda activa
        // fuera de la pantalla, la deja fuera de la hoja) y no sabe de rangos extendidos.
        let (fila_activa, fila_foco) = match resultado {
            Ok(()) => {
                let superior = self.top_left_cell().0;
                if let Ok(g) = self.geometry(self.view.sheet) {
                    self.set_scroll_nav(None, Some(g.row_y(superior)));
                }
                let delta = superior - superior_antes;
                (
                    (antes.active.0 + delta).clamp(1, MAX_ROWS),
                    (antes.focus.0 + delta).clamp(1, MAX_ROWS),
                )
            }
            // Junto al final de la hoja `UserModel` no encuentra una página entera: se llega al
            // borde.
            Err(_) => {
                let borde = if down { MAX_ROWS } else { 1 };
                (borde, borde)
            }
        };
        self.um
            .set_selected_cell(antes.active.0, antes.active.1)
            .map_err(CoreError::Motor)?;
        if extend {
            self.um
                .set_selected_range(antes.active.0, antes.active.1, fila_foco, antes.focus.1)
                .map_err(CoreError::Motor)?;
        } else {
            self.um
                .set_selected_cell(fila_activa, antes.active.1)
                .map_err(CoreError::Motor)?;
        }
        self.ensure_visible();
        Ok(())
    }

    /// Inicio (`ctrl = false`): primera columna visible de la fila activa. Ctrl+Inicio: A1
    /// (la primera fila y columna no ocultas) y el desplazamiento vuelve al origen.
    pub fn home(&mut self, ctrl: bool) -> Result<(), CoreError> {
        self.prepare_key();
        let g = self.geometry(self.view.sheet)?;
        let primera =
            |tam: &dyn Fn(i32) -> f64, n: i32| (1..=n).find(|&i| tam(i) > 0.0).unwrap_or(1);
        let col = primera(&|c| g.col_w(c), MAX_COLS);
        let fila = if ctrl {
            primera(&|r| g.row_h(r), MAX_ROWS)
        } else {
            self.selection().active.0
        };
        self.um
            .set_selected_cell(fila, col)
            .map_err(CoreError::Motor)?;
        self.ensure_visible();
        Ok(())
    }

    /// Desplaza la vista lo mínimo para que se vea la selección (su foco, ver el módulo),
    /// por filas y columnas enteras. Devuelve el desplazamiento resultante. Lo usa cada
    /// comando de navegación; la edición (E08) lo llama al confirmar con avance.
    pub fn ensure_visible(&mut self) -> (f64, f64) {
        self.sync_state();
        let (fila, col) = self.selection().visible_target();
        self.ensure_axes_visible(fila, col)
    }

    /// Como [`Core::ensure_visible`] para una celda concreta `(fila, columna)` (base 1; los
    /// valores fuera de la hoja se acotan).
    pub fn ensure_cell_visible(&mut self, row: i32, col: i32) -> (f64, f64) {
        self.sync_state();
        self.ensure_axes_visible(Some(row.clamp(1, MAX_ROWS)), Some(col.clamp(1, MAX_COLS)))
    }

    fn ensure_axes_visible(&mut self, fila: Option<i32>, col: Option<i32>) -> (f64, f64) {
        let v = *self.view();
        if let Ok(g) = self.geometry(v.sheet) {
            let x = col.map(|c| {
                axis_scroll(v.scroll_x, v.cells_w(), c, &|i| g.col_x(i), &|p| {
                    g.col_at(p)
                })
            });
            let y = fila.map(|r| {
                axis_scroll(v.scroll_y, v.cells_h(), r, &|i| g.row_y(i), &|p| {
                    g.row_at(p)
                })
            });
            self.set_scroll_nav(x, y);
        }
        (self.view.scroll_x, self.view.scroll_y)
    }

    /// Antes de un comando de teclado: hoja y desplazamiento coherentes con el motor, y la
    /// vista de vuelta en la celda activa si el usuario se había desplazado lejos de ella (como
    /// en Office, la tecla actúa desde donde está el cursor).
    fn prepare_key(&mut self) {
        self.sync_state();
        self.ensure_visible();
    }

    /// Hoja y desplazamiento de `UserModel` coherentes con los del núcleo.
    fn sync_state(&mut self) {
        self.sync_view_sheet();
        self.sync_um_scroll();
    }

    /// Deja la selección en la celda activa si el movimiento no la redujo (un borde de la hoja
    /// que no deja moverse no debe conservar un rango).
    fn collapse_to_active(&mut self) -> Result<(), CoreError> {
        let sel = self.selection();
        if !sel.is_single_cell() {
            self.um
                .set_selected_cell(sel.active.0, sel.active.1)
                .map_err(CoreError::Motor)?;
        }
        Ok(())
    }

    /// Selecciona las filas `ancla.0..=hasta` enteras. Con `nueva`, la celda activa pasa a
    /// `ancla`; si no, se conserva la actual (que debe quedar dentro del rango).
    fn select_rows(&mut self, ancla: (i32, i32), hasta: i32, nueva: bool) -> Result<(), CoreError> {
        if nueva {
            self.um
                .set_selected_cell(ancla.0, ancla.1)
                .map_err(CoreError::Motor)?;
        }
        self.um
            .set_selected_range(ancla.0, 1, hasta, MAX_COLS)
            .map_err(CoreError::Motor)
    }

    /// Como [`Core::select_rows`] para columnas.
    fn select_cols(&mut self, ancla: (i32, i32), hasta: i32, nueva: bool) -> Result<(), CoreError> {
        if nueva {
            self.um
                .set_selected_cell(ancla.0, ancla.1)
                .map_err(CoreError::Motor)?;
        }
        self.um
            .set_selected_range(1, ancla.1, MAX_ROWS, hasta)
            .map_err(CoreError::Motor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::OpenOptions;

    /// Libro con tres hojas y la hoja seleccionada cambiada directamente en el motor, como lo
    /// hará `SelectSheet` (E08): la vista debe seguirla antes de actuar.
    #[test]
    fn la_vista_sigue_a_la_hoja_seleccionada_en_el_motor() {
        let mut core = Core::new_empty(OpenOptions::default()).unwrap();
        core.um.new_sheet().unwrap();
        core.um.new_sheet().unwrap();
        // `new_sheet` deja seleccionada la hoja nueva; la vista la sigue en cuanto se actúa.
        core.set_viewport(1046.0, 525.0, 1.0);
        assert_eq!(core.view().sheet, 2);
        core.um.set_selected_sheet(0).unwrap();
        core.sync_view_sheet();
        assert_eq!(core.view().sheet, 0);

        // La hoja 2 recuerda su selección y su celda superior izquierda.
        core.um.set_selected_sheet(2).unwrap();
        core.um.set_selected_cell(40, 6).unwrap();
        core.um.set_top_left_visible_cell(30, 4).unwrap();
        core.um.set_selected_sheet(0).unwrap();
        core.sync_view_sheet();
        assert_eq!(core.view().sheet, 0);

        let version = core.data_version();
        core.um.set_selected_sheet(2).unwrap();
        // Una operación de navegación sincroniza primero y actúa en la hoja nueva.
        core.move_active(Dir::Down, false).unwrap();
        assert_eq!(core.view().sheet, 2);
        assert_eq!(core.selection().sheet, 2);
        assert_eq!(core.selection().active, (41, 6));
        assert!(
            core.data_version() > version,
            "cambiar de hoja invalida el bloque"
        );
        // El desplazamiento es el de esa hoja (30, 4): la celda activa se ve y el motor
        // coincide.
        let g = core.geometry(2).unwrap();
        let v = *core.view();
        assert_eq!(
            core.top_left_cell(),
            (g.row_at(v.scroll_y), g.col_at(v.scroll_x))
        );
        assert_eq!(v.scroll_x, g.col_x(4));
        assert_eq!(v.scroll_y, g.row_y(30));

        // Y volver a la primera hoja recupera su propio desplazamiento.
        core.um.set_selected_sheet(0).unwrap();
        core.sync_view_sheet();
        assert_eq!(
            (
                core.view().sheet,
                core.view().scroll_x,
                core.view().scroll_y
            ),
            (0, 0.0, 0.0)
        );
        assert_eq!(core.selection().sheet, 0);
    }
}
