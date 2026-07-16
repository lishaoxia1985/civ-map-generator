use super::{
    Name,
    common::{RequiredTerrain, Yields},
    enums::*,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NaturalWonderInfo {
    pub name: String,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnsIntoTerrain {
    pub terrain_type: TerrainType,
    #[serde(default)]
    pub base_terrain: Option<BaseTerrain>,
}
