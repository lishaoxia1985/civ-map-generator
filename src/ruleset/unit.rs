use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub name: String,
    pub unit_type: String,
    pub movement: i8,
    #[serde(default)]
    pub strength: i16,
    #[serde(default)]
    pub cost: i16,
    #[serde(default)]
    pub required_tech: String,
    #[serde(default)]
    pub obsolete_tech: String,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub replaces: String,
    #[serde(default)]
    pub upgrades_to: String,
    #[serde(default)]
    pub hurry_cost_modifier: i8,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
    #[serde(default)]
    pub promotions: Vec<String>,
    #[serde(default)]
    pub attack_sound: String,
}

impl Name for Unit {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
