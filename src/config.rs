
use indexmap::indexmap;
use serde::Deserialize;
use str_macro::str;

use crate::data::Columns;
use crate::data::ColumnDef;
use crate::data::Sizing;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub columns: Columns,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            columns: indexmap! {
                str!("ARTIST") => ColumnDef {
                    title: str!("Artist"),
                    sizing: Sizing::Auto,
                },
                str!("TITLE") => ColumnDef {
                    title: str!("Title"),
                    sizing: Sizing::Auto,
                },
                str!("ALBUM") => ColumnDef {
                    title: str!("Album"),
                    sizing: Sizing::Auto,
                },
                str!("FILENAME") => ColumnDef {
                    title: str!("File Name"),
                    sizing: Sizing::Auto,
                },
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize() {
        let input = r#"{
            "columns": {
                "ARTIST": {
                    "title": "Artist",
                    "sizing": null
                },
                "TITLE": {
                    "title": "Title",
                    "sizing": null
                },
                "FILENAME": {
                    "title": "File Name",
                    "sizing": null
                }
            }
        }"#;

        let config = serde_json::from_str::<Config>(&input).unwrap();
        println!("{:?}", config);
    }
}
