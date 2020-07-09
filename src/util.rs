
use std::iter::Peekable;

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::consts::*;
use crate::model::Columns;
use crate::model::Records;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStatus {
    Untrimmed,
    Trimmed(usize, bool),
}

impl TrimStatus {
    pub fn was_trimmed(&self) -> bool {
        match self {
            Self::Untrimmed => false,
            Self::Trimmed(..) => true,
        }
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
    pub display_string: &'a str,
    pub output_width: usize,
    pub trim_status: TrimStatus,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum TrimOutput<'a> {
//     Untrimmed(&'a str, usize),
//     Trimmed(&'a str, usize, bool, usize),
// }

// impl<'a> TrimOutput<'a> {
//     pub fn value(&self) -> &str {
//         match self {
//             Self::Untrimmed(v, _) => v,
//             Self::Trimmed(v, _, _, _) => v,
//         }
//     }

//     pub fn was_trimmed(&self) -> bool {
//         match self {
//             Self::Untrimmed(..) => false,
//             Self::Trimmed(..) => true,
//         }
//     }

//     pub fn padding(&self) -> usize {
//         match self {
//             Self::Untrimmed(..) => 0,
//             Self::Trimmed(_, padding, _, _) => *padding,
//         }
//     }

//     pub fn emit_ellipsis(&self) -> bool {
//         match self {
//             Self::Untrimmed(..) => false,
//             Self::Trimmed(_, _, emit_ellipsis, _) => *emit_ellipsis,
//         }
//     }

//     pub fn string_width(&self) -> usize {
//         match self {
//             Self::Untrimmed(_, w) => *w,
//             Self::Trimmed(_, _, _, w) => *w,
//         }
//     }
// }

pub struct Util;

impl Util {
    pub fn trim_display_str(original_str: &str, target_width: usize) -> (&str, usize, bool) {
        let mut curr_width = 0;

        for (i, ch) in original_str.char_indices() {
            let last_width = curr_width;

            curr_width += ch.width_cjk().unwrap_or(0);

            // Stop once the current width strictly exceeds the target width.
            if curr_width > target_width {
                // If this is non-zero, it means that the target width ends in
                // the middle of a multiwidth character.
                // This character will end up getting omitted from the final
                // trimmed string.
                let padding = target_width - last_width;

                return (&original_str[..i], padding, true);
            }
        }

        // The string does not need trimming, just return unchanged.
        (original_str, 0, false)
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

            curr_width += ch.width_cjk().unwrap_or(0);

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
            Some(column_def) => column_def.title.width_cjk(),
            None => { return 0; },
        };

        for record in records.iter() {
            let curr_row_width = record.get(column_key).map(|s| s.width_cjk()).unwrap_or(0);
            max_seen = max_seen.max(curr_row_width);
        }

        max_seen
    }

    pub fn extend_with_fitted_str(buffer: &mut String, original_str: &str, content_width: usize) {
        let original_width = original_str.width_cjk();

        let (display_str, padding, add_ellipsis) =
            if original_width > content_width {
                let trimmed_width = content_width.saturating_sub(ELLIPSIS_STR_WIDTH);
                let (trimmed_str, internal_padding, was_trimmed) =
                    Util::trim_display_str(original_str, trimmed_width)
                ;

                (trimmed_str, internal_padding, was_trimmed)
            } else {
                (original_str, content_width - original_width, false)
            }
        ;

        buffer.push_str(display_str);

        // Add padding and ellipsis, if needed.
        for _ in 0..padding {
            buffer.push(' ');
        }
        if add_ellipsis {
            buffer.push_str(ELLIPSIS_STR);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintAtomsOutput<'a> {
    Text(&'a str),
    MissingSentinel(usize),
}

#[derive(Debug, Clone, Copy)]
enum PrintAtomsState {
    Value,
    Ellipsis,
    Delimiter,
}

pub struct PrintAtoms<'a, S, W>
where
    S: Iterator<Item = Option<&'a str>>,
    W: Iterator<Item = usize>,
{
    strings: Peekable<S>,
    widths: Peekable<W>,
    curr_offset: usize,
    state: PrintAtomsState,
}

impl<'a, S, W> PrintAtoms<'a, S, W>
where
    S: Iterator<Item = Option<&'a str>>,
    W: Iterator<Item = usize>,
{
    pub fn new(strings: S, widths: W) -> Self {
        Self {
            strings: strings.peekable(),
            widths: widths.peekable(),
            curr_offset: 0,
            state: PrintAtomsState::Value,
        }
    }
}

impl<'a, S, W> Iterator for PrintAtoms<'a, S, W>
where
    S: Iterator<Item = Option<&'a str>>,
    W: Iterator<Item = usize>,
{
    type Item = (PrintAtomsOutput<'a>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        // This is to avoid yielding separators/ellipses if there are no more
        // strings/widths to process.
        self.strings.peek()?;
        self.widths.peek()?;

        match self.state {
            PrintAtomsState::Value => {
                let opt_original_str = self.strings.next()?;
                let target_width = self.widths.next()?;
                let original_str = match opt_original_str {
                    Some(original_str) => original_str,

                    // A missing value, signal that a sentinel value needs to be
                    // printed.
                    None => {
                        let ret = Some((PrintAtomsOutput::MissingSentinel(target_width), self.curr_offset));
                        self.curr_offset += target_width;
                        return ret;
                    },
                };

                let trim_output =
                    Util::trim_display_str_elided(
                        original_str,
                        target_width,
                        ELLIPSIS_STR_WIDTH,
                    )
                ;

                let ret = Some((PrintAtomsOutput::Text(trim_output.display_string), self.curr_offset));

                if trim_output.trim_status.emit_ellipsis() {
                    self.state = PrintAtomsState::Ellipsis;
                } else {
                    self.state = PrintAtomsState::Delimiter;
                }

                if trim_output.trim_status.was_trimmed() {
                    self.curr_offset += trim_output.output_width;
                } else {
                    self.curr_offset += target_width;
                }

                ret
            },
            PrintAtomsState::Ellipsis => {
                let ret = Some((PrintAtomsOutput::Text(ELLIPSIS_STR), self.curr_offset));

                self.state = PrintAtomsState::Delimiter;
                self.curr_offset += ELLIPSIS_STR_WIDTH;

                ret
            },
            PrintAtomsState::Delimiter => {
                let ret = Some((PrintAtomsOutput::Text(COLUMN_SEP), self.curr_offset));

                self.state = PrintAtomsState::Value;
                self.curr_offset += COLUMN_SEP_WIDTH;

                ret
            },
        }
    }
}

impl<'a, S, W> std::iter::FusedIterator for PrintAtoms<'a, S, W>
where
    S: Iterator<Item = Option<&'a str>>,
    W: Iterator<Item = usize>,
{}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn trim_display_str() {
        assert_eq!(
            Util::trim_display_str("hello!", 3),
            ("hel", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("hello!", 0),
            ("", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("hello!", 6),
            ("hello!", 0, false),
        );
        assert_eq!(
            Util::trim_display_str("oh y̆es", 0),
            ("", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("oh y̆es", 3),
            ("oh ", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("oh y̆es", 4),
            ("oh y̆", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("oh y̆es", 6),
            ("oh y̆es", 0, false),
        );
        assert_eq!(
            Util::trim_display_str("日本人の氏名", 0),
            ("", 0, true),
        );
        assert_eq!(
            Util::trim_display_str("日本人の氏名", 1),
            ("", 1, true),
        );
        assert_eq!(
            Util::trim_display_str("日本人の氏名", 2),
            ("日", 0, true),
        );
    }

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

    #[test]
    fn print_atoms() {
        let strings = &[Some("wow"), Some("tubular"), Some("日本人の氏名"), Some("neat"), None];
        let widths = &[5, 5, 5, 5, 5];

        let produced =
            PrintAtoms::new(
                strings.iter().copied(),
                widths.iter().copied(),
            )
            .collect::<Vec<_>>()
        ;
        let expected = vec![
            (PrintAtomsOutput::Text("wow"), 0),
            (PrintAtomsOutput::Text(" │ "), 5),
            (PrintAtomsOutput::Text("tubu"), 8),
            (PrintAtomsOutput::Text("⋯"), 12),
            (PrintAtomsOutput::Text(" │ "), 13),
            (PrintAtomsOutput::Text("日本"), 16),
            (PrintAtomsOutput::Text("⋯"), 20),
            (PrintAtomsOutput::Text(" │ "), 21),
            (PrintAtomsOutput::Text("neat"), 24),
            (PrintAtomsOutput::Text(" │ "), 29),
            (PrintAtomsOutput::MissingSentinel(5), 32),
        ];

        assert_eq!(produced, expected);
    }
}
