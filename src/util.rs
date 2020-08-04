
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

type SavePoint<'a> = (usize, TrimOutput<'a>, Interpolator<'a>);

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
        let uc_width = target_width.saturating_sub(ellipsis_width);

        let mut used_width = 0;
        let mut save_point = None;
        let figment_iter = Interpolator::new(values, FIELD_SEP_STR);

        for figment in figment_iter {
            // Some uncontested width remaining.
            if let Some(rem_uc_width) = uc_width.checked_sub(used_width) {
                // Try doing a non-elided trim with the remaining
                // uncontested width, in order to see if the current figment
                // can fit in the remaining uncontested width.
                let trim_output = Self::trim_display_str(figment, rem_uc_width);

                let trim_status = &trim_output.trim_status;

                if save_point.is_none() && trim_status.is_trimmed() {
                    // The remaining width was not enough to fully print this
                    // figment. Save the current offset, trim result, and
                    // iterator state.
                    save_point = Some((used_width, trim_output, figment_iter.clone()));
                }
                else {
                    // No trimming occured, just print the string.
                    printer.print((used_width, 0), figment);
                }

                // In either case, update the current taken width with the full
                // real width of the figment.
                used_width = used_width.saturating_add(trim_output.full_real_width);
            }

            // See if the current taken width now exceeds the target width.
            if used_width > target_width {
                // The attempted string overflowed the target width.
                // Just print out the save point, padding, and ellipsis, and
                // then return.
                let (mut offset_x, trim_output, _) = save_point.unwrap();

                // Print the last trimmed string.
                printer.print((offset_x, 0), trim_output.display_str);

                // Increment the offset and draw the ellipsis, if available.
                offset_x = offset_x.saturating_add(trim_output.ellipsis_offset());

                if trim_output.trim_status.emit_ellipsis() {
                    printer.print((offset_x, 0), ELLIPSIS_STR);
                }

                return;
            }
        }

        // At this point, the entire delimited string fits in the target width.
        // If no save point has been registered, that means the entire string
        // has already been printed out, so just return. Else, use the saved
        // figment iterator and starting offset to print out the remaining
        // unprinted figments.
        if let Some((mut offset_x, _, backup_iter)) = save_point {
            for figment in backup_iter {
                printer.print((offset_x, 0), figment);
                offset_x = offset_x.saturating_add(figment.width());
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
}
