use serde::{Deserialize, Serialize};

use crate::{
    ruleset::Yields,
    tile_component::{BaseTerrain, TerrainType},
};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NaturalWonderInfo {
    pub name: String,
    pub r#type: String,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub impassable: bool,
    #[serde(default)]
    pub unbuildable: bool,
    #[serde(default)]
    pub weight: i8,
    #[serde(default)]
    pub override_stats: bool,
    #[serde(default)]
    pub is_fresh_water: bool,
    #[serde(default)]
    pub required_terrain: RequiredTerrain,
    pub turns_into_terrain: TurnsIntoTerrain,
    #[serde(default)]
    pub uniques: Vec<String>,
}

impl Name for NaturalWonderInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl NaturalWonderInfo {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredTerrain {
    pub terrain_type: Vec<TerrainType>,
    pub base_terrain: Vec<BaseTerrain>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnsIntoTerrain {
    pub terrain_type: TerrainType,
    #[serde(default)]
    pub base_terrain: Option<BaseTerrain>,
}
