//! This module defines the `Generator` trait for map generation and provides common methods for map generators.

use crate::{map_parameters::MapParameters, ruleset::Ruleset, tile_map::TileMap};

pub mod fractal;
pub mod pangaea;

/// A trait that allows for the generation of a tile map.
///
/// If you want to create a new map generator, you need to implement this trait.
pub trait Generator {
    fn new(map_parameters: &MapParameters) -> Self;

    fn into_inner(self) -> TileMap;

    fn tile_map_mut(&mut self) -> &mut TileMap;

    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_terrain_types(map_parameters);
    }

    fn recalculate_areas(&mut self, ruleset: &Ruleset) {
        self.tile_map_mut().recalculate_areas(ruleset);
    }

    fn generate_lakes(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_lakes(map_parameters);
    }

    fn generate_base_terrains(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_base_terrains(map_parameters);
    }

    fn expand_coasts(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().expand_coasts(map_parameters);
    }

    fn add_rivers(&mut self) {
        self.tile_map_mut().add_rivers();
    }

    fn add_lakes(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().add_lakes(map_parameters);
    }

    fn add_features(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.tile_map_mut().add_features(map_parameters, ruleset);
    }

    fn generate_regions(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_regions(map_parameters);
    }

    fn start_plot_system(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.tile_map_mut()
            .start_plot_system(map_parameters, ruleset);
    }

    fn generate(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap
    where
        Self: Sized,
    {
        let mut map = Self::new(map_parameters);
        // The order of the following methods is important. Do not change it.
        map.generate_terrain_types(map_parameters);
        map.recalculate_areas(ruleset);
        map.generate_lakes(map_parameters);
        map.generate_base_terrains(map_parameters);
        map.expand_coasts(map_parameters);
        map.add_rivers();
        map.add_lakes(map_parameters);
        map.recalculate_areas(ruleset);
        map.add_features(map_parameters, ruleset);
        map.recalculate_areas(ruleset);
        map.generate_regions(map_parameters);
        map.start_plot_system(map_parameters, ruleset);
        map.into_inner()
    }
}

/// Generates common methods for a struct.
///
/// This macro generates the following methods:
/// - `new`: Creates a new instance of the struct with the given `MapParameters`.
/// - `into_inner`: Consumes the struct and returns the inner `TileMap`.
/// - `tile_map_mut`: Provides a mutable reference to the inner `TileMap`.
#[macro_export]
macro_rules! generate_common_methods {
    () => {
        /// Creates a new instance of the struct with the given `MapParameters`.
        fn new(map_parameters: &MapParameters) -> Self {
            Self(TileMap::new(map_parameters))
        }

        /// Consumes the struct and returns the inner `TileMap`.
        fn into_inner(self) -> TileMap {
            self.0
        }

        /// Provides a mutable reference to the inner `TileMap`.
        fn tile_map_mut(&mut self) -> &mut TileMap {
            &mut self.0
        }
    };
}
