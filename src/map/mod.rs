pub mod fractal;
pub mod pangaea;

use crate::{
    ruleset::Ruleset,
    tile_map::{MapParameters, TileMap},
};

/// A trait that allows for the generation of a tile map.
///
/// If you want to create a new map generator, you need to implement this trait.
pub trait Generator {
    fn into_inner(self) -> TileMap;

    fn tile_map_mut(&mut self) -> &mut TileMap;

    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_terrain_types(map_parameters);
    }

    fn generate_coasts(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_coasts(map_parameters);
    }

    fn recalculate_areas(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().recalculate_areas(map_parameters);
    }

    fn generate_lakes(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_lakes(map_parameters);
    }

    fn generate_terrain(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().generate_terrain(map_parameters);
    }

    fn add_rivers(&mut self, map_parameters: &MapParameters) {
        self.tile_map_mut().add_rivers(map_parameters);
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

    fn generate(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.generate_terrain_types(&map_parameters);
        self.generate_coasts(&map_parameters);
        self.recalculate_areas(&map_parameters);
        self.generate_lakes(&map_parameters);
        self.generate_terrain(&map_parameters);
        self.add_rivers(&map_parameters);
        self.add_lakes(&map_parameters);
        self.recalculate_areas(&map_parameters);
        self.add_features(&map_parameters, &ruleset);
        self.recalculate_areas(&map_parameters);
        self.generate_regions(&map_parameters);
        self.start_plot_system(&map_parameters, &ruleset);
    }
}
