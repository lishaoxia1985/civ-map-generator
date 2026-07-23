use super::{
    common::{RequiredTerrain, Yields},
    enums::Specialist,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInfo {
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
    pub specialist_slots: HashMap<Specialist, i8>,
    #[serde(default)]
    pub hurry_cost_modifier: i32,
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
    pub maintenance: i32,
    #[serde(default)]
    pub replaces: String,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub city_strength: i32,
    #[serde(default)]
    pub city_health: i32,
    #[serde(default)]
    pub cost: i32,
    #[serde(default)]
    pub percent_stat_bonus: HashMap<String, i8>,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub quote: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequiredBuilding {
    /// Required building in a city.
    RequiredBuildingInCity(String),
    /// Required building in all cities(except puppeted city).
    RequiredBuildingInAllCities(String),
}
