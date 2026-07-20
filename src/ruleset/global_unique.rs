use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalUnique {
    name: String,
    uniques: Vec<String>,
}
