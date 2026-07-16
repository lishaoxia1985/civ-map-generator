use super::{Name, enums::VictoryType};
use serde::{Deserialize, Serialize};

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
    pub preferred_victory_type: Option<VictoryType>,
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
    pub nation_type: NationType,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Region type.
///
/// The variant are defined in order of priority.
/// The priority is typically used to sort the regions.
/// The highest priority is [`RegionType::Tundra`] and [`RegionType::Undefined`] is the lowest priority.
///
/// If you add a new region type, [`RegionType::Undefined`] should be always the last variant.
/// In the other words, [`RegionType::Undefined`] is always the lowest priority.
pub enum RegionType {
    Tundra,
    Jungle,
    Forest,
    Desert,
    Hill,
    Plain,
    Grassland,
    Hybrid,
    Undefined,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum NationType {
    #[default]
    Civilization,
    /// The string represents the type of city state, e.g. "Cultured", "Maritime", etc.
    CityState(String),
    Barbarians,
    Spectator,
}
