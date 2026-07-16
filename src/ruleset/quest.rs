use super::Name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestInfo {
    name: String,
    description: String,
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    influence: i8,
    #[serde(default)]
    minimum_civs: i8,
    #[serde(default)]
    duration: i8,
}

impl Name for QuestInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
