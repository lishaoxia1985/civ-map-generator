use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitType {
    name: String,
    movement_type: String,
    #[serde(default)]
    uniques: Vec<String>,
}

impl Name for UnitType {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
