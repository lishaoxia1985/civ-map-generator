use glam::{DVec2, IVec2};
use rand::Rng;

use crate::{
    fractal::{CvFractal, FractalFlags},
    generate_common_methods,
    grid::WorldSizeType,
    map_parameters::{MapParameters, SeaLevel, WorldAge},
    tile_component::terrain_type::TerrainType,
    tile_map::TileMap,
};

use super::Generator;

pub struct Pangaea(TileMap);

impl Generator for Pangaea {
    generate_common_methods!();

    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        let tile_map = self.tile_map_mut();
        let world_grid = tile_map.world_grid;
        let grid = world_grid.grid;

        let sea_level_low = 71;
        let sea_level_normal = 78;
        let sea_level_high = 84;
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
            SeaLevel::Random => tile_map
                .random_number_generator
                .gen_range(sea_level_low..=sea_level_high),
        };

        let grain = match world_grid.world_size_type {
            WorldSizeType::Duel => 3,
            WorldSizeType::Tiny => 3,
            WorldSizeType::Small => 4,
            WorldSizeType::Standard => 4,
            WorldSizeType::Large => 5,
            WorldSizeType::Huge => 5,
        };

        let num_plates = match world_grid.world_size_type {
            WorldSizeType::Duel => 6,
            WorldSizeType::Tiny => 9,
            WorldSizeType::Small => 12,
            WorldSizeType::Standard => 18,
            WorldSizeType::Large => 24,
            WorldSizeType::Huge => 30,
        };

        let continents_fractal = tile_map.continents_fractal();

        let flags = FractalFlags::empty();

        let mut mountains_fractal =
            CvFractal::create(&mut tile_map.random_number_generator, grid, 4, flags, 7, 6);

        mountains_fractal.ridge_builder(
            &mut tile_map.random_number_generator,
            num_plates * 2 / 3,
            flags,
            6,
            1,
        );

        let mut hills_fractal = CvFractal::create(
            &mut tile_map.random_number_generator,
            grid,
            grain,
            flags,
            7,
            6,
        );

        hills_fractal.ridge_builder(
            &mut tile_map.random_number_generator,
            num_plates,
            flags,
            1,
            2,
        );

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

        let width = grid.size.width;
        let height = grid.size.height;
        let center_position = DVec2::new(width as f64 / 2., height as f64 / 2.);

        let axis = center_position * 3. / 5.;

        tile_map.all_tiles().for_each(|tile| {
            let [x, y] = tile.to_offset(grid).to_array();
            let height = continents_fractal.get_height(x, y);

            let mountain_height = mountains_fractal.get_height(x, y);
            let hill_height = hills_fractal.get_height(x, y);

            let mut h = water_threshold as f64;

            let delta = IVec2::from([x, y]).as_dvec2() - center_position;
            let d = (delta / axis).length_squared();

            if d <= 1. {
                h = h + (h * 0.125)
            } else {
                h = h - (h * 0.125)
            }

            let height = ((height as f64 + h + h) * 0.33) as u32;

            if height <= water_threshold {
                // No hills or mountains here, but check for tectonic islands if that setting is active.
                if tectonic_islands {
                    // Build islands in oceans along tectonic ridge lines
                    if mountain_height == mountain_100 {
                        // Isolated peak in the ocean
                        tile_map.terrain_type_query[tile.index()] = TerrainType::Mountain;
                    } else if mountain_height == mountain_99 {
                        tile_map.terrain_type_query[tile.index()] = TerrainType::Hill;
                    } else if (mountain_height == mountain_97) || (mountain_height == mountain_95) {
                        tile_map.terrain_type_query[tile.index()] = TerrainType::Flatland;
                    }
                }
            } else if mountain_height >= mountain_threshold {
                if hill_height >= pass_threshold {
                    tile_map.terrain_type_query[tile.index()] = TerrainType::Hill;
                } else {
                    tile_map.terrain_type_query[tile.index()] = TerrainType::Mountain;
                }
            } else if mountain_height >= hills_near_mountains
                || (hill_height >= hills_bottom1 && hill_height <= hills_top1)
                || (hill_height >= hills_bottom2 && hill_height <= hills_top2)
            {
                tile_map.terrain_type_query[tile.index()] = TerrainType::Hill;
            } else {
                tile_map.terrain_type_query[tile.index()] = TerrainType::Flatland;
            };
        });
    }
}
