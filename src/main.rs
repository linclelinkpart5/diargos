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
use cursive::vec::Vec2;
use cursive::view::ScrollBase;
use cursive::view::View;
use cursive::views::Dialog;

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

pub struct SpreadsheetView {
    columns: IndexMap<String, ColumnDef>,
    records: Vec<Record>,

    enabled: bool,
    scroll_base: ScrollBase,
    last_size: Vec2,
    read_only: bool,

    cursor_pos: Option<(usize, usize)>,
    selected_cells: HashSet<(usize, usize)>,
    column_select: bool,

    on_sort: Option<OnSortCallback>,
    on_submit: Option<IndexCallback>,
    on_select: Option<IndexCallback>,
}

impl Default for SpreadsheetView {
    /// Creates a new empty view without any columns.
    fn default() -> Self {
        Self::new()
    }
}

impl SpreadsheetView {
    /// Creates a new empty view without any columns.
    pub fn new() -> Self {
        Self {
            columns: IndexMap::new(),
            records: Vec::new(),

            enabled: true,
            scroll_base: ScrollBase::new(),
            last_size: Vec2::new(0, 0),
            read_only: true,

            cursor_pos: None,
            selected_cells: HashSet::new(),
            column_select: false,

            on_sort: None,
            on_submit: None,
            on_select: None,
        }
    }
}

impl SpreadsheetView {
    pub fn draw_column(&self, column: &str, column_def: &ColumnDef, printer: &Printer) -> usize {
        // Actually want number of grapheme clusters, but this will do for now.
        // TODO: Look into the `unicode-segmentation` crate.
        let header_width = column.chars().count();
        let desired_width = column_def.desired_width;

        // This will need to change for Unicode text, but it will do for now.
        let column_width = Ord::max(desired_width, header_width);

        for (row_offset, record) in self.records.iter().enumerate() {
            let sub_printer = &printer.offset((0, row_offset)).focused(true);

            // See if this record has the target column.
            match record.get(column) {
                None => {
                    sub_printer.with_color(ColorStyle::highlight(), |pr| {
                        pr.print_hline((0, 0), column_width, "X");
                    })
                },
                Some(field) => {
                    let trunc_field = match field.char_indices().skip(column_width).next() {
                        Some((pos, _)) => &field[..pos],
                        None => &field,
                    };

                    sub_printer.print((0, 0), trunc_field);
                },
            };
        }

        // Return the actual width this column took.
        column_width
    }
}

impl View for SpreadsheetView {
    fn draw(&self, printer: &Printer) {
        let mut column_offset = 0;

        for (column, column_def) in self.columns.iter() {
            printer.print_vline((column_offset, 0), 100, "|");

            column_offset += 2;

            let width_used = self.draw_column(column, column_def, &printer.offset((column_offset, 0)));

            // Keep track of the width the last column took.
            column_offset += width_used + 1;
        }

        printer.print_vline((column_offset, 0), 100, "|");
    }
}

fn main() {
    let ssv = SpreadsheetView {
        columns: indexmap! {
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
                desired_width: 10,
            },
        },
        records: vec![
            hashmap! {
                str!("name") => str!("Mark LeMoine"),
                str!("age") => str!("32"),
                str!("fave_food") => str!("tacos"),
            },
            hashmap! {
                str!("name") => str!("Susanne Barajas"),
                str!("age") => str!("27"),
                str!("fave_food") => str!("chicken lettuce wraps"),
            },
            hashmap! {
                str!("name") => str!("Leopoldo Marquez"),
                str!("age") => str!("29"),
                str!("fave_food") => str!("steak"),
            },
        ],
        ..Default::default()
    };

    let mut siv = Cursive::default();

    siv.add_layer(Dialog::around(ssv.with_name("table").min_size((50, 20))).title("Tag View"));

    siv.run();
}
