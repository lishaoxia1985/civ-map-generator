use std::cmp::max;

use rand::Rng;

use crate::{
    component::map_component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    fractal::{CvFractal, FractalFlags},
    map_parameters::Temperature,
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    /// Generate base terrains except for [`BaseTerrain::Lake`].
    ///
    /// # Notice
    /// We don't generate [`BaseTerrain::Lake`] here, because the lake is a special base terrain that is generated in the [`TileMap::generate_lakes`] and [`TileMap::add_lakes`] method.
    pub fn generate_base_terrains(&mut self, map_parameters: &MapParameters) {
        let grid = self.world_grid.grid;

        let grain_amount = 3;

        let temperature_shift = 0.1;
        let desert_shift = 16;
        let mut desert_percent = 32;
        let plains_percent = 50;

        // Set default base terrain bands.
        // TODO: This should be moved to the map parameters and be configurable by the user in the future.
        // Notice: The number should be sorted in ascending order.
        let [mut grass_latitude, desert_bottom_latitude, mut desert_top_latitude, mut tundra_latitude, mut snow_latitude] =
            [0.1, 0.2, 0.5, 0.6, 0.75];

        match map_parameters.temperature {
            Temperature::Cool => {
                desert_percent -= desert_shift;
                tundra_latitude -= temperature_shift * 1.5;
                desert_top_latitude -= temperature_shift;
                grass_latitude -= temperature_shift * 0.5;
            }
            Temperature::Normal => {}
            Temperature::Hot => {
                desert_percent += desert_shift;
                snow_latitude += temperature_shift * 0.5;
                tundra_latitude += temperature_shift;
                desert_top_latitude += temperature_shift;
                grass_latitude -= temperature_shift * 0.5;
            }
        }

        let desert_top_percent = 100;
        let desert_bottom_percent = max(0, 100 - desert_percent);
        let plains_top_percent = 100;
        let plains_bottom_percent = max(0, 100 - plains_percent);

        let flags = FractalFlags::empty();

        //let (seed, seed2, seed3) = self.random_number_generator.gen();
        let variation_fractal = CvFractal::create(
            &mut self.random_number_generator,
            grid,
            grain_amount,
            flags,
            CvFractal::DEFAULT_WIDTH_EXP,
            CvFractal::DEFAULT_HEIGHT_EXP,
        );
        let deserts_fractal = CvFractal::create(
            &mut self.random_number_generator,
            grid,
            grain_amount,
            flags,
            CvFractal::DEFAULT_WIDTH_EXP,
            CvFractal::DEFAULT_HEIGHT_EXP,
        );
        let plains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            grid,
            grain_amount,
            flags,
            CvFractal::DEFAULT_WIDTH_EXP,
            CvFractal::DEFAULT_HEIGHT_EXP,
        );

        let [desert_top, plains_top] =
            deserts_fractal.get_height_from_percents([desert_top_percent, plains_top_percent]);
        let [desert_bottom, plains_bottom] =
            plains_fractal.get_height_from_percents([desert_bottom_percent, plains_bottom_percent]);

        self.iter_tiles().for_each(|tile| {
            let terrain_type = tile.terrain_type(self);
            match terrain_type {
                TerrainType::Water => {
                    // Generate coast terrain.
                    //
                    // The tiles that can be coast should meet all the conditions as follows:
                    // 1. They are ocean, that means they are water, not lake and not already coast.
                    // 2. They have at least one neighbor that is not water.
                    let neighbor_tiles = tile.neighbor_tiles(grid);
                    if tile.base_terrain(self) == BaseTerrain::Ocean
                        && neighbor_tiles.iter().any(|&neighbor_tile| {
                            neighbor_tile.terrain_type(self) != TerrainType::Water
                        })
                    {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Coast;
                    }
                }
                TerrainType::Flatland | TerrainType::Hill | TerrainType::Mountain => {
                    // Generate base terrain for land tiles.
                    let [x, y] = tile.to_offset(grid).to_array();

                    // Set default base terrain of all land tiles to `BaseTerrain::Grassland` because the default base terrain is `BaseTerrain::Ocean` in the tile map.
                    self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;

                    let deserts_height = deserts_fractal.get_height(x, y);
                    let plains_height = plains_fractal.get_height(x, y);

                    let mut latitude = tile.latitude(grid);
                    latitude += (128 - variation_fractal.get_height(x, y)) as f64 / (255.0 * 5.0);
                    latitude = latitude.clamp(0., 1.);

                    if latitude >= snow_latitude {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Snow;
                    } else if latitude >= tundra_latitude {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Tundra;
                    } else if latitude < grass_latitude {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
                    } else if deserts_height >= desert_bottom
                        && deserts_height <= desert_top
                        && latitude >= desert_bottom_latitude
                        && latitude < desert_top_latitude
                    {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Desert;
                    } else if plains_height >= plains_bottom && plains_height <= plains_top {
                        self.base_terrain_query[tile.index()] = BaseTerrain::Plain;
                    }
                }
            }
        });
    }

    /// Expand coast terrain.
    ///
    /// The tiles that can be expanded should meet all the conditions as follows:
    /// 1. They are water and not already coast
    /// 2. They have at least one neighbor that is coast
    /// 3. A random number generator will be used to determine whether the tile will be expanded.
    /// # Notice
    /// This method is called after the [`TileMap::generate_base_terrains`] method.
    pub fn expand_coasts(&mut self, map_parameters: &MapParameters) {
        let grid = self.world_grid.grid;
        map_parameters
            .coast_expand_chance
            .iter()
            .for_each(|&chance| {
                let mut expansion_tile = Vec::new();
                /* Don't update the base_terrain of the tile in the iteration.
                Because if we update the base_terrain of the tile in the iteration,
                the tile will be used in the next iteration(e.g. tile.tile_neighbors().iter().any()),
                which will cause the result to be wrong. */
                self.iter_tiles().for_each(|tile| {
                    // The tiles that can be expanded should meet some conditions:
                    //      1. They are ocean, that means they are water, not lake and not already coast.
                    //      2. They have at least one neighbor that is coast.
                    if tile.base_terrain(self) == BaseTerrain::Ocean
                        && tile.neighbor_tiles(grid).iter().any(|neighbor_tile| {
                            neighbor_tile.base_terrain(self) == BaseTerrain::Coast
                        })
                        && self.random_number_generator.gen_bool(chance)
                    {
                        expansion_tile.push(tile);
                    }
                });

                expansion_tile.into_iter().for_each(|tile| {
                    self.base_terrain_query[tile.index()] = BaseTerrain::Coast;
                });
            });
    }
}
