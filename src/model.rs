
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::slice::Iter as SliceIter;

use cursive::XY;
use indexmap::IndexMap;

use crate::util::Util;

#[derive(Debug, Clone, Copy)]
pub enum CursorDir {
    U, D, L, R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    All,
    Cell(usize, usize),
    Column(usize),
}

#[derive(Clone, Copy)]
pub enum Sizing {
    Auto,
    Fixed(usize),
}

#[derive(Clone)]
pub struct ColumnDef {
    /// A friendly human-readable name for the column, used for display.
    pub title: String,

    /// Sizing for this column.
    /// This affects the width of the content of the column, it does not include
    /// any column padding/separators in the width.
    pub sizing: Sizing,
}

pub type Record = HashMap<String, String>;

pub type Columns = IndexMap<String, ColumnDef>;
pub type Records = Vec<Record>;

pub struct Data {
    pub columns: Columns,
    pub records: Records,
}

impl Data {
    pub fn new() -> Self {
        Self::with_data(Columns::new(), Records::new())
    }

    pub fn with_data(columns: Columns, records: Records) -> Self {
        Self {
            columns,
            records,
        }
    }

    pub fn iter_column<'a>(&'a self, column_key: &'a str) -> IterColumn<'a> {
        IterColumn(column_key, self.records.iter())
    }

    pub fn sort_by_column(&mut self, column_key: &str) {
        self.records.sort_by(|ra, rb| {
            match (ra.get(column_key), rb.get(column_key)) {
                (None, None) => Ordering::Equal,
                (None, Some(..)) => Ordering::Less,
                (Some(..), None) => Ordering::Greater,
                (Some(a), Some(b)) => a.cmp(b),
            }
        });
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Model {
    pub data: Data,

    cursor_pos: (usize, usize),
    selections: HashSet<Selection>,

    cached_content_widths: Vec<usize>,
    dirty: bool,
}

impl Model {
    pub fn with_data(data: Data) -> Self {
        let cached_content_widths = Vec::with_capacity(data.columns.len());

        let mut new = Self {
            data,

            cursor_pos: (0, 0),
            selections: HashSet::new(),

            cached_content_widths,
            dirty: true,
        };

        new.recache();

        new
    }

    pub fn column_index(&self) -> Option<usize> {
        let (x, _) = self.cursor_pos;

        if x < self.data.columns.len() { Some(x) }
        else { None }
    }

    pub fn record_index(&self) -> Option<usize> {
        let (_, y) = self.cursor_pos;

        if y < self.data.records.len() { Some(y) }
        else { None }
    }

    pub fn cursor_position(&self) -> Option<(usize, usize)> {
        let x = self.column_index()?;
        let y = self.record_index()?;

        Some((x, y))
    }

    fn move_cursor(&mut self, cursor_dir: CursorDir, n: usize) {
        let (cx, cy) = self.cursor_pos;

        let max_x = self.data.columns.len().saturating_sub(1);
        let max_y = self.data.records.len().saturating_sub(1);

        self.cursor_pos = match cursor_dir {
            CursorDir::U => (cx, cy.saturating_sub(n)),
            CursorDir::D => (cx, max_y.min(cy + n)),
            CursorDir::L => (cx.saturating_sub(n), cy),
            CursorDir::R => (max_x.min(cx + n), cy),
        };
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

    pub fn select_all(&mut self) {
        self.selections.clear();
        self.selections.insert(Selection::All);
    }

    pub fn select_cell(&mut self, x: usize, y: usize, append: bool) {
        // If not appending, clear out the existing selection(s).
        if !append { self.selections.clear(); }
        self.selections.insert(Selection::Cell(x, y));
    }

    pub fn select_column(&mut self, col_idx: usize, append: bool) {
        // If not appending, clear out the existing selection(s).
        if !append { self.selections.clear(); }
        self.selections.insert(Selection::Column(col_idx));
    }

    pub fn select_current_cell(&mut self, append: bool) {
        // If not appending, clear out the existing selection(s).
        if !append { self.selections.clear(); }

        if let Some((x, y)) = self.cursor_position() {
            self.selections.insert(Selection::Cell(x, y));
        }
    }

    pub fn select_current_column(&mut self, append: bool) {
        // If not appending, clear out the existing selection(s).
        if !append { self.selections.clear(); }

        if let Some(col_idx) = self.column_index() {
            self.selections.insert(Selection::Column(col_idx));
        }
    }

    pub fn deselect_all(&mut self) {
        self.selections.clear();
    }

    /// Returns `true` if a given (x, y) position is marked as selected.
    pub fn is_xy_selected(&self, x: usize, y: usize) -> bool {
        // Check if this coordinate falls under any of the selections.
        self.selections.contains(&Selection::All)
        || self.selections.contains(&Selection::Cell(x, y))
        || self.selections.contains(&Selection::Column(x))
    }

    /// Returns `true` if a selection is in progress.
    pub fn is_selecting(&self) -> bool {
        !self.selections.is_empty()
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

    pub fn iter_cached_widths<'a>(&'a self) -> IterCache<'a> {
        IterCache(self.cached_content_widths.iter())
    }

    pub fn sort_by_column(&mut self, column_key: &str) {
        // No recaching should be needed with sorting.
        self.data.sort_by_column(column_key);
    }
}

pub struct IterColumn<'a>(&'a str, SliceIter<'a, Record>);

impl<'a> Iterator for IterColumn<'a> {
    type Item = Option<&'a String>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.1.next()?;
        Some(record.get(self.0))
    }
}

pub struct IterCache<'a>(SliceIter<'a, usize>);

impl<'a> Iterator for IterCache<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().copied()
    }
}
