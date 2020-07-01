
mod consts;
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
use cursive::Rect;
use cursive::XY;
use cursive::direction::Direction;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::theme::ColorStyle;
use cursive::traits::Resizable;
use cursive::traits::Scrollable;
use cursive::view::ScrollBase;
use cursive::view::View;
use cursive::view::scroll::Core as ScrollCore;
use cursive::view::scroll::Scroller;
use cursive::views::Canvas;
use cursive::views::LinearLayout;

use crate::consts::*;
use crate::model::Columns;
use crate::model::ColumnDef;
use crate::model::Model;
use crate::model::Record;
use crate::model::Records;
use crate::model::Sizing;
use crate::util::Util;

pub struct TagRecordView {
    shared_model: Arc<Mutex<Model>>,
    scroll_core: ScrollCore,
}

impl Scroller for TagRecordView {
    fn get_scroller(&self) -> &ScrollCore {
        &self.scroll_core
    }

    fn get_scroller_mut(&mut self) -> &mut ScrollCore {
        &mut self.scroll_core
    }
}

impl View for TagRecordView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let offset_x = self.scroll_core.content_viewport().left();

        cursive::view::scroll::draw(self, printer, |scroller, printer| {});
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        cursive::view::scroll::on_event(
            self,
            event,
            |s, e| EventResult::Ignored,
            |s, si| Rect::from_size((0, 0), (1, 1)),
        )
    }
}

pub struct TagEditorView {
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

                let mut offset_x = 0;
                let mut is_first_col = true;

                for content_width in model.iter_cache() {
                    if is_first_col { is_first_col = false; }
                    else {
                        printer.print((offset_x, 1), COLUMN_HEADER_SEP);
                        offset_x += COLUMN_SEP_WIDTH;
                    }

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

                for (offset_y, record) in model.records.iter().enumerate() {
                    let mut offset_x = 0;
                    let mut is_first_col = true;

                    for (column_key, content_width) in model.columns.keys().zip(model.iter_cache()) {
                        if is_first_col { is_first_col = false; }
                        else {
                            printer.print((offset_x, offset_y), COLUMN_SEP);
                            offset_x += COLUMN_SEP_WIDTH;
                        }

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

                        offset_x += content_width;
                    }
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

        Self {
            shared_model,
            linear_layout,
        }
    }

    pub fn mutate_columns<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Columns) -> R,
    {
        let mut model = self.shared_model.lock().unwrap();
        model.mutate_columns(func)
    }

    pub fn mutate_records<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Records) -> R,
    {
        let mut model = self.shared_model.lock().unwrap();
        model.mutate_records(func)
    }

    pub fn push_record(&mut self, record: Record) {
        let mut model = self.shared_model.lock().unwrap();
        model.mutate_records(move |m| m.push(record))
    }

    pub fn extend_records(&mut self, records: Records) {
        let mut model = self.shared_model.lock().unwrap();
        model.mutate_records(move |m| m.extend(records))
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
    use rand::seq::SliceRandom;
    use rand::seq::IteratorRandom;

    let fave_foods = vec![
        "pizza",
        "steak",
        "lasagna",
        "tacos",
        "burritos",
        "chicken",
        "burgers",
        "waffles",
        "sushi",
        "curry",
        "ice cream",
        "brownies",
        "popcorn",
        "burritos",
        "fried rice",
    ];

    let names = vec![
        "Raina Salas",
        "Mariah Hernandez",
        "Kadin Rivas",
        "Osvaldo Hebert",
        "Adrien Lutz",
        "Peyton Mckenzie",
        "Valentin Nixon",
        "Greta Miles",
        "Cameron French",
        "Jayden Romero",
        "Alden Conrad",
        "Peter King",
        "Jake Duncan",
        "Shaun Barr",
        "Danna Shannon",
        "日本人の氏名",
    ];

    let num_records = 100;
    let mut rng = rand::thread_rng();

    let records =
        (0..=num_records)
        .map(|_| {
            hashmap! {
                str!("name") => names.choose(&mut rng).unwrap().to_string(),
                str!("age") => str!((18..=70).choose(&mut rng).unwrap()),
                str!("fave_food") => fave_foods.choose(&mut rng).unwrap().to_string(),
            }
        })
        .collect::<Vec<_>>()
    ;

    let columns = indexmap! {
        str!("name") => ColumnDef {
            title: str!("Name"),
            sizing: Sizing::Auto,
        },
        str!("age") => ColumnDef {
            title: str!("Age"),
            sizing: Sizing::Auto,
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            sizing: Sizing::Auto,
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
