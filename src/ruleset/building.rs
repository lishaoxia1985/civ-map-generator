use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ruleset::Yields,
    tile_component::{BaseTerrain, Feature, TerrainType},
};

use super::Name;
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub name: String,
    #[serde(default)]
    pub is_national_wonder: bool,
    #[serde(default)]
    pub is_wonder: bool,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub great_person_points: HashMap<String, i8>,
    #[serde(default)]
    pub specialist_slots: HashMap<String, i8>,
    #[serde(default)]
    pub hurry_cost_modifier: i8,
    #[serde(default)]
    pub required_terrain: RequiredTerrain,
    #[serde(default)]
    #[serde(flatten)]
    pub required_building: Option<RequiredBuilding>,
    #[serde(default)]
    pub required_tech: String,
    #[serde(default)]
    pub required_nearby_improved_resources: Vec<String>,
    #[serde(default)]
    pub maintenance: i8,
    #[serde(default)]
    pub replaces: String,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub city_strength: i8,
    #[serde(default)]
    pub city_health: i8,
    #[serde(default)]
    pub cost: i16,
    #[serde(default)]
    pub percent_stat_bonus: HashMap<String, i8>,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub quote: String,
}

impl Name for Building {
    fn name(&self) -> String {
        self.name.to_owned()
    }
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
            freshwater: None,
            extra_conditions: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequiredBuilding {
    /// Required building in a city.
    RequiredBuildingInCity(String),
    /// Required building in all cities(except puppeted city).
    RequiredBuildingInAllCities(String),
}
