use serde::{Deserialize, Serialize};

use crate::tile_component::{BaseTerrain, Feature, TerrainType};

pub trait Name {
    fn name(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Yields {
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredTerrain {
    #[serde(default = "default_terrain_type")]
    pub terrain_type: Vec<TerrainType>,
    #[serde(default = "default_base_terrain")]
    pub base_terrain: Vec<BaseTerrain>,
    /// When it's `None`, it means the required terrain will ignore this value,
    /// which means it can be any feature or no feature.
    pub feature: Option<Vec<Feature>>,
    /// When it's `None`, it means the required terrain will ignore this value,
    /// which means it has a river or not.
    river: Option<bool>,
    /// When it's `None`, it means the required terrain will ignore this value,
    /// which means the required terrain can be freshwater or not.
    #[serde(default)]
    pub freshwater: Option<bool>,
    #[serde(default)]
    pub extra_conditions: Vec<String>,
}

impl Default for RequiredTerrain {
    fn default() -> Self {
        Self {
            terrain_type: vec![TerrainType::Flatland, TerrainType::Hill],
            base_terrain: vec![
                BaseTerrain::Grassland,
                BaseTerrain::Plain,
                BaseTerrain::Desert,
                BaseTerrain::Tundra,
                BaseTerrain::Snow,
            ],
            feature: None,
            river: None,
            freshwater: None,
            extra_conditions: Vec::new(),
        }
    }
}

fn default_terrain_type() -> Vec<TerrainType> {
    vec![TerrainType::Flatland, TerrainType::Hill]
}

fn default_base_terrain() -> Vec<BaseTerrain> {
    vec![
        BaseTerrain::Grassland,
        BaseTerrain::Plain,
        BaseTerrain::Desert,
        BaseTerrain::Tundra,
        BaseTerrain::Snow,
    ]
}
