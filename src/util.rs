
use crate::model::Columns;
use crate::model::Records;

pub struct Util;

impl Util {
    pub fn str_width(s: &str) -> usize {
        s.char_indices().count()
    }

    pub fn max_column_content_width(column_key: &str, columns: &Columns, records: &Records) -> usize {
        let mut max_seen = match columns.get(column_key) {
            Some(column_def) => Self::str_width(&column_def.title),
            None => { return 0; },
        };

        for record in records.iter() {
            let curr_row_width = record.get(column_key).map(|s| Self::str_width(s)).unwrap_or(0);
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

        let mut char_indices = original_str.char_indices();

        // Skip the number of characters needed to show a truncated view.
        match char_indices.by_ref().skip(trunc_width).peekable().peek() {
            // The number of characters in the string is less than or equal to
            // the truncated column width. Just show it as-is, with no ellipsis.
            None => (&original_str[..], false),

            // The number of characters in the string is greater than the
            // truncated column width. Check to see how that number compares to
            // the non-truncated column width.
            Some(&(trunc_pos, _)) => {
                // Skip the number of characters in the ellipsis.
                match char_indices.by_ref().skip(ellipsis_width).next() {
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
}
