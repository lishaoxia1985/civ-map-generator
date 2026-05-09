use serde::{Deserialize, Serialize};

use crate::tile_map::RegionType;

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NationInfo {
    pub name: String,
    #[serde(default)]
    pub leader_name: String,
    #[serde(default)]
    pub adjective: Vec<String>,
    #[serde(default)]
    pub start_bias: Option<StartBias>,
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
    #[serde(default)]
    pub outer_color: [u8; 3],
    #[serde(default)]
    pub inner_color: [u8; 3],
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

impl Name for NationInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StartBias {
    AlongOcean,
    AlongRiver,
    RegionTypePriority(Vec<RegionType>),
    RegionTypeAvoid(Vec<RegionType>),
}
