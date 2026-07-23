use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitInfo {
    pub name: String,
    pub unit_type: String,
    pub movement: i32,
    #[serde(default)]
    pub strength: i32,
    #[serde(default)]
    pub cost: i32,
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
    pub hurry_cost_modifier: i32,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
    #[serde(default)]
    pub promotions: Vec<String>,
    #[serde(default)]
    pub attack_sound: String,
}
