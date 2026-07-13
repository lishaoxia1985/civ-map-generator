use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitPromotionInfo {
    name: String,
    #[serde(default)]
    prerequisites: Vec<String>,
    uniques: Vec<String>,
    #[serde(default)]
    unit_types: Vec<String>,
}

impl Name for UnitPromotionInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
