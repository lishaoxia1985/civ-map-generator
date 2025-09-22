//! This module contains the components of the tile in the map.
//! For example, it includes the tile's TerrainType, BaseTerrain, NationWonder, Resource, and so on.

mod base_terrain;
mod feature;
mod natural_wonder;
mod resource;
mod terrain_type;

pub use base_terrain::*;
pub use feature::*;
pub use natural_wonder::*;
pub use resource::*;
pub use terrain_type::*;
