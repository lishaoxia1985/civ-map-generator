pub mod component;
mod fractal;
pub mod grid;
mod map_generator;
pub mod map_parameters;
pub mod ruleset;
pub mod tile;
pub mod tile_map;

use map_generator::{fractal::Fractal, pangaea::Pangaea, Generator};
use map_parameters::{MapParameters, MapType};
use ruleset::Ruleset;
use tile_map::TileMap;

pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters, ruleset),
        MapType::Pangaea => Pangaea::generate(map_parameters, ruleset),
    }
}
