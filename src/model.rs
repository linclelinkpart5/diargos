
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::slice::Iter as SliceIter;

use cursive::XY;
use indexmap::IndexMap;
use unicode_width::UnicodeWidthStr;

use crate::consts::*;
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
    data: Data,

    cursor_pos: (usize, usize),
    selections: HashSet<Selection>,

    cached_content_widths: Vec<usize>,
    dirty: bool,
    header: String,
    header_bar: String,
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
            header: String::new(),
            header_bar: String::new(),
        };

        new.recache();

        new
    }

    pub fn get_data(&self) -> &Data {
        &self.data
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

        // Create the cached header and header bar.
        let mut is_first_col = true;
        self.header.clear();
        self.header_bar.clear();

        let content_widths = self.cached_content_widths.iter().cloned();

        for (column_def, content_width) in self.data.columns.values().zip(content_widths) {
            if is_first_col { is_first_col = false; }
            else {
                self.header.push_str(COLUMN_SEP);
                self.header_bar.push_str(COLUMN_HEADER_SEP);
            }

            Util::extend_with_fitted_str(&mut self.header, &column_def.title, content_width);

            // Extend the header bar.
            for _ in 0..content_width {
                self.header_bar.push_str(COLUMN_HEADER_BAR);
            }
        }
    }

    pub fn headers(&self) -> (&str, &str) {
        (&self.header, &self.header_bar)
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

pub struct IterWidthsOffsets<'a> {
    width_iter: SliceIter<'a, usize>,
    curr_offset: usize,
    column_sep_width: usize,
    is_first: bool,
}

impl<'a> IterWidthsOffsets<'a> {
    pub fn new(widths: &'a [usize], column_sep_width: usize) -> Self {
        Self {
            width_iter: widths.iter(),
            curr_offset: 0,
            column_sep_width,
            is_first: true,
        }
    }
}

impl<'a> Iterator for IterWidthsOffsets<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let width = self.width_iter.next().copied()?;

        if self.is_first { self.is_first = false; }
        else { self.curr_offset += self.column_sep_width; }

        let ret = (width, self.curr_offset);

        self.curr_offset += width;

        Some(ret)
    }
}

#[derive(Debug, Clone, Copy)]
enum PrintAtomsState {
    Value,
    Ellipsis,
    Delimiter,
}

pub struct PrintAtoms<'a, S, W>
where
    S: Iterator<Item = &'a str>,
    W: Iterator<Item = usize>,
{
    atoms: std::iter::Peekable<std::iter::Zip<S, W>>,
    curr_offset: usize,
    is_first: bool,
    state: PrintAtomsState,
}

impl<'a, S, W> PrintAtoms<'a, S, W>
where
    S: Iterator<Item = &'a str>,
    W: Iterator<Item = usize>,
{
    pub fn new(strings: S, widths: W) -> Self {
        Self {
            atoms: strings.zip(widths).peekable(),
            curr_offset: 0,
            is_first: true,
            state: PrintAtomsState::Value,
        }
    }
}

impl<'a, S, W> Iterator for PrintAtoms<'a, S, W>
where
    S: Iterator<Item = &'a str>,
    W: Iterator<Item = usize>,
{
    type Item = (&'a str, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.atoms.peek()?;

        match self.state {
            PrintAtomsState::Value => {
                let (original_str, target_width) = self.atoms.next()?;

                let original_width = original_str.width_cjk();

                let (display_str, needs_ellipsis) =
                    if original_width > target_width {
                        if target_width >= ELLIPSIS_STR_WIDTH {
                            let elided_width = target_width.saturating_sub(ELLIPSIS_STR_WIDTH);
                        }
                        // Degenerate case: the ellipsis is too wide to fit in
                        // the target width. Do not bother with ellipsis in the case.
                        else {

                        }


                        let elided_width = target_width.saturating_sub(ELLIPSIS_STR_WIDTH);

                        let (trimmed_str, _, was_trimmed) =
                            Util::trim_display_str(original_str, elided_width)
                        ;

                        let needs_ellipsis = if target_width < ELLIPSIS_STR_WIDTH {
                            false
                        } else {
                            was_trimmed
                        };

                        (trimmed_str, needs_ellipsis)
                    } else {
                        (original_str, false)
                    }
                ;

                let ret = Some((display_str, self.curr_offset));

                if needs_ellipsis {
                    self.state = PrintAtomsState::Ellipsis;
                    self.curr_offset += target_width.saturating_sub(ELLIPSIS_STR_WIDTH);
                } else {
                    self.state = PrintAtomsState::Delimiter;
                    self.curr_offset += target_width;
                }

                ret
            },
            PrintAtomsState::Ellipsis => {
                let ret = Some((ELLIPSIS_STR, self.curr_offset));

                self.state = PrintAtomsState::Delimiter;
                self.curr_offset += ELLIPSIS_STR_WIDTH;

                ret
            },
            PrintAtomsState::Delimiter => {
                let ret = Some((COLUMN_SEP, self.curr_offset));

                self.state = PrintAtomsState::Value;
                self.curr_offset += COLUMN_SEP_WIDTH;

                ret
            },
        }
    }
}

impl<'a, S, W> std::iter::FusedIterator for PrintAtoms<'a, S, W>
where
    S: Iterator<Item = &'a str>,
    W: Iterator<Item = usize>,
{}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn print_atoms() {
        let strings = &["wow", "tubular", "日本人の氏名", "neat"];
        let widths = &[5, 5, 5, 5];

        let produced =
            PrintAtoms::new(
                strings.iter().copied(),
                widths.iter().copied(),
            )
            .collect::<Vec<_>>()
        ;
        let expected = vec![
            ("wow", 0),
            (" │ ", 5),
            ("tubu", 8),
            ("⋯", 12),
            (" │ ", 13),
            ("日本", 16),
            ("⋯", 20),
            (" │ ", 21),
            ("neat", 24),
        ];

        assert_eq!(produced, expected);
    }
}
