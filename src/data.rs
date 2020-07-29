
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::slice::Iter as SliceIter;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(from = "SizingRepr")]
pub enum Sizing {
    Auto,
    Fixed(usize),
    Lower(usize),
    Upper(usize),
    Bound(usize, usize),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(untagged)]
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoKind {
    FileName,
    FilePath,
}

#[derive(Debug, Clone, Hash, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnKey {
    Meta(String),
    Info(InfoKind),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    /// The raw string metadata key for this column.
    #[serde(flatten)]
    pub key: ColumnKey,

    /// A friendly human-readable name for the column, used for display.
    pub title: String,

    /// Sizing for this column.
    /// This affects the width of the content of the column, it does not include
    /// any column padding/separators in the width.
    pub sizing: Sizing,
}

pub struct Record {
    pub metadata: HashMap<String, String>,
    pub file_path: PathBuf,
}

impl Record {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            file_path: PathBuf::new(),
        }
    }

    pub fn get_meta(&self, meta_key: &str) -> Option<&str> {
        self.metadata.get(meta_key).map(AsRef::as_ref)
    }

    pub fn get_info(&self, info_kind: &InfoKind) -> Option<&str> {
        match info_kind {
            InfoKind::FileName => self.file_path.file_name().and_then(|f| f.to_str()),
            InfoKind::FilePath => self.file_path.to_str(),
        }
    }

    pub fn get(&self, column_key: &ColumnKey) -> Option<&str> {
        match column_key {
            ColumnKey::Meta(ref meta_key) => self.get_meta(meta_key),
            ColumnKey::Info(ref info_kind) => self.get_info(info_kind),
        }
    }
}

pub type Columns = Vec<Column>;
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
        if let Some(column) = self.columns.get(column_index) {
            let column_key = &column.key;
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
        Some(record.metadata.get(self.0))
    }
}

pub struct IterCache<'a>(SliceIter<'a, usize>);

impl<'a> Iterator for IterCache<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().copied()
    }
}
