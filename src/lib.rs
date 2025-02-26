pub mod component;
pub mod grid;
mod map;
pub mod ruleset;
pub mod tile_map;

pub use component::*;
pub use grid::*;
use map::{fractal::FractalMap, pangaea::PangaeaMap, Generator};
use tile_map::{MapType, TileMap};

pub fn generate_map(
    map_parameters: &tile_map::MapParameters,
    ruleset: &ruleset::Ruleset,
) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => {
            let mut map = FractalMap::new(map_parameters);
            map.generate(map_parameters, ruleset);
            map.into_inner()
        }
        MapType::Pangaea => {
            let mut map = PangaeaMap::new(map_parameters);
            map.generate(map_parameters, ruleset);
            map.into_inner()
        }
    }
}
