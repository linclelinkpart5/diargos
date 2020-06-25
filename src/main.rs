use std::collections::HashMap;

// use indexmap::IndexMap;
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
// use cursive::traits::Resizable;
use cursive::traits::Scrollable;
// use cursive::vec::Vec2;
// use cursive::view::ScrollBase;
use cursive::view::View;
// use cursive::views::Canvas;
// use cursive::views::Dialog;
// use cursive::views::Panel;
// use cursive::views::ScrollView;
// use cursive::views::TextView;

const ELLIPSIS_STR: &str = "⋯";
const ELLIPSIS_STR_WIDTH: usize = 1;

const MISSING_STR: &str = "╳";

pub enum ColumnWidth {
    Auto,
    Fixed(usize),
}

fn width(s: &str) -> usize {
    s.char_indices().count()
}

pub type Record = HashMap<String, String>;

pub struct ColumnDef {
    /// A friendly human-readable name for the column, used for display.
    pub title: String,

    /// Display width for this column.
    pub width: ColumnWidth,
}

pub struct TagRecordModel(Vec<Record>);

impl TagRecordModel {
    pub fn new() -> Self {
        Self::with_records(Vec::new())
    }

    pub fn with_records(records: Vec<Record>) -> Self {
        Self(records)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn records(&self) -> &[Record] {
        self.0.as_slice()
    }

    pub fn records_mut(&mut self) -> &mut [Record] {
        self.0.as_mut_slice()
    }

    fn get_max_width_for_column(&self, column_key: &str) -> usize {
        let mut max_seen = 0;

        for record in self.0.iter() {
            let curr_row_width = record.get(column_key).map(|s| width(s)).unwrap_or(0);
            max_seen = max_seen.max(curr_row_width);
        }

        max_seen
    }

    fn trim_display_str(original_str: &str, display_width: usize) -> (&str, bool) {
        // If there is not enough room to even print an ellipsis, just return.
        if display_width < ELLIPSIS_STR_WIDTH {
            return ("", original_str != "")
        }

        let trunc_width = display_width - ELLIPSIS_STR_WIDTH;

        let mut char_indices = original_str.char_indices();

        // Skip the number of characters needed to show a truncated view.
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
    }

    fn draw_columns<'a, I>(&'a self, printer: &'a Printer, keys_and_widths: I)
    where
        I: IntoIterator<Item = (&'a str, usize)>,
    {
        let mut column_offset = 0;
        let mut is_first_col = true;

        for (column_key, content_width) in keys_and_widths {
            if is_first_col { is_first_col = false; }
            else {
                // Pad, then draw a vertical separator, then pad again.
                column_offset += 1;
                printer.print_vline((column_offset, 0), self.0.len(), "│");
                column_offset += 1;
                column_offset += 1;
            }

            // Only do work if the content width is greater than 0.
            if content_width > 0 {
                for (row_offset, record) in self.0.iter().enumerate() {
                    // See if this record contains the given field.
                    match record.get(column_key) {
                        None => {
                            // Print out a highlighted sentinel, to indicate a missing value.
                            printer.with_color(ColorStyle::highlight_inactive(), |pr| {
                                pr.print_hline((column_offset, row_offset), content_width, MISSING_STR);
                            })
                        },
                        Some(field) => {
                            let (trimmed_field, was_trimmed) = Self::trim_display_str(field, content_width);

                            if was_trimmed {
                                printer.print_hline((column_offset, row_offset), content_width, ELLIPSIS_STR);
                            }

                            printer.print((column_offset, row_offset), trimmed_field);
                        }
                    }
                }

                // Increment the offset.
                column_offset += content_width;
            }
        }
    }
}

impl View for TagRecordModel {
    fn draw(&self, printer: &Printer) {
        self.draw_columns(printer, vec![
            ("name", 20),
            ("fave_food", 30),
            ("age", 10),
        ])
    }

    fn required_size(&mut self, constraint: XY<usize>) -> XY<usize> {
        // constraint

        // NOTE: This seems to cause the `ScrollView` to work.
        //       The `ScrollView` ends up "governing" an underlying `View` that
        //       has this size, and scrolls the viewport in both axes.
        (10000, 100).into()
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        true
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
    //         width: ColumnWidth::Fixed(10),
    //     },
    //     str!("age") => ColumnDef {
    //         title: str!("Age"),
    //         width: ColumnWidth::Fixed(10),
    //     },
    //     str!("fave_food") => ColumnDef {
    //         title: str!("Favorite Food"),
    //         width: ColumnWidth::Fixed(40),
    //     },
    // };

    let trv = TagRecordModel::with_records(records);

    let mut siv = Cursive::default();

    siv.add_layer(trv.scrollable().scroll_x(true).scroll_y(true));

    // let dialog = Dialog::around(Panel::new(TextView::new(include_str!("main.rs")).scrollable()))
    //     .title("Unicode and wide-character support")
    //     // This is the alignment for the button
    //     .h_align(HAlign::Center)
    //     .button("Quit", |s| s.quit());

    // siv.add_layer(dialog);

    siv.run();
}
