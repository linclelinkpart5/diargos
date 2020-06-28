
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
use cursive::event::Event;
use cursive::event::EventResult;
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
use self::model::ColumnDef;
use self::model::Model;
use self::model::Record;
use self::model::Sizing;
use self::util::Util;

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
            .with_layout(|model, _constraints| {
                let mut model = model.lock().unwrap();
                model.recache();
            })
            .with_draw(|model, printer| {
                let model = model.lock().unwrap();

                let mut offset_x = 0;
                let mut is_first_col = true;

                for (column_def, content_width) in model.columns.values().zip(model.iter_cache()) {
                    if is_first_col { is_first_col = false; }
                    else {
                        printer.print((offset_x, 0), COLUMN_SEP);
                        offset_x += COLUMN_SEP_WIDTH;
                    }

                    let title = &column_def.title;

                    let (display_title, was_trimmed) = Util::trim_display_str(
                        title,
                        content_width,
                        ELLIPSIS_STR_WIDTH,
                    );

                    if was_trimmed {
                        printer.print_hline((offset_x, 0), content_width, ELLIPSIS_STR);
                    }

                    printer.print((offset_x, 0), display_title);

                    offset_x += content_width;
                }
            })
            .with_required_size(|model, _constraints| {
                let mut model = model.lock().unwrap();
                model.recache();
                let total_width = model.total_display_width(COLUMN_SEP_WIDTH);

                (total_width, 2).into()
            })
        ;

        let records_canvas =
            Canvas::new(shared_model.clone())
            .with_draw(|model, printer| {
                let model = model.lock().unwrap();

                let mut offset_x = 0;
                let mut is_first_col = true;

                for (column_key, content_width) in model.columns.keys().zip(model.iter_cache()) {
                    if is_first_col { is_first_col = false; }
                    else {
                        printer.print_vline((offset_x, 0), model.records.len(), COLUMN_SEP);
                        offset_x += COLUMN_SEP_WIDTH;
                    }

                    for (offset_y, record) in model.records.iter().enumerate() {
                        match record.get(column_key) {
                            None => {
                                // Print out a highlighted sentinel, to indicate a missing value.
                                printer.with_color(ColorStyle::highlight_inactive(), |pr| {
                                    pr.print_hline((offset_x, offset_y), content_width, MISSING_STR);
                                });
                            },
                            Some(value) => {
                                let (display_value, was_trimmed) = Util::trim_display_str(
                                    value,
                                    content_width,
                                    ELLIPSIS_STR_WIDTH,
                                );

                                if was_trimmed {
                                    printer.print_hline((offset_x, offset_y), content_width, ELLIPSIS_STR);
                                }

                                printer.print((offset_x, offset_y), display_value);
                            },
                        }
                    }

                    offset_x += content_width;
                }
            })
            .with_required_size(|model, _constraints| {
                let mut model = model.lock().unwrap();
                model.recache();
                let total_width = model.total_display_width(COLUMN_SEP_WIDTH);

                (total_width, model.records.len()).into()
            })
            .scrollable()
            .scroll_x(false)
            .scroll_y(true)
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

impl View for TagEditorView {
    fn draw(&self, printer: &Printer) {
        self.linear_layout.draw(printer);
    }

    fn layout(&mut self, constraint: XY<usize>) {
        self.linear_layout.layout(constraint)
    }

    fn required_size(&mut self, constraint: XY<usize>) -> XY<usize> {
        self.linear_layout.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.linear_layout.on_event(event)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.linear_layout.take_focus(source)
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

    let columns = indexmap! {
        str!("name") => ColumnDef {
            title: str!("Name"),
            sizing: Sizing::Fixed(40),
        },
        str!("age") => ColumnDef {
            title: str!("Age"),
            sizing: Sizing::Fixed(10),
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            sizing: Sizing::Fixed(40),
        },
    };

    let model = Model::with_data(columns, records);

    let tag_editor_view = TagEditorView::new(model);

    let mut siv = Cursive::default();

    siv.add_layer(tag_editor_view.scrollable().scroll_x(true));

    siv.run();
}
