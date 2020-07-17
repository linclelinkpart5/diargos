
use cursive::XY;

use crate::data::Columns;
use crate::data::Data;
use crate::data::Records;
use crate::data::Sizing;
use crate::util::Util;

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
    pub fn to_xy(&self) -> Option<(usize, usize)> {
        match self {
            Self::Cell(x, y) => Some((*x, *y)),
            Self::Column(..) => None,
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
}

pub struct Model {
    pub data: Data,
    pub cursor: Cursor,

    pub cached_content_widths: Vec<usize>,
    dirty: bool,
}

impl Model {
    pub fn with_data(data: Data) -> Self {
        let cached_content_widths = Vec::with_capacity(data.columns.len());

        let mut new = Self {
            data,
            cursor: Cursor::Cell(0, 0),

            cached_content_widths,
            dirty: true,
        };

        new.recache();

        new
    }

    fn move_cursor(&mut self, cursor_dir: CursorDir, n: usize) {
        self.cursor.shift(cursor_dir, n, self.data.columns.len(), self.data.records.len());
    }

    pub fn move_cursor_up(&mut self, n: usize) {
        self.move_cursor(CursorDir::U, n)
    }

    pub fn move_cursor_down(&mut self, n: usize) {
        self.move_cursor(CursorDir::D, n)
    }

    pub fn move_cursor_left(&mut self, n: usize) {
        self.move_cursor(CursorDir::L, n)
    }

    pub fn move_cursor_right(&mut self, n: usize) {
        self.move_cursor(CursorDir::R, n)
    }

    pub fn is_cursor_at_column(&self, x: usize) -> bool {
        if let Cursor::Column(cx) = self.cursor {
            cx == x
        } else {
            false
        }
    }

    pub fn is_cursor_at_cell(&self, x: usize, y: usize) -> bool {
        if let Cursor::Cell(cx, cy) = self.cursor {
            cx == x && cy == y
        } else {
            false
        }
    }

    pub fn recache(&mut self) {
        // Proceed and clear the flag if it was set.
        // Otherwise, bail out.
        if self.dirty { self.dirty = false; }
        else { return; }

        self.cached_content_widths.clear();
        self.cached_content_widths.reserve(self.data.columns.len());

        for (column_key, column_def) in self.data.columns.iter() {
            let column_sizing = column_def.sizing;

            let content_width = match column_sizing {
                Sizing::Fixed(width) => width,
                Sizing::Auto => Util::max_column_content_width(
                    column_key,
                    &self.data.columns,
                    &self.data.records,
                ),
            };

            self.cached_content_widths.push(content_width);
        }

        assert_eq!(self.cached_content_widths.len(), self.data.columns.len());
    }

    pub fn total_display_width(&self, column_sep_width: usize) -> usize {
        let total_sep_width = self.cached_content_widths.len().saturating_sub(1) * column_sep_width;
        self.cached_content_widths.iter().sum::<usize>() + total_sep_width
    }

    pub fn column_offset(&self, column_index: usize, column_sep_width: usize) -> Option<usize> {
        if column_index >= self.cached_content_widths.len() {
            None
        } else {
            let offset =
                self.cached_content_widths.iter().cloned().take(column_index).sum::<usize>()
                + column_sep_width * column_index
            ;
            Some(offset)
        }
    }

    pub fn required_size(&self, column_sep_width: usize) -> XY<usize> {
        XY::new(self.total_display_width(column_sep_width), self.data.records.len())
    }

    pub fn mutate_columns<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Columns) -> R,
    {
        let result = func(&mut self.data.columns);
        self.dirty = true;
        result
    }

    pub fn mutate_records<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Records) -> R,
    {
        let result = func(&mut self.data.records);
        self.dirty = true;
        result
    }

    pub fn sort_by_column(&mut self, column_key: &str) {
        // No recaching should be needed with sorting.
        self.data.sort_by_column(column_key);
    }

    pub fn iter_cached_widths<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.cached_content_widths.iter().copied()
    }
}
