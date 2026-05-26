use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ruleset::Yields,
    tile_component::{BaseTerrain, Feature, TerrainType},
};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileImprovement {
    pub name: String,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub required_terrain: Vec<RequiredTerrain>,
    #[serde(default)]
    pub required_tech: String,
    #[serde(default)]
    pub turns_to_build: i8,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub shortcut_key: Option<char>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
}

impl Name for TileImprovement {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredTerrain {
    pub terrain_type: Vec<TerrainType>,
    pub base_terrain: Vec<BaseTerrain>,
    /// When it's `None`, it means the required terrain will ignore this value,
    /// which means it can be any feature or no feature.
    pub feature: Option<Vec<Feature>>,
    /// When it's `None`, it means the required terrain will ignore this value,
    /// which means the required terrain can be freshwater or not.
    #[serde(default)]
    pub freshwater: Option<bool>,
    #[serde(default)]
    pub extra_conditions: Vec<String>,
}
