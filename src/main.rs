use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use indexmap::IndexMap;
use indexmap::indexmap;
use maplit::hashmap;
use str_macro::str;

use cursive::Cursive;
use cursive::CursiveExt;
use cursive::Printer;
use cursive::align::HAlign;
use cursive::direction::Direction;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::theme::ColorStyle;
use cursive::traits::Nameable;
use cursive::traits::Resizable;
use cursive::traits::Scrollable;
use cursive::vec::Vec2;
use cursive::view::ScrollBase;
use cursive::view::View;
use cursive::views::Dialog;

const ELLIPSIS_STR: &str = "⋯";
const ELLIPSIS_STR_WIDTH: usize = 1;

const MISSING_STR: &str = "╳";

fn width(s: &str) -> usize {
    s.char_indices().count()
}

pub type Record = HashMap<String, String>;

pub struct ColumnDef {
    /// A friendly human-readable name for the column, used for display.
    pub title: String,

    /// Desired column width, actual column width my be longer than this to
    /// accomodate the header display.
    pub desired_width: usize,

    // /// Horizontal alignment of the header for this column.
    // pub header_align: HAlign,

    // /// Horizontal alignment of the data for this column.
    // pub data_align: HAlign,

    // /// Flags if this column has been selected.
    // pub selected: bool,
}

/// Callback for when a column is sorted. Takes the column and ordering as input.
type OnSortCallback = Rc<dyn Fn(&mut Cursive, &str, Ordering)>;

/// Callback taking as argument the row and the index of an element.
type IndexCallback = Rc<dyn Fn(&mut Cursive, usize, usize)>;

pub struct TagRecordView {
    records: Vec<Record>,

    scroll_base: ScrollBase,
}

impl TagRecordView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_records(records: Vec<Record>) -> Self {
        Self {
            records,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn records(&self) -> &[Record] {
        self.records.as_slice()
    }

    pub fn records_mut(&mut self) -> &mut [Record] {
        self.records.as_mut_slice()
    }

    fn get_max_width_for_column(&self, column_key: &str) -> usize {
        let mut max_seen = 0;

        for record in self.records.iter() {
            let curr_row_width = record.get(column_key).map(|s| s.char_indices().count()).unwrap_or(0);
            max_seen = max_seen.max(curr_row_width);
        }

        max_seen
    }

    fn print_display_str(printer: &Printer, original_str: &str, display_width: usize) {
        // If there is not enough room to even print an ellipsis, just return.
        if display_width < ELLIPSIS_STR_WIDTH {
            return
        }

        let trunc_width = display_width - ELLIPSIS_STR_WIDTH;

        let mut char_indices = original_str.char_indices();

        // Skip the number of characters needed to show a truncated view.
        let (display_str, show_ellipsis) =
            match char_indices.by_ref().skip(trunc_width).peekable().peek() {
                // The number of characters in the string is less than or equal to
                // the truncated column width. Just show it as-is, with no ellipsis.
                None => (&original_str[..], false),

                // The number of characters in the string is greater than the
                // truncated column width. Check to see how that number compares to
                // the non-truncated column width.
                Some(&(trunc_pos, _)) => {
                    // Skip the number of characters in the ellipsis.
                    match char_indices.by_ref().skip(ELLIPSIS_STR_WIDTH).next() {
                        // The string will fit in the full column width.
                        // Just show as-is, with no ellipsis.
                        None => (&original_str[..], false),

                        // There are characters left that will not fit in the column.
                        // Return a slice of the string, with enough room left over
                        // to include an ellipsis.
                        Some(..) => (&original_str[..trunc_pos], true),
                    }
                },
            }
        ;

        if show_ellipsis {
            printer.print_hline((0, 0), display_width, ELLIPSIS_STR);
        }

        printer.print((0, 0), display_str);
    }

    /// Draws the contents of a column, by field.
    /// The `Printer` should be configured to begin printing at the correct starting position.
    fn draw_column(&self, printer: &Printer, field_key: &str, display_width: usize) {
        if display_width <= 0 {
            return;
        }

        self.scroll_base.draw(printer, |printer, i| {
            let record = &self.records[i];

            // See if this record contains the given field.
            match record.get(field_key) {
                None => {
                    // Print out a highlighted sentinel, to indicate a missing value.
                    printer.with_color(ColorStyle::highlight_inactive(), |pr| {
                        pr.print_hline((0, 0), display_width, MISSING_STR);
                    })
                },
                Some(field) => {
                    Self::print_display_str(printer, field, display_width);
                }
            }
        });
    }
}

impl Default for TagRecordView {
    fn default() -> Self {
        Self {
            records: Vec::new(),

            scroll_base: ScrollBase::new(),
        }
    }
}

pub struct TagEditorView {
    columns: IndexMap<String, ColumnDef>,
    tag_record_view: TagRecordView,

    last_size: Vec2,
}

impl Default for TagEditorView {
    /// Creates a new empty view without any columns.
    fn default() -> Self {
        Self::new()
    }
}

impl TagEditorView {
    /// Creates a new empty view without any columns.
    pub fn new() -> Self {
        Self {
            columns: IndexMap::new(),
            tag_record_view: Default::default(),
            last_size: Vec2::new(0, 0),
        }
    }
}

impl TagEditorView {
    fn trunc_column_str(original_str: &str, target_width: usize) -> (&str, bool) {
        match original_str.char_indices().skip(target_width).next() {
            // The number of characters in the string is less than or equal to
            // the truncated column width. Just show it as-is, with no ellipsis.
            None => (&original_str[..], false),

            // The number of characters in the string is greater than the
            // truncated column width. Slice the string to that point.
            Some((trunc_pos, _)) => (&original_str[..trunc_pos], true),
        }
    }

    pub fn draw_column_sep(printer: &Printer, height: usize) {
        if height > 0 {
            printer.print((0, 0), "│");
        }

        if height > 1 {
            printer.print((0, 1), "┼");
        }

        if height > 2 {
            let trailing_height = height - 2;
            printer.print_vline((0, 2), trailing_height, "│");
        }
    }

    pub fn draw_column(&self, column: &str, column_def: &ColumnDef, printer: &Printer) -> usize {
        // Actually want number of grapheme clusters, but this will do for now.
        // TODO: Look into the `unicode-segmentation` crate.
        let header_width = column_def.title.char_indices().count();
        let desired_width = column_def.desired_width;

        // This is the maximum content width.
        let data_width = Ord::max(desired_width, header_width);

        // This is the total width of the column, including padding and content.
        let column_width = 1 + data_width + 1;

        // Print the header for this column.
        // This uses the human-readable title.
        let (header, _was_trunc) = Self::trunc_column_str(&column_def.title, data_width);

        printer.print((1, 0), header);
        printer.print_hline((0, 1), column_width, "─");

        let printer = printer.offset((1, 2));

        self.tag_record_view.draw_column(&printer, column, data_width);

        // Return the actual width this column took.
        column_width
    }
}

impl View for TagEditorView {
    fn draw(&self, printer: &Printer) {
        let mut column_offset = 0;

        for (i, (column, column_def)) in self.columns.iter().enumerate() {
            if i > 0 {
                Self::draw_column_sep(&printer.offset((column_offset, 0)), 100);
                column_offset += 1;
            }

            let width_used = self.draw_column(
                column,
                column_def,
                &printer.offset((column_offset, 0)),
            );

            // Keep track of the width the last column took.
            column_offset += width_used;
        }
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
    }
}

fn main() {
    let records = vec![
        hashmap! {
            str!("name") => str!("Mark LeMoine"),
            str!("age") => str!("32"),
            str!("fave_food") => str!("tacos + burritos + burgers"),
        },
        hashmap! {
            str!("name") => str!("Susanne Barajas"),
            str!("age") => str!("27"),
            str!("fave_food") => str!("chicken lettuce wraps"),
        },
        hashmap! {
            str!("name") => str!("Leopoldo Marquez"),
            str!("age") => str!("29"),
            // str!("fave_food") => str!("steak"),
        },
    ];

    let columns = indexmap! {
        str!("name") => ColumnDef {
            title: str!("Name"),
            desired_width: 10,
        },
        str!("age") => ColumnDef {
            title: str!("Age"),
            desired_width: 10,
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            desired_width: 40,
        },
    };

    let ssv = TagEditorView {
        columns,
        tag_record_view: TagRecordView::with_records(records),
        ..Default::default()
    };

    let mut siv = Cursive::default();

    siv.add_layer(ssv.scrollable().min_size((30, 20)));

    siv.run();
}
