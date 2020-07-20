
#[derive(Debug, Clone, Copy)]
pub enum CursorDir {
    U, D, L, R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Cell(usize, usize),
    Column(usize),
}

impl Cursor {
    pub fn to_xy(&self) -> (usize, Option<usize>) {
        match self {
            Self::Cell(x, y) => (*x, Some(*y)),
            Self::Column(x) => (*x, None),
        }
    }

    pub fn column_index(&self) -> Option<usize> {
        match self {
            Self::Cell(..) => None,
            Self::Column(x) => Some(*x),
        }
    }

    pub fn clamp(&mut self, bound_x: usize, bound_y: usize) {
        let max_idx_x = bound_x.saturating_sub(1);
        let max_idx_y = bound_y.saturating_sub(1);

        match self {
            Self::Cell(ref mut x, ref mut y) => {
                *x = max_idx_x.min(*x);
                *y = max_idx_y.min(*y);
            },
            Self::Column(ref mut x) => {
                *x = max_idx_x.min(*x);
            },
        };
    }

    pub fn shift(&mut self, dir: CursorDir, n: usize, bound_x: usize, bound_y: usize) {
        // Skip work if a delta of 0 is given.
        if n > 0 {
            match dir {
                CursorDir::U => {
                    match self {
                        Self::Cell(x, ref mut y) => {
                            match y.checked_sub(n) {
                                Some(yp) => { *y = yp; }
                                None => { *self = Self::Column(*x); },
                            }
                        },
                        Self::Column(..) => {}
                    }
                },
                CursorDir::D => {
                    match self {
                        Self::Cell(_, ref mut y) => { *y = y.saturating_add(n); },
                        Self::Column(x) => { *self = Self::Cell(*x, n.saturating_sub(1)); }
                    }
                },
                CursorDir::L => {
                    match self {
                        Self::Cell(ref mut x, _) => { *x = x.saturating_sub(n); },
                        Self::Column(ref mut x) => { *x = x.saturating_sub(n); }
                    }
                },
                CursorDir::R => {
                    match self {
                        Self::Cell(ref mut x, _) => { *x = x.saturating_add(n); },
                        Self::Column(ref mut x) => { *x = x.saturating_add(n); }
                    }
                },
            };
        }

        // Still want to clamp, even if a delta of 0 was given.
        self.clamp(bound_x, bound_y);
    }

    pub fn is_in_column_mode(&self) -> bool {
        matches!(self, Self::Column(..))
    }

    pub fn is_in_cell_mode(&self) -> bool {
        matches!(self, Self::Cell(..))
    }
}
