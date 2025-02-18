#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::component::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileResource {
    pub name: String,
    pub resource_type: String,
    #[serde(default)]
    pub can_be_found_on_type: Vec<TerrainType>,
    #[serde(default)]
    pub can_be_found_on_base: Vec<BaseTerrain>,
    #[serde(default)]
    pub can_be_found_on_feature: Vec<Feature>,
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
    pub improvement: String,
    #[serde(default)]
    pub revealed_by: String,
    pub improvement_stats: Option<HashMap<String, i8>>,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub major_deposit_amount: Option<HashMap<String, i8>>,
    pub minor_deposit_amount: Option<HashMap<String, i8>>,
}

impl Name for TileResource {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
