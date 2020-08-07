
use std::collections::HashMap;
use std::io::Error as IoError;
use std::path::Path;

use cursive::Printer;
use globset::Glob;
use metaflac::Tag;
use metaflac::Block;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::data::Column;
use crate::data::Record;
use crate::data::Records;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStatus {
    Untrimmed,
    Trimmed(usize, bool),
}

impl TrimStatus {
    pub fn is_trimmed(&self) -> bool {
        matches!(self, Self::Trimmed(..))
    }

    pub fn padding(&self) -> usize {
        match self {
            Self::Untrimmed => 0,
            Self::Trimmed(padding, _) => *padding,
        }
    }

    pub fn emit_ellipsis(&self) -> bool {
        match self {
            Self::Untrimmed => false,
            Self::Trimmed(_, emit_ellipsis) => *emit_ellipsis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimOutput<'a> {
    pub display_str: &'a str,
    pub output_width: usize,
    pub full_real_width: usize,
    pub trim_status: TrimStatus,
}

impl<'a> TrimOutput<'a> {
    pub fn ellipsis_offset(&self) -> usize {
        self.output_width + self.trim_status.padding()
    }
}

/// Alternates between yielding strings from a slice and a separator string.
#[derive(Debug, Clone, Copy)]
struct Interpolator<'a> {
    values: &'a [&'a str],
    separator: &'a str,
    index: usize,
    emit_sep: bool,
}

impl<'a> Interpolator<'a> {
    pub fn new(values: &'a [&'a str], separator: &'a str) -> Self {
        Self {
            values,
            separator,
            index: 0,
            emit_sep: false,
        }
    }
}

impl<'a> Iterator for Interpolator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = if self.emit_sep {
            // Only emit separator if there is another value after it.
            self.values.get(self.index)?;
            self.separator
        }
        else {
            let s = self.values.get(self.index)?;
            self.index += 1;
            s
        };

        self.emit_sep = !self.emit_sep;

        Some(ret)
    }
}

enum State<'a> {
    Head {
        figment_iter: Interpolator<'a>,
        target_width: usize,
        uncontested_width: usize,
    },
    Tail(Interpolator<'a>),
    Ellipsis(usize),
    Done,
}

enum FigOrWidth<'a> {
    Figment(&'a str),
    Width(usize),
}

struct MultiFigments<'a> {
    offset: usize,
    ellipsis: &'a str,
    ellipsis_width: usize,
    state: State<'a>,
}

impl<'a> MultiFigments<'a> {
    pub fn new(values: &'a [&'a str], target_width: usize, separator: &'a str, ellipsis: &'a str) -> Self {
        // If the ellipsis is too wide for the target width, do not try and print it.
        let ellipsis_width =
            match ellipsis.width() {
                x if x <= target_width => { x },
                _ => 0,
            }
        ;

        // This is the width that is always used by text. It will never
        // be possible to draw the ellipsis, if there is one, in this region.
        // The uncontested width will always be no larger than the target width.
        let uncontested_width = target_width.saturating_sub(ellipsis_width);

        // let figment_iter = Interpolator::new(values, FIELD_SEP_STR);
        let figment_iter = Interpolator::new(values, separator);

        Self {
            offset: 0,
            ellipsis,
            ellipsis_width,
            state: State::Head {
                figment_iter,
                target_width,
                uncontested_width,
            },
        }
    }
}

impl<'a> Iterator for MultiFigments<'a> {
    type Item = (usize, &'a str);

    fn next (&mut self) -> Option<Self::Item> {
        match self.state {
            State::Head { ref mut figment_iter, target_width, uncontested_width } => {
                // Get the next figment from the iterator.
                let figment = figment_iter.next()?;

                // Check if there is any uncontested width remaining.
                if let Some(rem_uc_width) = uncontested_width.checked_sub(self.offset) {
                    // Try doing a non-elided trim with the remaining
                    // uncontested width, in order to see if the current figment
                    // can fit in the remaining uncontested width.
                    let trim_output = Util::trim_display_str(figment, rem_uc_width);

                    if trim_output.trim_status.is_trimmed() {
                        // Test to see if this and the remaining figments can
                        // all fit in the total width.
                        let figment_width = trim_output.full_real_width;

                        let backup_iter = figment_iter.clone();

                        let mut frontier_offset = self.offset;
                        let frontier_iter =
                            std::iter::once(FigOrWidth::Width(figment_width))
                            .chain(figment_iter.map(FigOrWidth::Figment))
                        ;

                        for frontier_fow in frontier_iter {
                            let w = match frontier_fow {
                                FigOrWidth::Figment(f) => f.width(),
                                FigOrWidth::Width(w) => w,
                            };

                            frontier_offset += w;

                            if frontier_offset > target_width {
                                // Expected width overflows target width, emit the trimmed boundary.
                                let ret = Some((self.offset, trim_output.display_str));

                                // The offset increases by the trimmed length of the boundary figment.
                                self.offset += trim_output.output_width;

                                // Transition to padding/ellipsis emission.
                                self.state = State::Ellipsis(trim_output.trim_status.padding());

                                return ret;
                            }
                        }

                        // At this point, the current and frontier figments all fit within the
                        // target width. Emit the untrimmed current figment, and transition to tail
                        // emission. Note that the figment iterator will be in the correct position
                        // for the rest of the figments after this one that is being emitted.
                        let ret = Some((self.offset, figment));

                        // The offset increases by the original length of the boundary figment.
                        self.offset += figment_width;

                        // Transition to tail emission.
                        // TODO: May be better to cache calculated widths and iterate that instead?
                        self.state = State::Tail(backup_iter);

                        ret
                    }
                    else {
                        // No trimming occured, just emit the string and offset.
                        let ret = Some((self.offset, figment));

                        // Update the current taken width with the full real width of
                        // the figment.
                        self.offset += trim_output.full_real_width;

                        ret
                    }
                }
                else {
                    // TODO: What to do in this case?
                    unreachable!("");
                }
            },

            // Just iterate over the tail until empty, keeping count of the offsets.
            State::Tail(ref mut tail_figment_iter) => {
                let figment = tail_figment_iter.next()?;
                let width = figment.width();

                let ret = Some((self.offset, figment));

                self.offset += width;

                ret
            },

            State::Ellipsis(ref mut padding) => {
                let (s, offset_delta) =
                    if *padding > 0 {
                        *padding -= 1;
                        (" ", 1)
                    }
                    else {
                        self.state = State::Done;
                        (self.ellipsis, self.ellipsis_width)
                    }
                ;
                // Emit the trimmed boundary, and then advance to next state.
                let ret = Some((self.offset, s));

                self.offset += offset_delta;

                ret
            },
            State::Done => None,
        }
    }
}

pub struct Util;

impl Util {
    pub fn trim_display_str<'a>(original_str: &'a str, target_width: usize) -> TrimOutput<'a> {
        Self::trim_display_str_elided(original_str, target_width, 0)
    }

    pub fn trim_display_str_elided<'a>(
        original_str: &'a str,
        target_width: usize,
        ellipsis_width: usize,
    ) -> TrimOutput<'a>
    {
        let mut curr_width = 0;

        // If the ellipsis is too wide for this target width, set it to 0.
        // This effectively disables the ellipsis and just truncates the string,
        // but it is better than failing or displaying nothing.
        // This also elegantly handles the case of the target width being 0.
        let ellipsis_width =
            if target_width < ellipsis_width { 0 }
            else { ellipsis_width }
        ;

        // This is the index into the string byte array of where the elision
        // cutoff should happen.
        let mut elided_i = 0;
        let mut past_elision_point = false;

        // This is the width of the actual trimmed display string,
        // without the ellipsis. Including here to save cycles later on.
        let mut output_width = 0;

        // Padding is used for when the trim cutoff point occurs in the middle
        // of a multiwidth character. The character cut in the middle will be
        // trimmed, and padding will be calculated to fit the remining width.
        // This is not used if the string does not need trimming/eliding.
        let mut padding = 0;

        // If the original string proves to too wide to fit in the target width,
        // this will be the width the original string will be trimmed to.
        let elided_width = target_width.saturating_sub(ellipsis_width);

        for (i, ch) in original_str.char_indices() {
            let last_width = curr_width;

            curr_width += ch.width().unwrap_or(0);

            if !past_elision_point && curr_width > elided_width {
                past_elision_point = true;
                elided_i = i;
                padding = elided_width - last_width;
                output_width = last_width;
            }

            // Stop once the current width strictly exceeds the target width.
            if curr_width > target_width {
                // If the ellipsis width is 0, either because it was not desired
                // or was too big to fit in the target width, do not print it.
                let print_ellipsis = ellipsis_width != 0;

                // assert_eq!(output_width, &original_str[..elided_i].width());

                // Saving cycles later on by calculating the width of the original
                // string, as if it were untrimmed.
                let full_real_width = original_str[elided_i..].width().saturating_add(output_width);

                return TrimOutput {
                    display_str: &original_str[..elided_i],
                    output_width,
                    full_real_width,
                    trim_status: TrimStatus::Trimmed(padding, print_ellipsis),
                };
            }
        }

        // In this case, the output width and the full real width are the current width.
        let output_width = curr_width;
        let full_real_width = curr_width;

        // The string does not need trimming, just return unchanged.
        TrimOutput {
            display_str: original_str,
            output_width,
            full_real_width,
            trim_status: TrimStatus::Untrimmed,
        }
    }

    pub fn max_column_content_width(column: &Column, records: &Records) -> usize {
        let mut max_seen = column.title.width();
        let column_key = &column.key;

        for record in records.iter() {
            let curr_row_width = record.get(column_key).map(|s| s.width()).unwrap_or(0);
            max_seen = max_seen.max(curr_row_width);
        }

        max_seen
    }

    pub fn read_records_from_dir(working_dir: &Path) -> Result<Records, IoError> {
        let glob = Glob::new("*.flac").unwrap().compile_matcher();
        let mut records = Records::new();

        for entry in std::fs::read_dir(&working_dir)? {
            let path = entry?.path();

            if glob.is_match(&path) {
                let mut metadata = HashMap::new();

                let tag = Tag::read_from_path(&path).unwrap();

                for block in tag.blocks() {
                    if let Block::VorbisComment(vc_map) = block {
                        for (key, values) in vc_map.comments.iter() {
                            let combined_value = values.join("|");
                            metadata.insert(key.to_string(), combined_value);
                        }
                    }
                }

                let record = Record::new(metadata, path);

                records.push(record);
            }
        }

        Ok(records)
    }

    fn raw_draw(printer: &Printer, values: &[&str], target_width: usize) {
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::consts::ELLIPSIS_STR;
    use crate::consts::FIELD_SEP_STR;

    #[test]
    fn trim_display_str_elided() {
        assert_eq!(
            Util::trim_display_str_elided("hello!", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 3, 1),
            TrimOutput {
                display_str: "he",
                output_width: 2,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, true)
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 1),
            TrimOutput {
                display_str: "hell",
                output_width: 4,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 100),
            TrimOutput {
                display_str: "hello",
                output_width: 5,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 6, 100),
            TrimOutput {
                display_str: "hello!",
                output_width: 6,
                full_real_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 4, 1),
            TrimOutput {
                display_str: "oh ",
                output_width: 3,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 1),
            TrimOutput {
                display_str: "oh y̆",
                output_width: 4,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 100),
            TrimOutput {
                display_str: "oh y̆e",
                output_width: 5,
                full_real_width: 6,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 6, 100),
            TrimOutput {
                display_str: "oh y̆es",
                output_width: 6,
                full_real_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 1, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 2, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 3, 1),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 1),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 2),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                full_real_width: 12,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
    }

    #[test]
    fn interpolator() {
        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 0,
            emit_sep: false,
        };
        assert_eq!(i.collect::<Vec<_>>(), vec!["HELLO", "////", "WORLD"]);

        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 0,
            emit_sep: true,
        };
        assert_eq!(i.collect::<Vec<_>>(), vec!["////","HELLO", "////", "WORLD"]);

        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 1,
            emit_sep: false,
        };
        assert_eq!(i.collect::<Vec<_>>(), vec!["WORLD"]);

        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 1,
            emit_sep: true,
        };
        assert_eq!(i.collect::<Vec<_>>(), vec!["////", "WORLD"]);

        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 2,
            emit_sep: false,
        };
        assert_eq!(i.collect::<Vec<&str>>(), Vec::<&str>::new());

        let i = Interpolator {
            values: &["HELLO", "WORLD"],
            separator: "////",
            index: 2,
            emit_sep: true,
        };
        assert_eq!(i.collect::<Vec<&str>>(), Vec::<&str>::new());

        let i = Interpolator {
            values: &["WOW", "COOL", "RAD", "NEAT", "AYY"],
            separator: "|",
            index: 0,
            emit_sep: false,
        };
        assert_eq!(i.collect::<Vec<_>>(), vec!["WOW", "|", "COOL", "|", "RAD", "|", "NEAT", "|", "AYY"]);
    }

    #[test]
    fn multi_figments() {
        let mf = MultiFigments::new(&["WOW", "COOL", "RAD", "NEAT", "AYY"], 21, FIELD_SEP_STR, ELLIPSIS_STR);
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "WOW"),
                (3, FIELD_SEP_STR),
                (4, "COOL"),
                (8, FIELD_SEP_STR),
                (9, "RAD"),
                (12, FIELD_SEP_STR),
                (13, "NEAT"),
                (17, FIELD_SEP_STR),
                (18, "AYY"),
            ],
        );

        let mf = MultiFigments::new(&["WOW", "COOL", "RAD", "NEAT", "AYY"], 50, FIELD_SEP_STR, ELLIPSIS_STR);
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "WOW"),
                (3, FIELD_SEP_STR),
                (4, "COOL"),
                (8, FIELD_SEP_STR),
                (9, "RAD"),
                (12, FIELD_SEP_STR),
                (13, "NEAT"),
                (17, FIELD_SEP_STR),
                (18, "AYY"),
            ],
        );

        let mf = MultiFigments::new(&["WOW", "COOL", "RAD", "NEAT", "AYY"], 20, FIELD_SEP_STR, ELLIPSIS_STR);
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "WOW"),
                (3, FIELD_SEP_STR),
                (4, "COOL"),
                (8, FIELD_SEP_STR),
                (9, "RAD"),
                (12, FIELD_SEP_STR),
                (13, "NEAT"),
                (17, FIELD_SEP_STR),
                (18, "A"),
                (19, ELLIPSIS_STR),
            ],
        );

        let mf = MultiFigments::new(&["0123456789", "0123456789"], 20, "abcdefghijklmnopqrstuvwxyz", ELLIPSIS_STR);
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "0123456789"),
                (10, "abcdefghi"),
                (19, ELLIPSIS_STR),
            ],
        );

        let mf = MultiFigments::new(&["0123456789", "0123456789"], 21, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "0123456789"),
                (10, "|"),
                (11, "0123456789"),
            ],
        );

        let mf = MultiFigments::new(&["0123456789", "0123456789"], 20, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "0123456789"),
                (10, "|"),
                (11, "012345"),
                (17, "..."),
            ],
        );

        let mf = MultiFigments::new(&["0123456789", "0123456789"], 14, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "0123456789"),
                (10, "|"),
                (11, ""),
                (11, "..."),
            ],
        );

        let mf = MultiFigments::new(&["0123456789"], 10, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, "0123456789"),
            ],
        );

        let mf = MultiFigments::new(&[""], 10, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![
                (0, ""),
            ],
        );

        let mf = MultiFigments::new(&[], 10, "|", "...");
        assert_eq!(
            mf.collect::<Vec<_>>(),
            vec![],
        );
    }
}
