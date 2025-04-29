use std::collections::{BTreeSet, VecDeque};

use std::collections::HashSet;

use crate::component::map_component::terrain_type::TerrainType;
use crate::tile::Tile;
use crate::tile_map::{MapParameters, TileMap};

impl TileMap {
    /// Uses BFS to assign area IDs to tiles within the same area.
    fn bfs(&mut self, map_parameters: &MapParameters, mut area_tiles: HashSet<Tile>) {
        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&area_tile) = area_tiles.iter().next() {
            self.area_id_query[area_tile.index()] = current_area_id;
            area_tiles.remove(&area_tile);

            // Store all the entities in the current area.
            let mut tiles_in_current_area = HashSet::new();
            tiles_in_current_area.insert(area_tile);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tiles_to_check = VecDeque::new();
            tiles_to_check.push_back(area_tile);

            while let Some(tile_we_are_checking) = tiles_to_check.pop_front() {
                tile_we_are_checking
                    .neighbor_tiles(map_parameters)
                    .iter()
                    .for_each(|&tile| {
                        if !tiles_in_current_area.contains(&tile) && area_tiles.contains(&tile) {
                            tiles_in_current_area.insert(tile);
                            self.area_id_query[tile.index()] = current_area_id;
                            tiles_to_check.push_back(tile);
                            area_tiles.remove(&tile);
                        }
                    });
            }
            self.area_id_and_size
                .insert(current_area_id, tiles_in_current_area.len() as u32);
            current_area_id += 1;
        }
    }

    /// Uses DFS to assign area IDs to tiles within the same area.
    fn dfs(&mut self, map_parameters: &MapParameters, mut area_tiles: HashSet<Tile>) {
        let mut current_area_id = self.area_id_query.iter().max().unwrap() + 1;

        while let Some(&area_tile) = area_tiles.iter().next() {
            self.area_id_query[area_tile.index()] = current_area_id;
            area_tiles.remove(&area_tile);

            // Store all the entities in the current area.
            let mut tiles_in_current_area = HashSet::new();
            tiles_in_current_area.insert(area_tile);

            // Store all the entities that need to check whether their neighbors are in the current area within the following 'while {..}' loop.
            let mut tiles_to_check = Vec::new();
            tiles_to_check.push(area_tile);

            while let Some(tile_we_are_checking) = tiles_to_check.pop() {
                tile_we_are_checking
                    .neighbor_tiles(map_parameters)
                    .iter()
                    .for_each(|&tile| {
                        if !tiles_in_current_area.contains(&tile) && area_tiles.contains(&tile) {
                            tiles_in_current_area.insert(tile);
                            self.area_id_query[tile.index()] = current_area_id;
                            tiles_to_check.push(tile);
                            area_tiles.remove(&tile);
                        }
                    });
            }
            self.area_id_and_size
                .insert(current_area_id, tiles_in_current_area.len() as u32);
            current_area_id += 1;
        }
    }

    /// Recalculates the area IDs and sizes of the tiles in the map.
    ///
    /// This function is called when the map is generated or when the [`TerrainType`] of certain tiles changes.
    pub fn recalculate_areas(&mut self, map_parameters: &MapParameters) {
        self.area_id_and_size.clear();

        let height = map_parameters.map_size.height;
        let width = map_parameters.map_size.width;

        let size = (height * width) as usize;

        self.area_id_query = vec![-1; size];

        let mut water_tiles = HashSet::new();
        let mut hill_and_flatland_tiles = HashSet::new();
        let mut mountain_tiles = HashSet::new();

        self.iter_tiles().for_each(|tile| {
            match tile.terrain_type(self) {
                TerrainType::Water => water_tiles.insert(tile),
                TerrainType::Flatland | TerrainType::Hill => hill_and_flatland_tiles.insert(tile),
                TerrainType::Mountain => mountain_tiles.insert(tile),
            };
        });

        self.bfs(map_parameters, water_tiles);
        self.bfs(map_parameters, hill_and_flatland_tiles);
        self.bfs(map_parameters, mountain_tiles);

        self.reassign_area_id(map_parameters);
    }

    /// Reassigns the area IDs of small areas to the largest surrounding area.
    fn reassign_area_id(&mut self, map_parameters: &MapParameters) {
        const MIN_AREA_SIZE: u32 = 7;

        // Get the id of the smaller area whose size < MIN_AREA_SIZE
        let small_area_id: Vec<_> = self
            .area_id_and_size
            .iter()
            .filter_map(|(&id, &size)| (size < MIN_AREA_SIZE).then_some(id))
            .collect();

        small_area_id.into_iter().for_each(|current_area_id| {
            let tiles_in_current_area = self
                .iter_tiles()
                .filter(|tile| tile.area_id(self) == current_area_id)
                .collect::<Vec<_>>();

            let first_tile = tiles_in_current_area[0];
            // Check if the current area is water
            let current_area_is_water = first_tile.terrain_type(self) == TerrainType::Water;

            // Get the border tiles of the current area

            // Border tiles are the tiles that are not part of the area, but are adjacent to it.
            // That means these tiles don't belong to the area, but they surround the area.
            // Using BTreeSet to store the border tiles will make sure the tiles are processed in the same order every time.
            // That means that we can get the same 'surround_area_size_and_id' every time.
            let mut border_tiles = BTreeSet::new();

            tiles_in_current_area.iter().for_each(|&tile| {
                // Get the neighbor tiles of the current tile
                let neighbor_tiles = tile.neighbor_tiles(map_parameters);
                // Get the neighbor tiles that don't belong to the current area and add them to the border tile list
                neighbor_tiles.into_iter().for_each(|neighbor_tile| {
                    let neighbor_tile_is_water =
                        neighbor_tile.terrain_type(self) == TerrainType::Water;
                    // The neigbor tile is border tile if it meets the following conditions:
                    // 1. If the current area is water the neighbor tile is water, or if the current area is land the neighbor tile is land.
                    // 2. The neighbor tile doesn't belong to the current area.
                    if current_area_is_water == neighbor_tile_is_water
                        && !tiles_in_current_area.contains(&neighbor_tile)
                    {
                        border_tiles.insert(neighbor_tile);
                    }
                });
            });

            // Get area ID and size of the surround area
            // Notice: `surround_area_size_and_id` may have the same element
            let surround_area_size_and_id: Vec<(i32, u32)> = border_tiles
                .iter()
                .map(|tile| {
                    let area_id = tile.area_id(self);
                    let area_size = self.area_id_and_size[&area_id];
                    (area_id, area_size)
                })
                .collect();

            // Get the area ID and size of the largest surround area
            // Notice: `surround_area_size_and_id` may be empty when the current area is water but all surrounding tiles are land, or the current area is land but all surrounding tiles are water.
            if let Some(&(surround_area_id, surround_area_size)) = surround_area_size_and_id
                .iter()
                .max_by_key(|&(_, area_size)| area_size)
            {
                // Merge the current small area with the largest surround area when (surround_area_size >= MIN_AREA_SIZE) and (water or land area) is the same as the current area.
                if surround_area_size >= MIN_AREA_SIZE {
                    let old_area_id = first_tile.area_id(self);

                    self.area_id_and_size.remove(&old_area_id);

                    self.area_id_and_size
                        .entry(surround_area_id)
                        .and_modify(|e| *e += tiles_in_current_area.len() as u32);

                    tiles_in_current_area.iter().for_each(|&tile| {
                        self.area_id_query[tile.index()] = surround_area_id;
                    })
                }
            }
        });
    }
}
