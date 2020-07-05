
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::consts::*;
use crate::model::Columns;
use crate::model::Records;

pub struct Util;

impl Util {
    pub fn trim_display_str(original_str: &str, target_width: usize) -> (&str, usize, bool) {
        let mut curr_width = 0;

        for (i, ch) in original_str.char_indices() {
            let last_width = curr_width;

            curr_width += ch.width_cjk().unwrap_or(0);

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
}
