
use serde::Deserialize;

use crate::data::Columns;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub columns: Columns,
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
