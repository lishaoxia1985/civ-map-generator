// Auto-generated file. Do not edit manually.
// Re-exports all generated enum types

pub mod base_terrain;
pub use base_terrain::BaseTerrain;
pub mod belief;
pub use belief::Belief;
pub mod building;
pub use building::Building;
pub mod city_state_type;
pub use city_state_type::CityStateType;
pub mod difficulty;
pub use difficulty::Difficulty;
pub mod era;
pub use era::Era;
pub mod feature;
pub use feature::Feature;
pub mod nation;
pub use nation::Nation;
pub mod natural_wonder;
pub use natural_wonder::NaturalWonder;
pub mod quest;
pub use quest::Quest;
pub mod resource;
pub use resource::Resource;
pub mod specialist;
pub use specialist::Specialist;
pub mod terrain_type;
pub use terrain_type::TerrainType;
pub mod tile_improvement;
pub use tile_improvement::TileImprovement;
pub mod unit;
pub use unit::Unit;
pub mod unit_promotion;
pub use unit_promotion::UnitPromotion;
pub mod unit_type;
pub use unit_type::UnitType;

/// Trait for infallible conversion between enum variants and string representations
/// **PANICS** if string does not match any variant
pub trait EnumStr {
    /// Converts enum variant to its canonical string representation
    fn as_str(&self) -> &'static str;

    /// Converts string to enum variant **PANICS on invalid input**
    ///
    /// # Panics
    /// Panics if `s` does not match any variant's string representation
    fn from_str(s: &str) -> Self;
}
