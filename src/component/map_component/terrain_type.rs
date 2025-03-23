use enum_map::Enum;
use serde::{Deserialize, Serialize};

#[derive(Enum, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Debug)]
pub enum TerrainType {
    Water,
    Flatland,
    Mountain,
    Hill,
}

impl TerrainType {
    pub fn name(&self) -> &str {
        match self {
            TerrainType::Water => "Water",
            TerrainType::Flatland => "Flatland",
            TerrainType::Mountain => "Mountain",
            TerrainType::Hill => "Hill",
        }
    }
}
