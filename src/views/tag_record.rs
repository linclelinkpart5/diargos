
use std::sync::Arc;
use std::sync::Mutex;

use cursive::Printer;
use cursive::XY;
use cursive::Rect;
use cursive::direction::Direction;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::event::Key;
use cursive::theme::ColorStyle;
use cursive::view::View;
use cursive::view::scroll::Scroller;
use cursive::views::Canvas;
use cursive::views::ScrollView;
use unicode_width::UnicodeWidthStr;

use crate::consts::*;
use crate::data::ColumnKey;
// use crate::data::Data;
use crate::model::Model;
use crate::util::Util;
use crate::util::MultiFigments;

enum Atom<'a> {
    Single(&'a str, bool),
    Multi(&'a [String], bool),
    Missing(bool),
    Header,
}

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
                let data = &model.data;

                for (offset_y, record) in data.records.iter().enumerate() {
                    let atoms_and_widths =
                        data.columns.iter()
                        .enumerate()
                        .map(|(x, col)| {
                            let y = offset_y;
                            let highlighted = model.is_cursor_at_cell(x, y);

                            match &col.key {
                                ColumnKey::Meta(meta_key) => {
                                    match record.get_meta(meta_key) {
                                        None => Atom::Missing(highlighted),
                                        Some(vals) => Atom::Multi(vals, highlighted),
                                    }
                                },
                                ColumnKey::Info(info_key) => {
                                    match record.get_info(info_key) {
                                        None => Atom::Missing(highlighted),
                                        Some(val) => Atom::Single(val, highlighted),
                                    }
                                },
                            }
                        })
                        .zip(model.iter_cached_widths())
                    ;

                    Self::draw_delimited_row(printer, offset_y, COLUMN_SEP, atoms_and_widths);
                }
            })
            .with_required_size(|shared_model, _constraints| {
                let mut model = shared_model.lock().unwrap();
                model.recache();

                model.required_size(COLUMN_SEP.width())
            })
            .with_important_area(|shared_model, _final_size| {
                let model = shared_model.lock().unwrap();

                // Figure out the logical X and Y coordinates of the highlighted cell, if any.
                let (lx, ly) = match model.cursor.to_xy() {
                    // Return a view showing the entire visible canvas.
                    (lx, None) => (lx, 0),
                    (lx, Some(ly)) => (lx, ly),
                };

                let tx = model.column_offset(lx, COLUMN_SEP.width()).unwrap_or(0);
                let ty = ly;

                let dx = model.cached_content_widths.get(lx).copied().unwrap_or(0);
                let dy = 1;

                Rect::from_size((tx, ty), (dx, dy))
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

    // pub fn from_data(data: Data) -> Self {
    //     Self::new(Model::with_data(data))
    // }

    fn draw_delimited_row<'a>(
        printer: &Printer,
        offset_y: usize,
        separator: &str,
        atoms_and_widths: impl Iterator<Item = (Atom<'a>, usize)>,
    )
    {
        let mut offset_x = 0;
        let mut is_first_col = true;

        for (atom, content_width) in atoms_and_widths {
            if is_first_col { is_first_col = false; }
            else {
                printer.print((offset_x, offset_y), separator);
                offset_x += separator.width();
            }

            match atom {
                Atom::Missing(highlighted) => {
                    // Print out a highlighted sentinel, to indicate a missing value.
                    let color =
                        if highlighted { ColorStyle::highlight() }
                        else { ColorStyle::secondary() }
                    ;

                    printer.with_color(
                        color,
                        |pr| {
                            pr.print_hline(
                                (offset_x, offset_y),
                                content_width,
                                MISSING_FILL,
                            );
                        },
                    );

                },
                Atom::Header => {
                    printer.print_hline(
                        (offset_x, offset_y),
                        content_width,
                        COLUMN_HEADER_BAR,
                    );
                },
                Atom::Single(value, highlighted) => {
                    let color =
                        if highlighted { ColorStyle::highlight() }
                        else { ColorStyle::primary() }
                    ;

                    let trim_output = Util::trim_display_str_elided(
                        value,
                        content_width,
                        ELLIPSIS_STR.width(),
                    );

                    let display_str = trim_output.display_str;
                    let emit_ellipsis = trim_output.trim_status.emit_ellipsis();

                    printer.with_color(
                        color,
                        move |pr| {
                            pr.print((offset_x, offset_y), &display_str);

                            if emit_ellipsis {
                                let ellipsis_offset = trim_output.ellipsis_offset();

                                pr.print((offset_x + ellipsis_offset, offset_y), ELLIPSIS_STR);
                            }
                        },
                    );
                },
                Atom::Multi(values, highlighted) => {
                    let color =
                        if highlighted { ColorStyle::highlight() }
                        else { ColorStyle::primary() }
                    ;

                    // let trim_output = Util::trim_display_str_elided(
                    //     original_string,
                    //     content_width,
                    //     ELLIPSIS_STR.width(),
                    // );

                    let multi_figments = MultiFigments::new(values, content_width, FIELD_SEP_STR, ELLIPSIS_STR);

                    // let display_str = trim_output.display_str;
                    // let emit_ellipsis = trim_output.trim_status.emit_ellipsis();

                    for (offset, figment, figment_kind) in multi_figments {
                        let used_color =
                            if figment_kind.is_sep() { ColorStyle::title_primary() }
                            else { color }
                        ;

                        printer.with_color(
                            used_color,
                            move |pr| {
                                pr.print((offset_x + offset, offset_y), &figment);
                            },
                        );
                    }

                    // printer.with_color(
                    //     color,
                    //     move |pr| {
                    //         for (offset, figment, figment_kind) in multi_figments {
                    //             pr.print((offset_x + offset, offset_y), &figment);
                    //         }
                    //         // pr.print((offset_x, offset_y), &display_str);

                    //         // if emit_ellipsis {
                    //         //     let ellipsis_offset = trim_output.ellipsis_offset();

                    //         //     pr.print((offset_x + ellipsis_offset, offset_y), ELLIPSIS_STR);
                    //         // }
                    //     },
                    // );
                },
            };

            offset_x += content_width;
        }
    }
}

impl View for TagRecordView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let content_viewport = self.scroll_view.content_viewport();

        // This sub block is needed to avoid a deadlock.
        {
            let model = self.shared_model.lock().unwrap();
            let data = &model.data;

            // Draw the header and the header bar at the top vertical positions,
            // but all the way to the left, so they scroll with the content.
            let left_offset_printer = printer.content_offset((content_viewport.left(), 0));

            let atoms_and_widths =
                data.columns.iter()
                .enumerate()
                .map(|(x, col)| {
                    let highlighted = model.is_cursor_at_column(x);
                    Atom::Single(&col.title, highlighted)
                })
                .zip(model.iter_cached_widths())
            ;

            Self::draw_delimited_row(&left_offset_printer, 0, COLUMN_SEP, atoms_and_widths);

            let atoms_and_widths = model.iter_cached_widths().map(|w| (Atom::Header, w));

            Self::draw_delimited_row(&left_offset_printer, 1, COLUMN_HEADER_SEP, atoms_and_widths);
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
        {
            let mut model = self.shared_model.lock().unwrap();
            // let old_cursor = model.cursor;

            match event {
                Event::AltChar('d') => {
                    if let Some(col_idx) = model.cursor.column_index() {
                        model.sort_by_column_index(col_idx, true)
                    }
                },
                Event::AltChar('a') => {
                    if let Some(col_idx) = model.cursor.column_index() {
                        model.sort_by_column_index(col_idx, false)
                    }
                },
                Event::Key(Key::Up) => {
                    model.move_cursor_up(1);
                },
                Event::Key(Key::Down) => {
                    model.move_cursor_down(1);
                },
                Event::Key(Key::Left) => {
                    model.move_cursor_left(1);
                },
                Event::Key(Key::Right) => {
                    model.move_cursor_right(1);
                },
                Event::Key(Key::PageUp) => {
                    model.move_cursor_up(10);
                },
                Event::Key(Key::PageDown) => {
                    model.move_cursor_down(10);
                },
                _ => return EventResult::Ignored,
            };
        }

        self.scroll_view.scroll_to_important_area();

        EventResult::Consumed(None)

        // self.scroll_view.on_event(event)
    }

    fn take_focus(&mut self, source: Direction) -> bool {
        self.scroll_view.take_focus(source)
    }
}
