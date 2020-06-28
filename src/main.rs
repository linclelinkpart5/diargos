
mod model;
mod util;

use std::sync::Arc;
use std::sync::Mutex;

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
use cursive::traits::Resizable;
use cursive::traits::Scrollable;
use cursive::view::View;
use cursive::views::Canvas;
use cursive::views::LinearLayout;

use self::model::ColumnDef;
use self::model::Model;
use self::model::Sizing;
use self::util::Util;

const ELLIPSIS_STR: &str = "⋯";
const ELLIPSIS_STR_WIDTH: usize = 1;

const MISSING_STR: &str = "╳";

const COLUMN_SEP: &str = " │ ";
const COLUMN_HEADER_SEP: &str = "─┼─";
const COLUMN_SEP_WIDTH: usize = 3;

const COLUMN_HEADER_BAR: &str = "─";

pub struct TagEditorView(LinearLayout);

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
                        printer.print((offset_x, 1), COLUMN_HEADER_SEP);
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

                    printer.print_hline((offset_x, 1), content_width, COLUMN_HEADER_BAR);

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

        // TODO: See if there is an option to allow the records `Canvas` to have
        //       its scrollbar always visible.
        // TODO: Drawing vertically one column at a time might be slow, test out
        //       horizontal drawing.
        let linear_layout =
            LinearLayout::vertical()
            .child(columns_canvas)
            .child(records_canvas)
        ;

        Self(linear_layout)
    }
}

impl View for TagEditorView {
    fn draw(&self, printer: &Printer) {
        self.0.draw(printer);
    }

    fn layout(&mut self, constraint: XY<usize>) {
        self.0.layout(constraint)
    }

    fn required_size(&mut self, constraint: XY<usize>) -> XY<usize> {
        self.0.required_size(constraint)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        self.0.on_event(event)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.0.take_focus(source)
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
            sizing: Sizing::Fixed(10),
        },
        str!("age") => ColumnDef {
            title: str!("Age"),
            sizing: Sizing::Fixed(10),
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            sizing: Sizing::Fixed(500),
        },
    };

    let model = Model::with_data(columns, records);

    let tag_editor_view = TagEditorView::new(model);

    let mut siv = Cursive::default();

    siv.add_layer(
        tag_editor_view
        .scrollable()
        .scroll_x(true)
        .fixed_size((30, 20))
    );

    siv.run();
}
