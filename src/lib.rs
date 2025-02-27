pub mod component;
pub mod grid;
mod map;
pub mod ruleset;
pub mod tile_map;

pub use component::*;
pub use grid::*;
use map::{fractal::Fractal, pangaea::Pangaea, Generator};
use ruleset::Ruleset;
use tile_map::{MapParameters, MapType, TileMap};

macro_rules! generate_map_from_type {
    // Match each MapType variant
    ($map_parameters:expr, $ruleset:expr, $map_type:ident) => {{
        // Generate the corresponding map based on map_type
        let mut map = $map_type::new($map_parameters);  // Create a new map instance
        map.generate($map_parameters, $ruleset);  // Generate the map with the given parameters and rules
        map.into_inner()  // Return the generated map
    }};
}

pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => generate_map_from_type!(map_parameters, ruleset, Fractal),
        MapType::Pangaea => generate_map_from_type!(map_parameters, ruleset, Pangaea),
    }
}
