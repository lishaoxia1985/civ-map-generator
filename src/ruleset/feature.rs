use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tile_component::{BaseTerrain, TerrainType};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureInfo {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub food: i8,
    #[serde(default)]
    pub production: i8,
    #[serde(default)]
    pub science: i8,
    #[serde(default)]
    pub gold: i8,
    #[serde(default)]
    pub culture: i8,
    #[serde(default)]
    pub faith: i8,
    #[serde(default)]
    pub happiness: i8,
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
    pub occurs_on_type: Vec<TerrainType>,
    #[serde(default)]
    pub occurs_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

impl Name for FeatureInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl FeatureInfo {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}
