use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuinInfo {
    name: String,
    notification: String,
    uniques: Vec<String>,
    #[serde(default)]
    color: String,
    #[serde(default)]
    excluded_difficulties: Vec<String>,
}
