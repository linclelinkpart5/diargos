
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
use cursive::XY;
use cursive::direction::Direction;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::theme::ColorStyle;
use cursive::view::View;
use cursive::view::scroll::Scroller;
use cursive::views::Canvas;
use cursive::views::Dialog;
use cursive::views::ScrollView;
use unicode_width::UnicodeWidthStr;

use crate::consts::*;
use crate::model::ColumnDef;
use crate::model::Data;
use crate::model::Model;
use crate::model::Sizing;
use crate::util::Util;

pub struct TagRecordView {
    shared_model: Arc<Mutex<Model>>,
    scroll_view: ScrollView<Canvas<Arc<Mutex<Model>>>>,
}

impl TagRecordView {
    pub fn new(model: Model) -> Self {
        // use std::fs::OpenOptions;
        // use std::io::prelude::*;

        let shared_model = Arc::new(Mutex::new(model));

        // first_visible_record = printer.content_offset.y
        // num_visible_records = printer.output_size.y

        let canvas =
            Canvas::new(shared_model.clone())
            .with_draw(|shared_model, printer| {
                // let mut file =
                //     OpenOptions::new()
                //     .create(true)
                //     .write(true)
                //     .append(true)
                //     .open("logs.txt")
                //     .unwrap()
                // ;

                // let log = format!("{:?}, {:?}\n", printer.output_size, printer.content_offset);
                // file.write_all(log.as_bytes()).unwrap();

                let model = shared_model.lock().unwrap();

                for (offset_y, record) in model.get_data().records.iter().enumerate() {
                    let mut offset_x = 0;
                    let mut is_first_col = true;

                    for (column_key, content_width) in model.get_data().columns.keys().zip(model.iter_cached_widths()) {
                        if is_first_col { is_first_col = false; }
                        else {
                            printer.print((offset_x, offset_y), COLUMN_SEP);
                            offset_x += COLUMN_SEP.width();
                        }

                        match record.get(column_key) {
                            None => {
                                // Print out a highlighted sentinel, to indicate a missing value.
                                printer.with_color(ColorStyle::highlight_inactive(), |pr| {
                                    pr.print_hline((offset_x, offset_y), content_width, MISSING_STR);
                                });
                            },
                            Some(value) => {
                                // Rough approximation for capacity.
                                let mut buffer = String::with_capacity(content_width);

                                Util::extend_with_fitted_str(&mut buffer, value, content_width);

                                printer.print((offset_x, offset_y), &buffer);
                            },
                        }

                        offset_x += content_width;
                    }
                }
            })
            .with_required_size(|shared_model, _constraints| {
                let mut model = shared_model.lock().unwrap();
                model.recache();

                model.required_size(COLUMN_SEP.width())
            })
        ;

        let mut scroll_view = ScrollView::new(canvas).scroll_x(true).scroll_y(true);

        // Set the scrollbar padding to be 0 on both axes.
        let scroller = scroll_view.get_scroller_mut();
        scroller.set_scrollbar_padding((0, 0));

        Self {
            shared_model,
            scroll_view,
        }
    }

    pub fn from_data(data: Data) -> Self {
        Self::new(Model::with_data(data))
    }
}

impl View for TagRecordView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let content_viewport = self.scroll_view.content_viewport();

        // This sub block is needed to avoid a deadlock.
        {
            let model = self.shared_model.lock().unwrap();

            let (header, header_bar) = model.headers();

            // Draw the header and the header bar at the top vertical positions,
            // but all the way to the left, so they scroll with the content.
            let left_offset_printer = printer.content_offset((content_viewport.left(), 0));

            left_offset_printer.print((0, 0), header);
            left_offset_printer.print((0, 1), header_bar);
        }

        // Draw the `ScrollView` starting two columns down.
        self.scroll_view.draw(&printer.offset((0, 2)));
    }

    fn layout(&mut self, final_size: XY<usize>) {
        {
            let mut model = self.shared_model.lock().unwrap();
            model.recache();
        }

        let final_inner_size = final_size.saturating_sub((0, 2));
        self.scroll_view.layout(final_inner_size);
    }

    fn required_size(&mut self, hinted_size: XY<usize>) -> XY<usize> {
        let header_required_extra = XY::new(0, 2);
        let inner_hinted_size = hinted_size.saturating_sub(header_required_extra);
        self.scroll_view.required_size(inner_hinted_size) + header_required_extra
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // Forward the event to the inner `ScrollView`.
        self.scroll_view.on_event(event)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.scroll_view.take_focus(source)
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

    // PASS
    // Expected: V-scrollbar absent.
    // Produced: V-scrollbar absent.
    // let num_records = 63;

    // FAIL
    // Expected: V-scrollbar present.
    // Produced: V-scrollbar absent.
    // let num_records = 64;

    // PASS
    // Expected: V-scrollbar present.
    // Produced: V-scrollbar present.
    // let num_records = 65;

    let num_records = 100;

    let mut rng = rand::thread_rng();

    let records =
        (1..=num_records)
        .map(|i| {
            hashmap! {
                str!("index") => str!(i),
                str!("name") => names.choose(&mut rng).unwrap().to_string(),
                str!("age") => str!((18..=70).choose(&mut rng).unwrap()),
                str!("fave_food") => fave_foods.choose(&mut rng).unwrap().to_string(),
                str!("score") => str!((0..=100).choose(&mut rng).unwrap()),
                str!("is_outgoing") => str!(rand::random::<bool>()),
            }
        })
        .collect::<Vec<_>>()
    ;

    let columns = indexmap! {
        str!("index") => ColumnDef {
            title: str!("Index"),
            sizing: Sizing::Auto,
        },
        str!("name") => ColumnDef {
            title: str!("日本人の氏名"),
            sizing: Sizing::Fixed(6),
        },
        str!("age") => ColumnDef {
            title: str!("Age"),
            sizing: Sizing::Fixed(90),
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            sizing: Sizing::Fixed(90),
        },
        str!("score") => ColumnDef {
            title: str!("Score"),
            sizing: Sizing::Auto,
        },
        str!("is_outgoing") => ColumnDef {
            title: str!("Is Outgoing?"),
            sizing: Sizing::Fixed(90),
        },
    };

    let data = Data::with_data(columns, records);

    let model = Model::with_data(data);

    let main_view = TagRecordView::new(model);

    let mut siv = Cursive::default();

    siv.add_fullscreen_layer(
        Dialog::around(
            main_view
            // .fixed_size((60, 80))
        )
    );

    siv.run();
}
