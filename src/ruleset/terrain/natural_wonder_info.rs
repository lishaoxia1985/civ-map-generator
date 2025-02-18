use serde::{Deserialize, Serialize};

use crate::component::{base_terrain::BaseTerrain, terrain_type::TerrainType};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NaturalWonderInfo {
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
    pub turns_into_type: Option<TerrainType>,
    #[serde(default)]
    pub turns_into_base: Option<BaseTerrain>,
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
    pub occurs_on_type: Vec<TerrainType>,
    #[serde(default)]
    pub occurs_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub occurs_on_feature: Vec<String>,
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
