use rand::Rng;

use crate::{
    component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    tile_map::{tile::Tile, MapParameters, TileMap},
};

impl TileMap {
    /// This function generates lakes on the map.
    ///
    /// This fun is used because when we create the world map by System `spawn_tile_type`, some water areas will be created surrounded by land.
    /// If these water areas are small enough, they will be considered as lakes and will be replaced by the `TerrainType::Lake` terrain.
    pub fn generate_lakes(&mut self, map_parameters: &MapParameters) {
        self.iter_tiles().for_each(|tile| {
            if tile.terrain_type(self) == TerrainType::Water
                && self.area_id_and_size[&tile.area_id(self)] <= map_parameters.lake_max_area_size
            {
                self.base_terrain_query[tile.index()] = BaseTerrain::Lake;
            }
        });
    }

    pub fn add_lakes(&mut self, map_parameters: &MapParameters) {
        let large_lake_num = map_parameters.large_lake_num;

        let mut num_large_lakes_added = 0;
        let lake_plot_rand = 25;

        self.iter_tiles().for_each(|tile| {
            if self.can_add_lake(tile, map_parameters)
                && self.random_number_generator.gen_range(0..lake_plot_rand) == 0
            {
                if num_large_lakes_added < large_lake_num {
                    let add_more_lakes = self.add_more_lake(tile, map_parameters);

                    if add_more_lakes {
                        num_large_lakes_added += 1;
                    }
                }
                self.terrain_type_query[tile.index()] = TerrainType::Water;
                self.base_terrain_query[tile.index()] = BaseTerrain::Lake;
                self.feature_query[tile.index()] = None;
            }
        });
    }

    fn add_more_lake(&mut self, tile: Tile, map_parameters: &MapParameters) -> bool {
        let mut large_lake = 0;

        let mut lake_tile = Vec::new();

        tile.neighbor_tiles(map_parameters)
            .into_iter()
            .for_each(|neighbor_tile| {
                if self.can_add_lake(neighbor_tile, map_parameters)
                    && self.random_number_generator.gen_range(0..(large_lake + 4)) < 3
                {
                    lake_tile.push(neighbor_tile);
                    large_lake += 1;
                }
            });

        lake_tile.into_iter().for_each(|tile| {
            self.terrain_type_query[tile.index()] = TerrainType::Water;
            self.base_terrain_query[tile.index()] = BaseTerrain::Lake;
            self.feature_query[tile.index()] = None;
        });

        large_lake > 2
    }

    /// Checks if a tile can have a lake.
    ///
    /// A tile can have a lake if it meets the following conditions:
    /// 1. The tile is not water.
    /// 2. The tile is not a natural wonder.
    /// 3. The tile is not adjacent to a river.
    /// 4. The tile is not adjacent to water.
    /// 5. The tile is not adjacent to a natural wonder.
    ///
    /// # Parameters
    /// - `tile`: The tile being checked.
    /// - `map_parameters`: A reference to the map parameters to retrieve edge directions and neighboring tile information.
    ///
    /// # Returns
    /// - `true` if the tile can have a lake, otherwise `false`.
    fn can_add_lake(&self, tile: Tile, map_parameters: &MapParameters) -> bool {
        // Check if the current tile is suitable for a lake
        if tile.terrain_type(self) == TerrainType::Water
            || tile.natural_wonder(self).is_some()
            || tile.has_river(self, map_parameters)
        {
            return false;
        }

        let neighbor_tiles = tile.neighbor_tiles(map_parameters);

        // Check if all neighbor tiles are also suitable
        neighbor_tiles.iter().all(|neighbor_tile| {
            neighbor_tile.terrain_type(self) != TerrainType::Water
                && neighbor_tile.natural_wonder(self).is_none()
        })
    }
}
