
use std::collections::HashMap;
use std::io::Error as IoError;
use std::path::Path;

use cursive::Printer;
use globset::Glob;
use metaflac::Tag;
use metaflac::Block;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::consts::ELLIPSIS_STR;
use crate::consts::FIELD_SEP_STR;
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
    pub trim_status: TrimStatus,
}

impl<'a> TrimOutput<'a> {
    pub fn ellipsis_offset(&self) -> usize {
        self.output_width + self.trim_status.padding()
    }
}

#[derive(Debug, Clone, Copy)]
enum StopPointKind { Val, Sep }

#[derive(Debug, Clone, Copy)]
struct StopPoint {
    kind: StopPointKind,
    index: usize,
    ch_pos: usize,
}

struct Stub<'a> {
    trimmed_str: &'a str,
    trimmed_width: usize,
}

/// Alternates between yielding strings from a slice and a separator string.
#[derive(Debug, Clone, Copy)]
struct Interpolator<'a> {
    values: &'a [&'a str],
    index: usize,
    emit_sep: bool,
}

impl<'a> Interpolator<'a> {
    pub fn new(values: &'a [&'a str]) -> Self {
        Self {
            values,
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
            FIELD_SEP_STR
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

// #[derive(Debug, Clone, Copy)]
// pub enum Figment<'a> {
//     Value(&'a str),
//     Separator,
// }

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
            }

            // Stop once the current width strictly exceeds the target width.
            if curr_width > target_width {
                // If the ellipsis width is 0, either because it was not desired
                // or was too big to fit in the target width, do not print it.
                let print_ellipsis = ellipsis_width != 0;

                // This is the width of the actual trimmed display string,
                // without the ellipsis. Including here to save cycles later on.
                let output_width = elided_width.saturating_sub(padding);
                // assert_eq!(output_width, &original_str[..elided_i].width());

                // At this point, the elided width should be used.
                return TrimOutput {
                    display_str: &original_str[..elided_i],
                    output_width,
                    trim_status: TrimStatus::Trimmed(padding, print_ellipsis),
                };
            }
        }

        // In this case, the output width is the current width.
        let output_width = curr_width;

        // The string does not need trimming, just return unchanged.
        TrimOutput {
            display_str: original_str,
            output_width,
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

    fn raw_draw(
        printer: &Printer,
        values: &[&str],
        target_width: usize,
    )
    {
        struct SavePoint<'a> {
            offset_x: usize,
            trim_output: TrimOutput<'a>,
        }

        // If the ellipsis is too wide for the target width, do not try and print it.
        let ellipsis_width =
            match ELLIPSIS_STR.width() {
                x if x <= target_width => { x },
                _ => 0,
            }
        ;

        // This is the width that is always used by text. It will never
        // be possible to draw the ellipsis, if there is one, in this region.
        // The uncontested width will always be no larger than the target width.
        let uncontested_width = target_width.saturating_sub(ellipsis_width);

        let mut curr_taken_width = 0;
        let mut save_point = SavePoint {
            offset_x: 0,
            trim_output: TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Untrimmed,
            },
        };

        let master_iter = Interpolator::new(values);
        let mut backup_iter = master_iter.clone();

        for figment in master_iter {
            // See if the current figment fits in the remaining uncontested width.
            match uncontested_width.checked_sub(curr_taken_width) {
                // Some uncontested width remaining, see if it is enough to
                // use right now.
                Some(rem_uc_width) => {
                    // Try doing a non-elided trim with the remaining
                    // uncontested width.
                    let trim_output = Self::trim_display_str(FIELD_SEP_STR, rem_uc_width);

                    let trim_status = &trim_output.trim_status;

                    if trim_status.is_trimmed() {
                        // The remaining width was not enough to fully print this
                        // figment. Save the current offset and the trim result.
                        // Also, stop advancing the backup iterator.
                        save_point = SavePoint {
                            offset_x: curr_taken_width,
                            trim_output,
                        };
                    }
                    else {
                        // No trimming occured, print the string and advance the
                        // backup iterator.
                        printer.print((curr_taken_width, 0), figment);
                        backup_iter.next();
                    }

                    // In either case, update the current taken width.
                    curr_taken_width += (trim_output.output_width + trim_status.padding());
                },

                // Already past the point of uncontested width.
                // TODO: HAVE THIS BE A SEPARATE IF BRANCH AFTER THE LAST ONE!
                None => {},
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trim_display_str_elided() {
        assert_eq!(
            Util::trim_display_str_elided("hello!", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 3, 1),
            TrimOutput {
                display_str: "he",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true)
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 1),
            TrimOutput {
                display_str: "hell",
                output_width: 4,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 100),
            TrimOutput {
                display_str: "hello",
                output_width: 5,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 6, 100),
            TrimOutput {
                display_str: "hello!",
                output_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 4, 1),
            TrimOutput {
                display_str: "oh ",
                output_width: 3,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 1),
            TrimOutput {
                display_str: "oh y̆",
                output_width: 4,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 100),
            TrimOutput {
                display_str: "oh y̆e",
                output_width: 5,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 6, 100),
            TrimOutput {
                display_str: "oh y̆es",
                output_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 0, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 1, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 2, 1),
            TrimOutput {
                display_str: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 3, 1),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 1),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 2),
            TrimOutput {
                display_str: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
    }
}
