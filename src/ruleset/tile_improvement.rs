use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tile_component::{BaseTerrain, Feature, TerrainType};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileImprovement {
    name: String,
    #[serde(default)]
    food: i8,
    #[serde(default)]
    production: i8,
    #[serde(default)]
    science: i8,
    #[serde(default)]
    gold: i8,
    #[serde(default)]
    culture: i8,
    #[serde(default)]
    faith: i8,
    #[serde(default)]
    happiness: i8,
    #[serde(default)]
    can_be_built_on_type: Vec<TerrainType>,
    #[serde(default)]
    can_be_built_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    can_be_built_on_feature: Vec<Feature>,
    #[serde(default)]
    turns_to_build: i8,
    #[serde(default)]
    tech_required: String,
    #[serde(default)]
    unique_to: String,
    uniques: Vec<String>,
    shortcut_key: Option<char>,
    civilopedia_text: Option<Vec<HashMap<String, String>>>,
}

impl Name for TileImprovement {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
