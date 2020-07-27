
use std::io::Error as IoError;
use std::path::Path;

use globset::Glob;
use metaflac::Tag;
use metaflac::Block;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::data::Columns;
use crate::data::Record;
use crate::data::Records;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStatus {
    Untrimmed,
    Trimmed(usize, bool),
}

impl TrimStatus {
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
    pub display_string: &'a str,
    pub output_width: usize,
    pub trim_status: TrimStatus,
}

impl<'a> TrimOutput<'a> {
    pub fn ellipsis_offset(&self) -> usize {
        self.output_width + self.trim_status.padding()
    }
}

pub struct Util;

impl Util {
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

                // This is the width of the actual trimmed string, without the
                // ellipsis. Calculating here to save cycles later on.
                let output_width = elided_width.saturating_sub(padding);

                // At this point, the elided width should be used.
                return TrimOutput {
                    display_string: &original_str[..elided_i],
                    output_width,
                    trim_status: TrimStatus::Trimmed(padding, print_ellipsis),
                };
            }
        }

        // In this case, the output width is the current width.
        let output_width = curr_width;

        // The string does not need trimming, just return unchanged.
        TrimOutput {
            display_string: original_str,
            output_width,
            trim_status: TrimStatus::Untrimmed,
        }
    }

    pub fn max_column_content_width(column_key: &str, columns: &Columns, records: &Records) -> usize {
        let mut max_seen = match columns.get(column_key) {
            Some(column_def) => column_def.title.width(),
            None => { return 0; },
        };

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
                let mut record = Record::new();
                let tag = Tag::read_from_path(&path).unwrap();

                for block in tag.blocks() {
                    if let Block::VorbisComment(vc_map) = block {
                        for (key, values) in vc_map.comments.iter() {
                            let combined_value = values.join("|");
                            record.insert(key.to_string(), combined_value);
                        }
                    }
                }

                // let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                // record.insert(str!("FILENAME"), file_name);

                records.push(record);
            }
        }

        Ok(records)
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
                display_string: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 3, 1),
            TrimOutput {
                display_string: "he",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true)
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 1),
            TrimOutput {
                display_string: "hell",
                output_width: 4,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 5, 100),
            TrimOutput {
                display_string: "hello",
                output_width: 5,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("hello!", 6, 100),
            TrimOutput {
                display_string: "hello!",
                output_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 0, 1),
            TrimOutput {
                display_string: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 4, 1),
            TrimOutput {
                display_string: "oh ",
                output_width: 3,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 1),
            TrimOutput {
                display_string: "oh y̆",
                output_width: 4,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 5, 100),
            TrimOutput {
                display_string: "oh y̆e",
                output_width: 5,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("oh y̆es", 6, 100),
            TrimOutput {
                display_string: "oh y̆es",
                output_width: 6,
                trim_status: TrimStatus::Untrimmed,
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 0, 1),
            TrimOutput {
                display_string: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, false),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 1, 1),
            TrimOutput {
                display_string: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 2, 1),
            TrimOutput {
                display_string: "",
                output_width: 0,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 3, 1),
            TrimOutput {
                display_string: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 1),
            TrimOutput {
                display_string: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(1, true),
            },
        );
        assert_eq!(
            Util::trim_display_str_elided("日本人の氏名", 4, 2),
            TrimOutput {
                display_string: "日",
                output_width: 2,
                trim_status: TrimStatus::Trimmed(0, true),
            },
        );
    }
}
