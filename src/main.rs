
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
use cursive::view::View;
use cursive::view::scroll::Core as ScrollCore;
use cursive::view::scroll::Scroller;
use cursive::views::Canvas;
use cursive::views::ScrollView;

use crate::consts::*;
use crate::model::ColumnDef;
use crate::model::Model;
use crate::model::Sizing;
use crate::util::Util;

pub struct TagRecordView {
    shared_model: Arc<Mutex<Model>>,
    scroll_view: ScrollView<Canvas<Arc<Mutex<Model>>>>,
}

impl TagRecordView {
    pub fn new(model: Model) -> Self {
        let shared_model = Arc::new(Mutex::new(model));

        let canvas =
            Canvas::new(shared_model.clone())
            .with_draw(|shared_model, printer| {
                let model = shared_model.lock().unwrap();

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

                model.required_size(COLUMN_SEP_WIDTH)
            })
        ;

        let scroll_view = ScrollView::new(canvas).scroll_x(true).scroll_y(true);

        Self {
            shared_model,
            scroll_view,
        }
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

    fn layout(&mut self, size: XY<usize>) {
        {
            let mut model = self.shared_model.lock().unwrap();
            model.recache();
        }

        let inner_size = size.saturating_sub((0, 2));
        self.scroll_view.layout(inner_size);
    }

    fn required_size(&mut self, constraint: XY<usize>) -> XY<usize> {
        let header_required_extra = XY::new(0, 2);
        let inner_constraint = constraint.saturating_sub(header_required_extra);
        self.scroll_view.required_size(inner_constraint) + header_required_extra
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        // Forward the event to the inner `ScrollView`.
        self.scroll_view.on_event(event)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.scroll_view.take_focus(source)
    }
}

// pub struct TagRecordView {
//     shared_model: Arc<Mutex<Model>>,
//     scroll_core: ScrollCore,
// }

// impl TagRecordView {
//     pub fn new(model: Model) -> Self {
//         let shared_model = Arc::new(Mutex::new(model));
//         let scroll_core = ScrollCore::new().scroll_x(true).scroll_y(true);

//         Self {
//             shared_model,
//             scroll_core,
//         }
//     }

//     pub fn draw_records(&self, printer: &Printer) {
//         let model = self.shared_model.lock().unwrap();

//         for (offset_y, record) in model.records.iter().enumerate() {
//             let mut offset_x = 0;
//             let mut is_first_col = true;

//             for (column_key, content_width) in model.columns.keys().zip(model.iter_cache()) {
//                 if is_first_col { is_first_col = false; }
//                 else {
//                     printer.print((offset_x, offset_y), COLUMN_SEP);
//                     offset_x += COLUMN_SEP_WIDTH;
//                 }

//                 match record.get(column_key) {
//                     None => {
//                         // Print out a highlighted sentinel, to indicate a missing value.
//                         printer.with_color(ColorStyle::highlight_inactive(), |pr| {
//                             pr.print_hline((offset_x, offset_y), content_width, MISSING_STR);
//                         });
//                     },
//                     Some(value) => {
//                         // Rough approximation for capacity.
//                         let mut buffer = String::with_capacity(content_width);

//                         Util::extend_with_fitted_str(&mut buffer, value, content_width);

//                         printer.print((offset_x, offset_y), &buffer);
//                     },
//                 }

//                 offset_x += content_width;
//             }
//         }
//     }
// }

// impl Scroller for TagRecordView {
//     fn get_scroller(&self) -> &ScrollCore {
//         &self.scroll_core
//     }

//     fn get_scroller_mut(&mut self) -> &mut ScrollCore {
//         &mut self.scroll_core
//     }
// }

// impl View for TagRecordView {
//     fn draw(&self, printer: &Printer<'_, '_>) {
//         let content_viewport = self.scroll_core.content_viewport();

//         // This sub block is needed to avoid a deadlock.
//         {
//             let model = self.shared_model.lock().unwrap();

//             let (header, header_bar) = model.headers();

//             // Draw the header and the header bar at the top vertical positions,
//             // but all the way to the left, so they scroll with the content.
//             let left_offset_printer = printer.content_offset((content_viewport.left(), 0));

//             left_offset_printer.print((0, 0), header);
//             left_offset_printer.print((0, 1), header_bar);
//         }

//         cursive::view::scroll::draw(
//             self,
//             &printer.offset((0, 2)),
//             |scroller, sub_printer| {
//                 scroller.draw_records(sub_printer);
//             }
//         );
//     }

//     fn layout(&mut self, size: XY<usize>) {
//         cursive::view::scroll::layout(
//             self,
//             size,
//             true,
//             |scroller, _inner_size| {
//                 let mut model = scroller.shared_model.lock().unwrap();
//                 model.recache();
//             },
//             |scroller, constraint| { scroller.required_size(constraint) },
//         );
//     }

//     fn needs_relayout(&self) -> bool {
//         true
//     }

//     fn required_size(&mut self, _constraint: XY<usize>) -> XY<usize> {
//         let mut model = self.shared_model.lock().unwrap();
//         model.recache();

//         let size = model.required_size(COLUMN_SEP_WIDTH);

//         // Add in extra space for the header.
//         let header_extra = XY::new(0, 2);

//         // Add in extra space for scrollbars on each axis, if applicable.
//         let scrollbar_paddings = self.scroll_core.get_scrollbar_padding();
//         let scrollbar_sizes = self.scroll_core.scrollbar_size();

//         println!("{:?} {:?}", scrollbar_sizes, scrollbar_paddings);

//         size + header_extra //).saturating_sub(scrollbar_sizes)
//     }

//     fn on_event(&mut self, event: Event) -> EventResult {
//         cursive::view::scroll::on_event(
//             self,
//             event,
//             |_scroller, _sub_event| EventResult::Ignored,
//             |_scroller, _sub_area| Rect::from_size((0, 0), (1, 1)),
//         )
//     }

//     fn take_focus(&mut self, _source: Direction) -> bool {
//         true
//     }
// }

// pub struct TagEditorView {
//     shared_model: Arc<Mutex<Model>>,
//     linear_layout: LinearLayout,
// }

// impl TagEditorView {
//     pub fn new(model: Model) -> Self {
//         let shared_model = Arc::new(Mutex::new(model));

//         let columns_canvas =
//             Canvas::new(shared_model.clone())
//             .with_layout(|model, _constraints| {
//                 let mut model = model.lock().unwrap();
//                 model.recache();
//             })
//             .with_draw(|model, printer| {
//                 let model = model.lock().unwrap();

//                 let mut offset_x = 0;
//                 let mut is_first_col = true;

//                 for (column_def, content_width) in model.columns.values().zip(model.iter_cache()) {
//                     if is_first_col { is_first_col = false; }
//                     else {
//                         printer.print((offset_x, 0), COLUMN_SEP);
//                         offset_x += COLUMN_SEP_WIDTH;
//                     }

//                     let title = &column_def.title;

//                     let (display_title, was_trimmed) = Util::trim_display_str(
//                         title,
//                         content_width,
//                         ELLIPSIS_STR_WIDTH,
//                     );

//                     if was_trimmed {
//                         printer.print_hline((offset_x, 0), content_width, ELLIPSIS_STR);
//                     }

//                     printer.print((offset_x, 0), display_title);

//                     offset_x += content_width;
//                 }

//                 let mut offset_x = 0;
//                 let mut is_first_col = true;

//                 for content_width in model.iter_cache() {
//                     if is_first_col { is_first_col = false; }
//                     else {
//                         printer.print((offset_x, 1), COLUMN_HEADER_SEP);
//                         offset_x += COLUMN_SEP_WIDTH;
//                     }

//                     printer.print_hline((offset_x, 1), content_width, COLUMN_HEADER_BAR);

//                     offset_x += content_width;
//                 }
//             })
//             .with_required_size(|model, _constraints| {
//                 let mut model = model.lock().unwrap();
//                 model.recache();
//                 let total_width = model.total_display_width(COLUMN_SEP_WIDTH);

//                 (total_width, 2).into()
//             })
//         ;

//         let records_canvas =
//             Canvas::new(shared_model.clone())
//             .with_draw(|model, printer| {
//                 let model = model.lock().unwrap();

//                 for (offset_y, record) in model.records.iter().enumerate() {
//                     let mut offset_x = 0;
//                     let mut is_first_col = true;

//                     for (column_key, content_width) in model.columns.keys().zip(model.iter_cache()) {
//                         if is_first_col { is_first_col = false; }
//                         else {
//                             printer.print((offset_x, offset_y), COLUMN_SEP);
//                             offset_x += COLUMN_SEP_WIDTH;
//                         }

//                         match record.get(column_key) {
//                             None => {
//                                 // Print out a highlighted sentinel, to indicate a missing value.
//                                 printer.with_color(ColorStyle::highlight_inactive(), |pr| {
//                                     pr.print_hline((offset_x, offset_y), content_width, MISSING_STR);
//                                 });
//                             },
//                             Some(value) => {
//                                 let (display_value, was_trimmed) = Util::trim_display_str(
//                                     value,
//                                     content_width,
//                                     ELLIPSIS_STR_WIDTH,
//                                 );

//                                 if was_trimmed {
//                                     printer.print_hline((offset_x, offset_y), content_width, ELLIPSIS_STR);
//                                 }

//                                 printer.print((offset_x, offset_y), display_value);
//                             },
//                         }

//                         offset_x += content_width;
//                     }
//                 }
//             })
//             .with_required_size(|model, _constraints| {
//                 let mut model = model.lock().unwrap();
//                 model.recache();
//                 let total_width = model.total_display_width(COLUMN_SEP_WIDTH);

//                 (total_width, model.records.len()).into()
//             })
//             .scrollable()
//             .scroll_x(false)
//             .scroll_y(true)
//         ;

//         // TODO: See if there is an option to allow the records `Canvas` to have
//         //       its scrollbar always visible.
//         // TODO: Drawing vertically one column at a time might be slow, test out
//         //       horizontal drawing.
//         let linear_layout =
//             LinearLayout::vertical()
//             .child(columns_canvas)
//             .child(records_canvas)
//         ;

//         Self {
//             shared_model,
//             linear_layout,
//         }
//     }

//     pub fn mutate_columns<F, R>(&mut self, func: F) -> R
//     where
//         F: FnOnce(&mut Columns) -> R,
//     {
//         let mut model = self.shared_model.lock().unwrap();
//         model.mutate_columns(func)
//     }

//     pub fn mutate_records<F, R>(&mut self, func: F) -> R
//     where
//         F: FnOnce(&mut Records) -> R,
//     {
//         let mut model = self.shared_model.lock().unwrap();
//         model.mutate_records(func)
//     }

//     pub fn push_record(&mut self, record: Record) {
//         let mut model = self.shared_model.lock().unwrap();
//         model.mutate_records(move |m| m.push(record))
//     }

//     pub fn extend_records(&mut self, records: Records) {
//         let mut model = self.shared_model.lock().unwrap();
//         model.mutate_records(move |m| m.extend(records))
//     }
// }

// impl View for TagEditorView {
//     fn draw(&self, printer: &Printer) {
//         self.linear_layout.draw(printer);
//     }

//     fn layout(&mut self, constraint: XY<usize>) {
//         self.linear_layout.layout(constraint)
//     }

//     fn required_size(&mut self, constraint: XY<usize>) -> XY<usize> {
//         self.linear_layout.required_size(constraint)
//     }

//     fn on_event(&mut self, event: Event) -> EventResult {
//         self.linear_layout.on_event(event)
//     }

//     fn take_focus(&mut self, source: Direction) -> bool {
//         self.linear_layout.take_focus(source)
//     }
// }

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
            sizing: Sizing::Fixed(120),
        },
        str!("fave_food") => ColumnDef {
            title: str!("Favorite Food"),
            sizing: Sizing::Fixed(120),
        },
        str!("score") => ColumnDef {
            title: str!("Score"),
            sizing: Sizing::Auto,
        },
        str!("is_outgoing") => ColumnDef {
            title: str!("Is Outgoing?"),
            sizing: Sizing::Fixed(50),
        },
    };

    let model = Model::with_data(columns, records);

    // let main_view = TagEditorView::new(model);
    let main_view = TagRecordView::new(model);

    let mut siv = Cursive::default();

    siv.add_fullscreen_layer(
        main_view
        // .scrollable()
        // .scroll_x(true)
        // .scroll_y(false)
        // .fixed_size((30, 20))
    );

    siv.run();
}
