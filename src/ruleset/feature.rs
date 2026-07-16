use super::{
    Name,
    common::{RequiredTerrain, Yields},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureInfo {
    pub name: String,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub defence_bonus: f32,
    #[serde(default)]
    pub movement_cost: i8,
    #[serde(default)]
    pub impassable: bool,
    #[serde(default)]
    pub unbuildable: bool,
    #[serde(default)]
    pub override_stats: bool,
    #[serde(default)]
    pub required_terrain: RequiredTerrain,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
}

impl Name for FeatureInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
