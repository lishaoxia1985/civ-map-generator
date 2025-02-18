use rand::Rng;

use crate::{
    component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    /// Generate coast terrain.
    ///
    /// The algorithm is as follows:
    /// 1. For each tile, if it is water and has at least one neighbor that is not water, set its base_terrain to coast.
    /// 2. Expand the coast terrain to its eligible neighbors according the Vec `coast_expand_chance` in MapParameters.
    pub fn generate_coasts(&mut self, map_parameters: &MapParameters) {
        self.iter_tiles().for_each(|tile| {
            if tile.terrain_type(self) == TerrainType::Water {
                let neighbor_tiles = tile.neighbor_tiles(map_parameters);
                if neighbor_tiles
                    .iter()
                    .any(|&neighbor_tile| neighbor_tile.terrain_type(self) != TerrainType::Water)
                {
                    self.base_terrain_query[tile.index()] = BaseTerrain::Coast;
                }
            }
        });

        self.expand_coasts(map_parameters);
    }

    /// Expand coast terrain.
    ///
    /// The tiles that can be expanded should meet some conditions:
    /// 1. They are water and not already coast
    /// 2. They have at least one neighbor that is coast
    fn expand_coasts(&mut self, map_parameters: &MapParameters) {
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
                    //      1. They are water and not already coast
                    //      2. They have at least one neighbor that is coast

                    // Notice: we don't replce `tile.is_water() && tile.base_terrain != BaseTerrain::Coast` with `tile.base_terrain = BaseTerrain::Ocean`,
                    //      because when we create the map we set Ocean as the default BaseTerrain to all the tile,
                    //      that means at this time there are some tiles that their base_terrain = Ocean but their terrain_type is not Water!
                    //      We will tackle with this situation in [`TileMap::generate_terrain`].
                    if tile.terrain_type(self) == TerrainType::Water
                        && tile.base_terrain(self) != BaseTerrain::Coast
                        && tile
                            .neighbor_tiles(map_parameters)
                            .iter()
                            .any(|neighbor_tile| {
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
