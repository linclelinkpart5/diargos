
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::model::Columns;
use crate::model::Records;

#[derive(Copy, Clone)]
pub enum TrimResult {
    Untrimmed,
    Trimmed(usize),
}

impl TrimResult {
    pub fn padding(&self) -> usize {
        match self {
            Self::Untrimmed => 0,
            Self::Trimmed(padding) => *padding,
        }
    }

    pub fn was_trimmed(&self) -> bool {
        match self {
            Self::Untrimmed => false,
            Self::Trimmed(..) => true,
        }
    }
}

pub struct Util;

impl Util {
    pub fn new_trim_display(original_str: &str, target_width: usize) -> (&str, TrimResult) {
        let mut curr_width = 0;

        for (i, ch) in original_str.char_indices() {
            let last_width = curr_width;

            curr_width += ch.width_cjk().unwrap_or(0);

            if curr_width > target_width {
                return
                    // Tried to trim off part of a multiwidth character.
                    // This means that the target width lies in between a character.
                    // Remove the whole character and flag as such, noting how
                    // much leftover padding is needed.
                    if last_width < target_width {
                        (&original_str[..i], TrimResult::Trimmed(target_width - last_width))
                    }
                    // Trim the string as normal.
                    else {
                        (&original_str[..i], TrimResult::Trimmed(0))
                    }
                ;
            }
        }

        // The string does not need trimming, just return unchanged.
        (original_str, TrimResult::Untrimmed)
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

    pub fn trim_display_str(original_str: &str, content_width: usize, ellipsis_width: usize) -> (&str, bool) {
        // If there is not enough room to even print an ellipsis, just return.
        if content_width < ellipsis_width {
            return ("", original_str != "")
        }

        let trunc_width = content_width.saturating_sub(ellipsis_width);

        let mut char_indices = original_str.char_indices().peekable();

        for _ in 0..trunc_width { char_indices.next(); }

        // Skip the number of characters needed to show a truncated view.
        match char_indices.peek() {
            // The number of characters in the string is less than or equal to
            // the truncated column width. Just show it as-is, with no ellipsis.
            None => (&original_str[..], false),

            // The number of characters in the string is greater than the
            // truncated column width. Check to see how that number compares to
            // the non-truncated column width.
            Some(&(trunc_pos, _)) => {
                // Skip the number of characters in the ellipsis.
                for _ in 0..ellipsis_width { char_indices.next(); }

                match char_indices.peek() {
                    // The string will fit in the full column width.
                    // Just show as-is, with no ellipsis.
                    None => (&original_str[..], false),

                    // There are characters left that will not fit in the column.
                    // Return a slice of the string, with enough room left over
                    // to include an ellipsis.
                    Some(..) => (&original_str[..trunc_pos], true),
                }
            },
        }
    }

    pub fn skip_first_n_width(string: &str, n: usize) -> (&str, bool) {
        let mut last_width = 0;
        let mut last_i = 0;
        for (i, _) in string.char_indices() {
            let prefix = &string[..i];
            // println!("{:?}, {}, {}", prefix, i, prefix.width_cjk());
            let curr_width = prefix.width_cjk();
            if curr_width > n {
                return if last_width < n {
                    // Double-width character that was split across the cut boundary.
                    // Chop off the entire character, and flag that a double-width
                    // character was cut "in the middle".
                    (&string[i..], true)
                } else {
                    (&string[last_i..], false)
                }
            }

            last_i = i;
            last_width = curr_width;
        }

        ("", false)
    }

    pub fn take_first_n_width(string: &str, n: usize) -> (&str, bool) {
        let mut last_width = 0;
        let mut last_i = 0;
        for (i, _) in string.char_indices() {
            let prefix = &string[..i];
            let curr_width = prefix.width_cjk();
            if curr_width > n {
                return if last_width < n {
                    // Double-width character that was split across the cut boundary.
                    // Chop off the entire character, and flag that a double-width
                    // character was cut "in the middle".
                    (&string[..last_i], true)
                } else {
                    (&string[..last_i], false)
                }
            }

            last_i = i;
            last_width = curr_width;
        }

        (string, false)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn skip_first_n_width() {
        assert_eq!(
            Util::skip_first_n_width("hello!", 3),
            ("lo!", false),
        );
        assert_eq!(
            Util::skip_first_n_width("hello!", 0),
            ("hello!", false),
        );
        assert_eq!(
            Util::skip_first_n_width("hello!", 6),
            ("", false),
        );
        assert_eq!(
            Util::skip_first_n_width("oh y̆es", 0),
            ("oh y̆es", false),
        );
        assert_eq!(
            Util::skip_first_n_width("oh y̆es", 3),
            ("y̆es", false),
        );
        assert_eq!(
            Util::skip_first_n_width("oh y̆es", 4),
            ("es", false),
        );
        assert_eq!(
            Util::skip_first_n_width("oh y̆es", 6),
            ("", false),
        );
        assert_eq!(
            Util::skip_first_n_width("日本人の氏名", 0),
            ("日本人の氏名", false),
        );
        assert_eq!(
            Util::skip_first_n_width("日本人の氏名", 1),
            ("本人の氏名", true),
        );
        assert_eq!(
            Util::skip_first_n_width("日本人の氏名", 2),
            ("本人の氏名", false),
        );
    }

    #[test]
    fn take_first_n_width() {
        assert_eq!(
            Util::take_first_n_width("hello!", 3),
            ("hel", false),
        );
        assert_eq!(
            Util::take_first_n_width("hello!", 0),
            ("", false),
        );
        assert_eq!(
            Util::take_first_n_width("hello!", 6),
            ("hello!", false),
        );
        assert_eq!(
            Util::take_first_n_width("oh y̆es", 0),
            ("", false),
        );
        assert_eq!(
            Util::take_first_n_width("oh y̆es", 3),
            ("oh ", false),
        );
        assert_eq!(
            Util::take_first_n_width("oh y̆es", 4),
            ("oh y̆", false),
        );
        assert_eq!(
            Util::take_first_n_width("oh y̆es", 6),
            ("oh y̆es", false),
        );
        assert_eq!(
            Util::take_first_n_width("日本人の氏名", 0),
            ("", false),
        );
        assert_eq!(
            Util::take_first_n_width("日本人の氏名", 1),
            ("", true),
        );
        assert_eq!(
            Util::take_first_n_width("日本人の氏名", 2),
            ("日", false),
        );
    }
}
