
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

pub enum Cursor {
    Cell(usize, usize),
    Column(usize),
}

impl Cursor {
    pub fn move_cursor(&mut self, dir: CursorDir, n: usize, max_x: usize, max_y: usize) {
        match dir {
            lr @ CursorDir::L | lr @ CursorDir::R => {
                let x = match self {
                    Self::Cell(ref mut x, _) => x,
                    Self::Column(ref mut x) => x,
                };

                match lr {
                    CursorDir::L => { *x = x.saturating_sub(n); },
                    CursorDir::R => { *x = max_x.min(*x + n); },
                    _ => unreachable!(),
                };
            },
            ud @ CursorDir::U | ud @ CursorDir::D => {
                let y = match self {
                    Self::Cell(_, ref mut y) => y,
                    Self::Column(..) => { return; },
                };

                match ud {
                    CursorDir::U => { *y = y.saturating_sub(n); },
                    CursorDir::D => { *y = max_y.min(*y + n); },
                    _ => unreachable!(),
                };
            },
        }
    }
}

pub struct Model {
    pub data: Data,
    pub cursor: Cursor,

    cached_content_widths: Vec<usize>,
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
        self.cursor.move_cursor(
            cursor_dir,
            n,
            self.data.columns.len(),
            self.data.records.len(),
        );
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
