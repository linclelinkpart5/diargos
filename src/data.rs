
use std::cmp::Ordering;
use std::collections::HashMap;
use std::slice::Iter as SliceIter;

use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(from = "SizingRepr")]
pub enum Sizing {
    Auto,
    Fixed(usize),
    Lower(usize),
    Upper(usize),
    Bound(usize, usize),
}

#[derive(Clone, Copy, Deserialize)]
pub enum SizingRepr {
    Auto,
    Fixed(usize),
    Lower(usize, ()),
    Upper((), usize),
    Bound(usize, usize),
}

impl From<SizingRepr> for Sizing {
    fn from(repr: SizingRepr) -> Self {
        match repr {
            SizingRepr::Auto => Sizing::Auto,
            SizingRepr::Fixed(width) => Sizing::Fixed(width),
            SizingRepr::Lower(min_width, ()) => Sizing::Lower(min_width),
            SizingRepr::Upper((), max_width) => Sizing::Upper(max_width),
            SizingRepr::Bound(min_width, max_width) => {
                // Ensure proper order.
                if min_width > max_width {
                    // TODO: Add log message here.
                    Sizing::Bound(min_width, min_width)
                } else {
                    Sizing::Bound(min_width, max_width)
                }
            },
        }
    }
}

#[derive(Clone, Deserialize)]
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

    pub fn sort_by_column_index(&mut self, column_index: usize, is_descending: bool) {
        if let Some((column_key, _)) = self.columns.get_index(column_index) {
            self.records.sort_by(move |ra, rb| {
                let o = match (ra.get(column_key), rb.get(column_key)) {
                    (None, None) => Ordering::Equal,
                    (None, Some(..)) => Ordering::Less,
                    (Some(..), None) => Ordering::Greater,
                    (Some(a), Some(b)) => a.cmp(b),
                };

                if is_descending { o.reverse() } else { o }
            });
        }
    }
}

impl Default for Data {
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
