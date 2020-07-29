
use serde::Deserialize;
use str_macro::str;

use crate::data::Column;
use crate::data::Columns;
use crate::data::ColumnKey;
use crate::data::Sizing;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub columns: Columns,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            columns: vec![
                Column {
                    key: ColumnKey::Meta(str!("ARTIST")),
                    title: str!("Artist"),
                    sizing: Sizing::Auto,
                },
                Column {
                    key: ColumnKey::Meta(str!("TITLE")),
                    title: str!("Title"),
                    sizing: Sizing::Auto,
                },
                Column {
                    key: ColumnKey::Meta(str!("ALBUM")),
                    title: str!("Album"),
                    sizing: Sizing::Auto,
                },
                Column {
                    key: ColumnKey::Meta(str!("FILENAME")),
                    title: str!("File Name"),
                    sizing: Sizing::Auto,
                },
            ],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize() {
        let input = r#"{
            "columns": [
                {
                    "meta": "ARTIST",
                    "title": "Artist",
                    "sizing": null
                },
                {
                    "meta": "TITLE",
                    "title": "Title",
                    "sizing": null
                },
                {
                    "meta": "FILENAME",
                    "title": "File Name",
                    "sizing": null
                }
            ]
        }"#;

        let config = serde_json::from_str::<Config>(&input).unwrap();
        println!("{:?}", config);
    }
}
