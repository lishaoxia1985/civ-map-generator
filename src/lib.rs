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

pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters, ruleset),
        MapType::Pangaea => Pangaea::generate(map_parameters, ruleset),
    }
}
