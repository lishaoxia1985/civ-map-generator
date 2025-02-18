use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
pub struct Belief {
    name: String,
    r#type: String,
    uniques: Vec<String>,
}

impl Name for Belief {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
