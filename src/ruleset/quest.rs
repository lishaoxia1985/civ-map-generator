use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestInfo {
    name: String,
    description: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    influence: i32,
    #[serde(default)]
    minimum_civs: i32,
    #[serde(default)]
    duration: i32,
}
