use std::cmp::max;

use crate::{
    component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    tile_map::{CvFractal, Flags, MapParameters, Temperature, TileMap},
};

impl TileMap {
    pub fn generate_terrain(&mut self, map_parameters: &MapParameters) {
        let temperature_shift = 0.1;
        let desert_shift = 16;
        let mut desert_percent = 32;
        let plains_percent = 50;
        let mut snow_latitude = 0.75;
        let mut tundra_latitude = 0.6;
        let mut grass_latitude = 0.1;
        let desert_bottom_latitude = 0.2;
        let mut desert_top_latitude = 0.5;

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

        //let (seed, seed2, seed3) = self.random_number_generator.gen();
        let variation_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            3,
            Flags {
                wrap_x: map_parameters.wrap_x,
                wrap_y: map_parameters.wrap_y,
                ..Default::default()
            },
            -1,
            -1,
        );
        let deserts_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            3,
            Flags {
                wrap_x: map_parameters.wrap_x,
                wrap_y: map_parameters.wrap_y,
                ..Default::default()
            },
            -1,
            -1,
        );
        let plains_fractal = CvFractal::create(
            &mut self.random_number_generator,
            map_parameters.map_size.width,
            map_parameters.map_size.height,
            3,
            Flags {
                wrap_x: map_parameters.wrap_x,
                wrap_y: map_parameters.wrap_y,
                ..Default::default()
            },
            -1,
            -1,
        );

        let [desert_top, plains_top] =
            deserts_fractal.get_height_from_percents(&[desert_top_percent, plains_top_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };
        let [desert_bottom, plains_bottom] = plains_fractal
            .get_height_from_percents(&[desert_bottom_percent, plains_bottom_percent])[..]
        else {
            panic!("Vec length does not match the pattern")
        };

        self.iter_tiles().for_each(|tile| {
            if self.terrain_type_query[tile.index()] != TerrainType::Water {
                let [x, y] = tile.to_offset_coordinate(map_parameters).to_array();

                // Set default base terrain of all land tiles to `BaseTerrain::Grassland` because the default base terrain is `BaseTerrain::Ocean` in the tile map.
                self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;

                let deserts_height = deserts_fractal.get_height(x, y);
                let plains_height = plains_fractal.get_height(x, y);

                let mut latitude = tile.latitude(map_parameters);
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
        });
    }
}
