
use std::collections::HashMap;
use std::slice::Iter as SliceIter;

use indexmap::IndexMap;

use crate::util::Util;

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

pub struct Model {
    pub columns: Columns,
    pub records: Records,

    cached_content_widths: Vec<usize>,
    needs_recache: bool,
}

impl Model {
    pub fn new() -> Self {
        Self::with_data(Columns::new(), Records::new())
    }

    pub fn with_data(columns: Columns, records: Records) -> Self {
        let cached_content_widths = Vec::with_capacity(columns.len());

        let mut new = Self {
            columns,
            records,
            cached_content_widths,
            needs_recache: true,
        };

        new.recache();

        new
    }

    pub fn recache(&mut self) {
        // Proceed and clear the flag if it was set.
        // Otherwise, bail out.
        if self.needs_recache { self.needs_recache = false; }
        else { return; }

        self.cached_content_widths.clear();
        self.cached_content_widths.reserve(self.columns.len());

        for (column_key, column_def) in self.columns.iter() {
            let column_sizing = column_def.sizing;

            let content_width = match column_sizing {
                Sizing::Fixed(width) => width,
                Sizing::Auto => Util::max_column_content_width(column_key, &self.columns, &self.records),
            };

            self.cached_content_widths.push(content_width);
        }

        assert_eq!(self.cached_content_widths.len(), self.columns.len());
    }

    pub fn mutate_columns<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Columns) -> R,
    {
        let result = func(&mut self.columns);
        self.needs_recache = true;
        result
    }

    pub fn mutate_records<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Records) -> R,
    {
        let result = func(&mut self.records);
        self.needs_recache = true;
        result
    }

    pub fn total_display_width(&self, column_sep_width: usize) -> usize {
        let total_sep_width = self.cached_content_widths.len().saturating_sub(1) * column_sep_width;
        self.cached_content_widths.iter().sum::<usize>() + total_sep_width
    }

    pub fn iter_column<'a>(&'a self, column_key: &'a str) -> IterColumn<'a> {
        IterColumn(column_key, self.records.iter())
    }

    pub fn iter_cache<'a>(&'a self) -> IterCache<'a> {
        IterCache(self.cached_content_widths.iter())
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
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
