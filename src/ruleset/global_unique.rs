use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalUnique {
    name: String,
    uniques: Vec<String>,
}

impl Name for GlobalUnique {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
