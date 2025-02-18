#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::Name;
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub name: String,
    #[serde(default)]
    pub is_national_wonder: bool,
    #[serde(default)]
    pub is_wonder: bool,
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
    pub great_person_points: Option<HashMap<String, i8>>,
    pub specialist_slots: Option<HashMap<String, i8>>,
    #[serde(default)]
    pub hurry_cost_modifier: i8,
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
    pub required_building: String,
    #[serde(default)]
    pub required_tech: String,
    pub percent_stat_bonus: Option<HashMap<String, i8>>,
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
