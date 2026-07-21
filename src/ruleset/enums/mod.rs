// Auto-generated file. Do not edit manually.
// Re-exports all generated enum types

mod base_terrain;
pub use base_terrain::*;
mod belief;
pub use belief::*;
mod building;
pub use building::*;
mod city_state_type;
pub use city_state_type::*;
mod difficulty;
pub use difficulty::*;
mod era;
pub use era::*;
mod feature;
pub use feature::*;
mod nation;
pub use nation::*;
mod natural_wonder;
pub use natural_wonder::*;
mod policy_branch;
pub use policy_branch::*;
mod quest;
pub use quest::*;
mod resource;
pub use resource::*;
mod ruin;
pub use ruin::*;
mod specialist;
pub use specialist::*;
mod terrain_type;
pub use terrain_type::*;
mod tile_improvement;
pub use tile_improvement::*;
mod unit;
pub use unit::*;
mod unit_promotion;
pub use unit_promotion::*;
mod unit_type;
pub use unit_type::*;
mod victory_type;
pub use victory_type::*;
mod technology;
pub use technology::*;
mod policy;
pub use policy::*;
mod religion;
pub use religion::*;

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
