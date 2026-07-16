use super::Name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VictoryTypeInfo {
    pub name: String,
    #[serde(default)]
    pub victory_screen_header: Option<String>,
    pub milestones: Vec<String>,
    #[serde(default)]
    pub required_spaceship_parts: Option<Vec<String>>,
    pub victory_string: String,
    pub defeat_string: String,
    #[serde(default)]
    pub hidden_in_victory_screen: bool,
}

impl Name for VictoryTypeInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
