
use serde::Deserialize;

use crate::data::Columns;

#[derive(Deserialize)]
pub struct Config {
    columns: Columns,
}
