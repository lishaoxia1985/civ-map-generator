use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tile_component::{BaseTerrain, Feature, TerrainType};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileImprovement {
    pub name: String,
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
    pub can_be_built_on_type: Vec<TerrainType>,
    #[serde(default)]
    pub can_be_built_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub can_be_built_on_feature: Vec<Feature>,
    #[serde(default)]
    pub turns_to_build: i8,
    #[serde(default)]
    pub required_tech: String,
    #[serde(default)]
    pub unique_to: String,
    pub uniques: Vec<String>,
    pub shortcut_key: Option<char>,
    pub civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

impl Name for TileImprovement {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
