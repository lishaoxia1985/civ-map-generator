//! # Civilization Map Generator
//!
//! This crate provides a map generation algorithm for civilization-style games.
//! The implementation is primarily based on Civilization V, with some
//! references from Civilization VI.

////////////////////////////////////////////////////////////////////////////////

pub mod fractal;
pub mod grid;
pub mod map_generator;
pub mod map_parameters;
pub mod nation;
pub mod ruleset;
pub mod tile;
pub mod tile_component;
pub mod tile_map;

use map_generator::{Generator, fractal::Fractal, pangaea::Pangaea};
use map_parameters::{MapParameters, MapType};
use ruleset::Ruleset;
use tile_map::TileMap;

/// Generates a map based on the provided parameters and ruleset.
pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters, ruleset),
        MapType::Pangaea => Pangaea::generate(map_parameters, ruleset),
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate_map, map_parameters::MapParameters, ruleset::Ruleset};

    /// Tests for consistent map generation output when provided with the same random seed.
    #[test]
    fn test_generate_map() {
        let map_parameters = MapParameters::default();
        let ruleset = Ruleset::default();
        for _ in 0..25 {
            let map_a = generate_map(&map_parameters, &ruleset);
            let map_b = generate_map(&map_parameters, &ruleset);
            assert_eq!(map_a, map_b);
        }
    }
}
