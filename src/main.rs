
mod model;
mod util;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use indexmap::IndexMap;
use indexmap::indexmap;
use maplit::hashmap;
use str_macro::str;

use cursive::Cursive;
use cursive::CursiveExt;
use cursive::Printer;
use cursive::XY;
use cursive::direction::Direction;
// use cursive::event::Event;
// use cursive::event::EventResult;
use cursive::theme::ColorStyle;
// use cursive::traits::Nameable;
use cursive::traits::Resizable;
use cursive::traits::Scrollable;
// use cursive::vec::Vec2;
// use cursive::view::ScrollBase;
use cursive::view::View;
use cursive::views::Canvas;
// use cursive::views::Dialog;
use cursive::views::LinearLayout;
// use cursive::views::Panel;
// use cursive::views::ScrollView;
// use cursive::views::TextView;

use self::model::Columns;
use self::model::Model;
use self::model::Record;
use self::model::Sizing;

const ELLIPSIS_STR: &str = "⋯";
const ELLIPSIS_STR_WIDTH: usize = 1;

const MISSING_STR: &str = "╳";

const COLUMN_SEP: &str = " │ ";
const COLUMN_SEP_WIDTH: usize = 3;


pub struct TagEditorView {
    /// Contains all of the columns and records to display in this view.
    shared_model: Arc<Mutex<Model>>,

    linear_layout: LinearLayout,
}

impl TagEditorView {
    pub fn new(model: Model) -> Self {
        let shared_model = Arc::new(Mutex::new(model));

        let columns_canvas =
            Canvas::new(shared_model.clone())
            .with_draw(|_model, _printer| {})
        ;
        let records_canvas =
            Canvas::new(shared_model.clone())
            .with_draw(|_model, _printer| {})
        ;

        let linear_layout =
            LinearLayout::vertical()
            .child(columns_canvas)
            .child(records_canvas)
        ;

        Self {
            shared_model,
            linear_layout,
        }
    }
}

// fn max_column_content_width(records: &[Record], column_key: &str) -> usize {
//     let mut max_seen = 0;

//     for record in records.iter() {
//         let curr_row_width = record.get(column_key).map(|s| str_width(s)).unwrap_or(0);
//         max_seen = max_seen.max(curr_row_width);
//     }

//     max_seen
// }

// fn total_display_size(columns: &IndexMap<String, ColumnDef>, records: &[Record]) -> (usize, usize) {
//     // The total Y length is easy, it is just the number of records.
//     let y = records.len();

//     let mut x = 0;
//     let mut is_first_col = true;

//     let keys_and_sizings = columns.iter().map(|(k, d)| (k.as_str(), d.sizing));

//     for (column_key, sizing) in keys_and_sizings {
//         if is_first_col { is_first_col = false; }
//         else {
//             // Draw the column separator, and increment X by the column separator length.
//             // opt_printer.map(|pr| { pr.print_vline((x, 0), y, COLUMN_SEP); });
//             x += COLUMN_SEP_WIDTH;
//         }

//         // Resolve the actual content width.
//         let content_width = match sizing {
//             Sizing::Fixed(width) => width,
//             Sizing::Auto => max_column_content_width(records, column_key),
//         };

//         // Print out the column, if a `Printer` was provided.
//         // if let Some(printer) = opt_printer {
//         //     if content_width > 0 {
//         //         for (row_offset, record) in self.0.iter().enumerate() {
//         //             // See if this record contains the given field.
//         //             match record.get(column_key) {
//         //                 None => {
//         //                     // Print out a highlighted sentinel, to indicate a missing value.
//         //                     printer.with_color(ColorStyle::highlight_inactive(), |pr| {
//         //                         pr.print_hline((x, row_offset), content_width, MISSING_STR);
//         //                     });
//         //                 },
//         //                 Some(field) => {
//         //                     let (trimmed_field, was_trimmed) = trim_display_str(field, content_width);

//         //                     if was_trimmed {
//         //                         printer.print_hline((x, row_offset), content_width, ELLIPSIS_STR);
//         //                     }

//         //                     printer.print((x, row_offset), trimmed_field);
//         //                 }
//         //             }
//         //         }
//         //     }
//         // }

//         // Increment X by the content width.
//         x += content_width;
//     }

//     (x, y)
// }

// pub struct TagRecordModel(Vec<Record>);

// impl TagRecordModel {
//     pub fn new() -> Self {
//         Self::with_records(Vec::new())
//     }

//     pub fn with_records(records: Vec<Record>) -> Self {
//         Self(records)
//     }

//     pub fn len(&self) -> usize {
//         self.0.len()
//     }

//     pub fn records(&self) -> &[Record] {
//         self.0.as_slice()
//     }

//     pub fn records_mut(&mut self) -> &mut [Record] {
//         self.0.as_mut_slice()
//     }

//     fn max_width_for_column(&self, column_key: &str) -> usize {
//         let mut max_seen = 0;

//         for record in self.0.iter() {
//             let curr_row_width = record.get(column_key).map(|s| str_width(s)).unwrap_or(0);
//             max_seen = max_seen.max(curr_row_width);
//         }

//         max_seen
//     }

//     fn calc_extents_and_optionally_draw<'a, I>(&'a self, keys_and_sizings: I, opt_printer: Option<&'a Printer>) -> XY<usize>
//     where
//         I: IntoIterator<Item = (&'a str, Sizing)>
//     {
//         // The total Y length is easy, it is just the number of records.
//         let total_y = self.0.len();

//         let mut curr_x = 0;
//         let mut is_first_col = true;

//         for (column_key, sizing) in keys_and_sizings {
//             if is_first_col { is_first_col = false; }
//             else {
//                 // Draw the column separator, and increment X by the column separator length.
//                 opt_printer.map(|pr| { pr.print_vline((curr_x, 0), total_y, COLUMN_SEP); });
//                 curr_x += COLUMN_SEP_WIDTH;
//             }

//             // Resolve the actual content width.
//             let content_width = match sizing {
//                 Sizing::Fixed(width) => width,

//                 // TODO: Actually calculate!
//                 Sizing::Auto => 20,
//             };

//             // Print out the column, if a `Printer` was provided.
//             if let Some(printer) = opt_printer {
//                 if content_width > 0 {
//                     for (row_offset, record) in self.0.iter().enumerate() {
//                         // See if this record contains the given field.
//                         match record.get(column_key) {
//                             None => {
//                                 // Print out a highlighted sentinel, to indicate a missing value.
//                                 printer.with_color(ColorStyle::highlight_inactive(), |pr| {
//                                     pr.print_hline((curr_x, row_offset), content_width, MISSING_STR);
//                                 });
//                             },
//                             Some(field) => {
//                                 let (trimmed_field, was_trimmed) = trim_display_str(field, content_width);

//                                 if was_trimmed {
//                                     printer.print_hline((curr_x, row_offset), content_width, ELLIPSIS_STR);
//                                 }

//                                 printer.print((curr_x, row_offset), trimmed_field);
//                             }
//                         }
//                     }
//                 }
//             }

//             // Increment X by the content width.
//             curr_x += content_width;
//         }

//         (curr_x, total_y).into()
//     }

//     fn draw_columns<'a, I>(&'a self, printer: &'a Printer, keys_and_sizings: I)
//     where
//         I: IntoIterator<Item = (&'a str, Sizing)>,
//     {
//         self.calc_extents_and_optionally_draw(keys_and_sizings, Some(printer));
//     }
// }

// impl View for TagRecordModel {
//     fn draw(&self, printer: &Printer) {
//         self.draw_columns(printer, vec![
//             ("name", Sizing::Fixed(20)),
//             ("fave_food", Sizing::Fixed(30)),
//             ("age", Sizing::Fixed(10)),
//         ])
//     }

//     fn required_size(&mut self, _constraint: XY<usize>) -> XY<usize> {
//         let keys_and_sizings = vec![
//             ("name", Sizing::Fixed(20)),
//             ("fave_food", Sizing::Fixed(30)),
//             ("age", Sizing::Fixed(10)),
//         ];

//         self.calc_extents_and_optionally_draw(keys_and_sizings, None)
//     }

//     fn take_focus(&mut self, _: Direction) -> bool {
//         true
//     }
// }

// pub struct NewTagEditorView {
//     columns: IndexMap<String, ColumnDef>,
//     records: Vec<Record>,

//     layout: LinearLayout,

//     cached_content_widths: Vec<usize>,
// }

// impl NewTagEditorView {
//     pub fn new() -> Self {
//         Self {
//             columns: IndexMap::new(),
//             records: Vec::new(),
//             layout: LinearLayout::vertical(),

//             cached_content_widths: Vec::new(),
//         }
//     }

//     fn max_column_content_width(&self, column_key: &str) -> usize {
//         let mut max_seen = 0;

//         for record in self.records.iter() {
//             let curr_row_width = record.get(column_key).map(|s| str_width(s)).unwrap_or(0);
//             max_seen = max_seen.max(curr_row_width);
//         }

//         max_seen
//     }

//     fn calculate_content_widths(&mut self) {
//         self.cached_content_widths.clear();
//         self.cached_content_widths.reserve(self.columns.len());

//         for (column_key, column_def) in self.columns.iter() {
//             let column_sizing = column_def.sizing;

//             let content_width = match column_sizing {
//                 Sizing::Fixed(width) => width,
//                 Sizing::Auto => self.max_column_content_width(column_key),
//             };

//             self.cached_content_widths.push(content_width);
//         }

//         assert_eq!(self.cached_content_widths.len(), self.columns.len());
//     }

//     fn calculate_bounds_optionally_draw(&self, opt_printer: Option<&Printer>) -> (usize, usize) {
//         // The total Y length is easy, it is just the number of records plus the height of the header.
//         let curr_y = 0;

//         let mut curr_x = 0;
//         let mut is_first_col = true;

//         for (column_key, &content_width) in self.columns.keys().zip(self.cached_content_widths.iter()) {
//             if is_first_col { is_first_col = false; }
//             else {
//                 // Draw the column separator, and increment X by the column separator length.
//                 opt_printer.map(|pr| { pr.print_vline((curr_x, 0), curr_y, COLUMN_SEP); });
//                 curr_x += COLUMN_SEP_WIDTH;
//             }

//             // Print out the column, if a `Printer` was provided.
//             if let Some(printer) = opt_printer {
//                 if content_width > 0 {
//                     for (row_offset, record) in self.records.iter().enumerate() {
//                         // See if this record contains the given field.
//                         match record.get(column_key) {
//                             None => {
//                                 // Print out a highlighted sentinel, to indicate a missing value.
//                                 printer.with_color(ColorStyle::highlight_inactive(), |pr| {
//                                     pr.print_hline((curr_x, row_offset), content_width, MISSING_STR);
//                                 });
//                             },
//                             Some(field) => {
//                                 let (trimmed_field, was_trimmed) = trim_display_str(field, content_width);

//                                 if was_trimmed {
//                                     printer.print_hline((curr_x, row_offset), content_width, ELLIPSIS_STR);
//                                 }

//                                 printer.print((curr_x, row_offset), trimmed_field);
//                             }
//                         }
//                     }
//                 }
//             }

//             // Increment X by the content width.
//             curr_x += content_width;
//         }

//         (curr_x, curr_y)
//     }
// }

// pub struct TagEditorView {
//     columns: IndexMap<String, ColumnDef>,
//     model: TagRecordModel,
// }

// impl TagEditorView {
//     fn iter_keys_and_sizings(&self) -> impl Iterator<Item = (&str, Sizing)> {
//         self.columns.iter().map(|(k, d)| (k.as_str(), d.sizing))
//     }

//     // fn calc_extents<'a, I>(&'a self, opt_printer: Option<&'a Printer>) -> XY<usize>
//     // where
//     //     I: IntoIterator<Item = (&'a str, Sizing)>
//     // {
//     //     // The total X size is the same as that of the model.
//     //     // The total Y size is the Y size of the model plus 2, for the header.
//     //     let mut total_y = 0;

//     //     // Draw the header, if requested.
//     //     if let Some(printer) = opt_printer.as_ref() {

//     //     }

//     //     total_y += 2;
//     //     let opt_offset_printer = opt_printer.map(|pr| pr.offset((0, total_y)));
//     //     let model_extents = self.model.calc_extents_and_optionally_draw(self.iter_keys_and_sizings(), opt_printer);
//     // }
// }

// impl View for TagEditorView {
//     fn draw(&self, printer: &Printer) {
//         self.model.calc_extents_and_optionally_draw(self.iter_keys_and_sizings(), Some(printer));
//     }

//     fn required_size(&mut self, _constraint: XY<usize>) -> XY<usize> {
//         let mut model_size = self.model.calc_extents_and_optionally_draw(self.iter_keys_and_sizings(), None);
//         model_size
//     }

//     fn take_focus(&mut self, _: Direction) -> bool {
//         true
//     }
// }

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
            str!("name") => str!("Leopoldo Marquezyayayayayaya"),
            str!("age") => str!("29"),
            // str!("fave_food") => str!("steak"),
        },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
        hashmap! { str!("name") => str!("Numi") },
    ];

    // let columns = indexmap! {
    //     str!("name") => ColumnDef {
    //         title: str!("Name"),
    //         width: Sizing::Fixed(10),
    //     },
    //     str!("age") => ColumnDef {
    //         title: str!("Age"),
    //         width: Sizing::Fixed(10),
    //     },
    //     str!("fave_food") => ColumnDef {
    //         title: str!("Favorite Food"),
    //         width: Sizing::Fixed(40),
    //     },
    // };

    // let trv = TagRecordModel::with_records(records);

    let mut siv = Cursive::default();

    // siv.add_layer(trv.scrollable().scroll_x(true).scroll_y(true).fixed_size((20, 20)));

    // let dialog = Dialog::around(Panel::new(TextView::new(include_str!("main.rs")).scrollable()))
    //     .title("Unicode and wide-character support")
    //     // This is the alignment for the button
    //     .h_align(HAlign::Center)
    //     .button("Quit", |s| s.quit());

    // siv.add_layer(dialog);

    siv.run();
}
