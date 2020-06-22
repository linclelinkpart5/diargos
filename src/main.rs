use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use indexmap::IndexMap;

use cursive::Cursive;
use cursive::Printer;
use cursive::align::HAlign;
use cursive::direction::Direction;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::theme::ColorStyle;
use cursive::vec::Vec2;
use cursive::view::ScrollBase;
use cursive::view::View;

pub type Record = HashMap<String, String>;

pub struct ColumnDef {
    /// A friendly human-readable name for the column, used for display.
    pub title: String,

    /// Desired column width, actual column width my be longer than this to
    /// accomodate the header display.
    pub desired_width: usize,

    /// Horizontal alignment of the header for this column.
    pub header_align: HAlign,

    /// Horizontal alignment of the data for this column.
    pub data_align: HAlign,

    /// Flags if this column has been selected.
    pub selected: bool,
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
    pub fn draw_column<C>(&self, column: &str, printer: &Printer) -> Option<usize>
    where
        C: Fn(&Printer, &str)
    {
        let column_def = self.columns.get(column)?;

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

        Some(column_width)
    }
}

fn main() {
    // let mut rng = rand::thread_rng();

    // let mut siv = Cursive::default();
    // let mut table = TableView::<Foo, BasicColumn>::new()
    //     .column(BasicColumn::Name, "Name", |c| c.width_percent(20))
    //     .column(BasicColumn::Count, "Count", |c| c.align(HAlign::Center))
    //     .column(BasicColumn::Rate, "Rate", |c| {
    //         c.ordering(Ordering::Greater)
    //             .align(HAlign::Right)
    //             .width_percent(20)
    //     });

    // let mut items = Vec::new();
    // for i in 0..50 {
    //     items.push(Foo {
    //         name: format!("Name {}", i),
    //         count: rng.gen_range(0, 255),
    //         rate: rng.gen_range(0, 255),
    //     });
    // }

    // table.set_items(items);

    // table.set_on_sort(|siv: &mut Cursive, column: BasicColumn, order: Ordering| {
    //     siv.add_layer(
    //         Dialog::around(TextView::new(format!("{} / {:?}", column.as_str(), order)))
    //             .title("Sorted by")
    //             .button("Close", |s| {
    //                 s.pop_layer();
    //             }),
    //     );
    // });

    // table.set_on_submit(|siv: &mut Cursive, row: usize, index: usize| {
    //     let value = siv
    //         .call_on_name("table", move |table: &mut TableView<Foo, BasicColumn>| {
    //             format!("{:?}", table.borrow_item(index).unwrap())
    //         })
    //         .unwrap();

    //     siv.add_layer(
    //         Dialog::around(TextView::new(value))
    //             .title(format!("Removing row # {}", row))
    //             .button("Close", move |s| {
    //                 s.call_on_name("table", |table: &mut TableView<Foo, BasicColumn>| {
    //                     table.remove_item(index);
    //                 });
    //                 s.pop_layer();
    //             }),
    //     );
    // });

    // siv.add_layer(Dialog::around(table.with_name("table").min_size((50, 20))).title("Table View"));

    // siv.run();
}
