
use cursive::XY;

use crate::cursor::Cursor;
use crate::cursor::CursorDir;
use crate::data::Columns;
use crate::data::Data;
use crate::data::Records;
use crate::data::Sizing;
use crate::util::Util;

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

        for column in self.data.columns.iter() {
            let column_sizing = column.sizing;

            let mccw = || {
                Util::max_column_content_width(
                    &column,
                    &self.data.records,
                )
            };

            let content_width = match column_sizing {
                Sizing::Auto => mccw(),
                Sizing::Fixed(width) => width,
                Sizing::Lower(min_width) => mccw().max(min_width),
                Sizing::Upper(max_width) => mccw().min(max_width),
                Sizing::Bound(min_width, max_width) => mccw().max(min_width).min(max_width),
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

    pub fn sort_by_column_index(&mut self, column_index: usize, is_descending: bool) {
        // No recaching should be needed with sorting.
        self.data.sort_by_column_index(column_index, is_descending);
        self.dirty = true;
    }

    pub fn iter_cached_widths<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.cached_content_widths.iter().copied()
    }
}
