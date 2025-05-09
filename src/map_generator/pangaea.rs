use glam::{DVec2, IVec2};
use rand::Rng;

use crate::{
    component::map_component::terrain_type::TerrainType,
    fractal::{CvFractal, Flags},
    generate_common_methods,
    map_parameters::{MapParameters, SeaLevel, WorldAge, WorldSize},
    tile_map::TileMap,
};

use super::Generator;

pub struct Pangaea(TileMap);

impl Generator for Pangaea {
    generate_common_methods!();

    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        let tile_map = self.tile_map_mut();

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

        let grain = match map_parameters.map_size.world_size {
            WorldSize::Duel => 3,
            WorldSize::Tiny => 3,
            WorldSize::Small => 4,
            WorldSize::Standard => 4,
            WorldSize::Large => 5,
            WorldSize::Huge => 5,
        };

        let num_plates = match map_parameters.map_size.world_size {
            WorldSize::Duel => 6,
            WorldSize::Tiny => 9,
            WorldSize::Small => 12,
            WorldSize::Standard => 18,
            WorldSize::Large => 24,
            WorldSize::Huge => 30,
        };

        let grid = map_parameters.grid;

        let width = map_parameters.map_size.width;
        let height = map_parameters.map_size.height;

        let continents_fractal = tile_map.continents_fractal(map_parameters);

        let mut mountains_fractal = CvFractal::create(
            &mut tile_map.random_number_generator,
            width,
            height,
            4,
            Flags {
                map_wrapping: map_parameters.map_wrapping,
                ..Default::default()
            },
            7,
            6,
        );

        mountains_fractal.ridge_builder(
            &mut tile_map.random_number_generator,
            num_plates * 2 / 3,
            &Flags {
                map_wrapping: map_parameters.map_wrapping,
                ..Default::default()
            },
            6,
            1,
            grid,
        );

        let mut hills_fractal = CvFractal::create(
            &mut tile_map.random_number_generator,
            width,
            height,
            grain,
            Flags {
                map_wrapping: map_parameters.map_wrapping,
                ..Default::default()
            },
            7,
            6,
        );

        hills_fractal.ridge_builder(
            &mut tile_map.random_number_generator,
            num_plates,
            &Flags {
                map_wrapping: map_parameters.map_wrapping,
                ..Default::default()
            },
            1,
            2,
            grid,
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

        let width = map_parameters.map_size.width;
        let height = map_parameters.map_size.height;
        let center_position = DVec2::new(width as f64 / 2., height as f64 / 2.);

        let axis = center_position * 3. / 5.;

        tile_map.iter_tiles().for_each(|tile| {
            let [x, y] = tile.to_offset_coordinate(grid).to_array();
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

            let height = ((height as f64 + h + h) * 0.33) as i32;

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
