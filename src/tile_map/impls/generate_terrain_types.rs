use rand::Rng;

use crate::{
    component::map_component::terrain_type::TerrainType,
    fractal::{CvFractal, FractalFlags},
    grid::WorldSizeType,
    map_parameters::{SeaLevel, WorldAge},
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    /// Generate terrain types for the map.
    /// This function uses the map's parameters to determine the terrain types for each tile.
    pub fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        let sea_level_low = 65;
        let sea_level_normal = 72;
        let sea_level_high = 78;
        let world_age_old = 2;
        let world_age_normal = 3;
        let world_age_new = 5;

        let extra_mountains = 0;

        // TODO: `tectonic_islands` should be configurable by the user in the future.
        let tectonic_islands = false;

        let adjustment = match map_parameters.world_age {
            WorldAge::Old => world_age_old,
            WorldAge::Normal => world_age_normal,
            WorldAge::New => world_age_new,
        };

        let adjust_plates = match map_parameters.world_age {
            WorldAge::Old => 0.75,
            WorldAge::Normal => 1.0,
            WorldAge::New => 1.5,
        };

        let mountains = 97 - adjustment - extra_mountains;
        let hills_near_mountains = 91 - (adjustment * 2) - extra_mountains;
        let hills_bottom1 = 28 - adjustment;
        let hills_top1 = 28 + adjustment;
        let hills_bottom2 = 72 - adjustment;
        let hills_top2 = 72 + adjustment;
        let hills_clumps = 1 + adjustment;

        let water_percent = match map_parameters.sea_level {
            SeaLevel::Low => sea_level_low,
            SeaLevel::Normal => sea_level_normal,
            SeaLevel::High => sea_level_high,
            SeaLevel::Random => self
                .random_number_generator
                .gen_range(sea_level_low..=sea_level_high),
        };

        let grain = match self.world_grid.world_size_type {
            WorldSizeType::Duel => 3,
            WorldSizeType::Tiny => 3,
            WorldSizeType::Small => 4,
            WorldSizeType::Standard => 4,
            WorldSizeType::Large => 5,
            WorldSizeType::Huge => 5,
        };

        let mut num_plates = match self.world_grid.world_size_type {
            WorldSizeType::Duel => 6,
            WorldSizeType::Tiny => 9,
            WorldSizeType::Small => 12,
            WorldSizeType::Standard => 18,
            WorldSizeType::Large => 24,
            WorldSizeType::Huge => 30,
        };

        num_plates = (num_plates as f64 * adjust_plates) as u32;

        let grid = self.world_grid.grid;

        let continents_fractal = self.continents_fractal();

        let flags = FractalFlags::empty();

        let mut mountains_fractal =
            CvFractal::create(&mut self.random_number_generator, grid, grain, flags, 7, 6);

        mountains_fractal.ridge_builder(
            &mut self.random_number_generator,
            num_plates * 2 / 3,
            flags,
            6,
            1,
        );

        let mut hills_fractal =
            CvFractal::create(&mut self.random_number_generator, grid, grain, flags, 7, 6);

        hills_fractal.ridge_builder(&mut self.random_number_generator, num_plates, flags, 1, 2);

        let [water_threshold] = continents_fractal.get_height_from_percents([water_percent]);

        let [pass_threshold, hills_bottom1, hills_top1, hills_bottom2, hills_top2] = hills_fractal
            .get_height_from_percents([
                hills_near_mountains,
                hills_bottom1,
                hills_top1,
                hills_bottom2,
                hills_top2,
            ]);

        let [mountain_threshold, hills_near_mountains, _hills_clumps, mountain_100, mountain_99, _mountain_98, mountain_97, mountain_95] =
            mountains_fractal.get_height_from_percents([
                mountains,
                hills_near_mountains,
                hills_clumps,
                100,
                99,
                98,
                97,
                95,
            ]);

        self.iter_tiles().for_each(|tile| {
            let [x, y] = tile.to_offset(grid).to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);

            if height <= water_threshold {
                self.terrain_type_query[tile.index()] = TerrainType::Water;
                // No hills or mountains here, but check for tectonic islands if that setting is active.
                if tectonic_islands {
                    // Build islands in oceans along tectonic ridge lines
                    if mountain_height == mountain_100 {
                        // Isolated peak in the ocean
                        self.terrain_type_query[tile.index()] = TerrainType::Mountain;
                    } else if mountain_height == mountain_99 {
                        self.terrain_type_query[tile.index()] = TerrainType::Hill;
                    } else if (mountain_height == mountain_97) || (mountain_height == mountain_95) {
                        self.terrain_type_query[tile.index()] = TerrainType::Flatland;
                    }
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    self.terrain_type_query[tile.index()] = TerrainType::Hill;
                } else {
                    self.terrain_type_query[tile.index()] = TerrainType::Mountain;
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                self.terrain_type_query[tile.index()] = TerrainType::Hill;
            } else {
                self.terrain_type_query[tile.index()] = TerrainType::Flatland;
            };
        });
    }

    pub fn continents_fractal(&mut self) -> CvFractal {
        let continent_grain = 2;
        // Default no rifts. Set grain to between 1 and 3 to add rifts.
        let rift_grain = -1;

        let num_plates_for_continents = match self.world_grid.world_size_type {
            WorldSizeType::Duel => 4,
            WorldSizeType::Tiny => 8,
            WorldSizeType::Small => 16,
            WorldSizeType::Standard => 20,
            WorldSizeType::Large => 24,
            WorldSizeType::Huge => 32,
        };

        let grid = self.world_grid.grid;

        let flags = FractalFlags::empty();

        let mut continents_fractal = if rift_grain > 0 && rift_grain < 4 {
            let rift_fractal = CvFractal::create(
                &mut self.random_number_generator,
                grid,
                rift_grain,
                flags,
                7,
                6,
            );

            CvFractal::create_rifts(
                &mut self.random_number_generator,
                grid,
                continent_grain,
                flags,
                &rift_fractal,
                7,
                6,
            )
        } else {
            CvFractal::create(
                &mut self.random_number_generator,
                grid,
                continent_grain,
                flags,
                7,
                6,
            )
        };

        // Blend a bit of ridge into the fractal.
        // This will do things like roughen the coastlines and build inland seas.
        continents_fractal.ridge_builder(
            &mut self.random_number_generator,
            num_plates_for_continents,
            flags,
            1,
            2,
        );

        continents_fractal
    }
}
