use serde::{Deserialize, Serialize};

use crate::tile_map::RegionType;

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nation {
    pub name: String,
    #[serde(default)]
    pub leader_name: String,
    #[serde(default)]
    pub adjective: Vec<String>,
    // These fields below are relevant to Civilization starting position.
    #[serde(default)]
    pub along_ocean: bool,
    #[serde(default)]
    /// This field is only used in CityState `Venice`, we don't tackle it because I don't know what it means.
    pub place_first_along_ocean: bool,
    #[serde(default)]
    /// Now this field is not used in the game, so we don't tackle it.
    pub along_river: bool,
    #[serde(default)]
    pub region_type_priority: Vec<RegionType>,
    #[serde(default)]
    pub avoid_region_type: Vec<RegionType>,
    #[serde(default)]
    pub preferred_victory_type: String,
    #[serde(default)]
    pub start_intro_part1: String,
    #[serde(default)]
    pub start_intro_part2: String,
    #[serde(default)]
    pub declaring_war: String,
    #[serde(default)]
    pub attacked: String,
    #[serde(default)]
    pub defeated: String,
    #[serde(default)]
    pub introduction: String,
    #[serde(default)]
    pub neutral_hello: String,
    #[serde(default)]
    pub hate_hello: String,
    #[serde(default)]
    pub trade_request: String,
    pub outer_color: Option<[u8; 3]>,
    pub inner_color: Option<[u8; 3]>,
    #[serde(default)]
    pub favored_religion: String,
    #[serde(default)]
    pub unique_name: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub cities: Vec<String>,
    #[serde(default)]
    pub city_state_type: String,
}

impl Name for Nation {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
