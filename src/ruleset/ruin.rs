use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ruin {
    name: String,
    notification: String,
    uniques: Vec<String>,
    #[serde(default)]
    color: String,
    #[serde(default)]
    excluded_difficulties: Vec<String>,
}

impl Name for Ruin {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
