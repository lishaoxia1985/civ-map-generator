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

    fn shift_terrain_types(&mut self) {
        self.tile_map_mut().shift_terrain_types();
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

    fn choose_civilization_starting_tiles(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut()
            .choose_civilization_starting_tiles(map_parameters);
    }

    fn balance_and_assign_civilization_starting_tiles(
        &mut self,
        map_parameters: &MapParameters,
        ruleset: &Ruleset,
    ) {
        self.tile_map_mut()
            .balance_and_assign_civilization_starting_tiles(map_parameters, ruleset);
    }

    fn place_natural_wonders(&mut self, ruleset: &Ruleset) {
        self.tile_map_mut().place_natural_wonders(ruleset);
    }

    fn assign_luxury_roles(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().assign_luxury_roles(map_parameters);
    }

    fn place_city_states(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.tile_map_mut()
            .place_city_states(map_parameters, ruleset);
    }

    fn place_luxury_resources(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.tile_map_mut()
            .place_luxury_resources(map_parameters, ruleset);
    }

    fn place_strategic_resources(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut()
            .place_strategic_resources(map_parameters);
    }

    fn place_bonus_resources(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().place_bonus_resources(map_parameters);
    }

    fn normalize_city_state_locations(&mut self) {
        self.tile_map_mut().normalize_city_state_locations();
    }

    fn fix_sugar_jungles(&mut self) {
        self.tile_map_mut().fix_sugar_jungles();
    }

    fn generate(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap
    where
        Self: Sized,
    {
        let mut map = Self::new(map_parameters);
        // The order of the following methods is important. Do not change it.

        /********** Process 1: Generate Terrain Types, Base Terrains, Features and add Rivers **********/
        map.generate_terrain_types(map_parameters);

        map.shift_terrain_types();

        map.recalculate_areas(ruleset);

        map.generate_lakes(map_parameters);

        map.generate_base_terrains(map_parameters);

        map.expand_coasts(map_parameters);

        map.add_rivers();

        map.add_lakes(map_parameters);

        map.recalculate_areas(ruleset);

        map.add_features(map_parameters, ruleset);

        map.recalculate_areas(ruleset);
        /********** The End of Process 1 **********/

        /********** Process 2: Place Civs, Natural Wonders, City-States and Resources **********/
        map.generate_regions(map_parameters);

        map.choose_civilization_starting_tiles(map_parameters);

        map.balance_and_assign_civilization_starting_tiles(map_parameters, ruleset);

        map.place_natural_wonders(ruleset);

        map.assign_luxury_roles(map_parameters);

        map.place_city_states(map_parameters, ruleset);

        // We have replace this code with `TileMap::generate_bonus_resource_tile_lists_in_map`,
        // `TileMap::generate_luxury_resource_tile_lists_in_map` and `TileMap::generate_strategic_resource_tile_lists_in_map`.
        // So the commented code is unnecessary.
        /* self:GenerateGlobalResourcePlotLists() */

        map.place_luxury_resources(map_parameters, ruleset);

        map.place_strategic_resources(map_parameters);

        map.place_bonus_resources(map_parameters);

        map.normalize_city_state_locations();
        /********** The End of Process 2 **********/

        /********** Process 3: Fix Graphics and Recalculate Areas **********/
        map.fix_sugar_jungles();

        map.recalculate_areas(ruleset);
        /********** The End of Process 3 **********/

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
