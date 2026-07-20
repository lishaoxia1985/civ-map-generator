use serde::{Deserialize, Serialize};

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
