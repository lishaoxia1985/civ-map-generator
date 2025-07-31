use std::collections::{HashSet, VecDeque};

use crate::{
    ruleset::Ruleset, tile::Tile, tile_component::terrain_type::TerrainType, tile_map::TileMap,
};

pub const UNINITIALIZED_AREA_ID: usize = usize::MAX;
pub const UNINITIALIZED_LANDMASS_ID: usize = usize::MAX;

impl TileMap {
    /// Recalculates Area and Landmass in the map.
    ///
    /// This function is called when the map is generated or when the [`TerrainType`] of certain tiles changes.
    pub fn recalculate_areas(&mut self, ruleset: &Ruleset) {
        self.calculate_areas(ruleset);
        self.calculate_landmasses();
    }

    fn calculate_areas(&mut self, ruleset: &Ruleset) {
        const MIN_AREA_SIZE: u32 = 7;

        self.area_list.clear();

        let grid = self.world_grid.grid;
        let height = grid.size.height;
        let width = grid.size.width;

        let size = (height * width) as usize;

        // Initialize the area_id_query with `UNINITIALIZED_AREA_ID`.
        // `UNINITIALIZED_AREA_ID` means that the tile is not part of any area.
        self.area_id_query = vec![UNINITIALIZED_AREA_ID; size];

        // Precompute tile properties to avoid borrowing `self` in the closure
        // `tile_impassable` is used to check if the tile is impassable or not.
        // `tile_water` is used to check if the tile is water or not.
        let (tile_impassable, tile_water): (Vec<bool>, Vec<bool>) = self
            .all_tiles()
            .map(|tile| (tile.is_impassable(self, ruleset), tile.is_water(self)))
            .unzip();

        let check_tile = |tile: Tile, before_tile: Tile| {
            let tile_idx = tile.index();
            let before_idx = before_tile.index();

            // Check if both tiles have the same terrain properties
            if tile_impassable[tile_idx] != tile_impassable[before_idx]
                || tile_water[tile_idx] != tile_water[before_idx]
            {
                return false;
            }

            // Get the neighbors of the two tiles
            let tile_neighbor_list: Vec<Tile> = tile.neighbor_tiles(grid).collect();
            let before_neighbor_list: Vec<Tile> = before_tile.neighbor_tiles(grid).collect();

            // Get the common neighbors iterator
            let mut common_neighbors_iter = tile_neighbor_list
                .iter()
                .filter(|t| before_neighbor_list.contains(t));

            // Verify all common neighbors maintain the same properties
            common_neighbors_iter.all(|&neighbor| {
                let n_idx = neighbor.index();
                tile_impassable[n_idx] == tile_impassable[before_idx]
                    && tile_water[n_idx] == tile_water[before_idx]
            })
        };

        // First iterate, wide area
        for tile in self.all_tiles() {
            // If the tile is already part of an area, skip it.
            if tile.area_id(self) != UNINITIALIZED_AREA_ID {
                continue;
            }

            let tiles_in_area = self.generate_tile_in_area_or_landmass(tile, check_tile);

            let current_area_id = self.area_list.len();
            let area_size = tiles_in_area.len() as u32;

            if area_size >= MIN_AREA_SIZE {
                let area = Area {
                    is_water: tile.is_water(self),
                    is_mountain: tile.terrain_type(self) == TerrainType::Mountain,
                    id: current_area_id,
                    size: area_size,
                };

                self.area_list.push(area);

                tiles_in_area.iter().for_each(|&tile| {
                    self.area_id_query[tile.index()] = current_area_id;
                });
            }
        }

        let check_tile = |tile: Tile, before_tile: Tile| {
            let tile_idx = tile.index();
            let before_idx = before_tile.index();

            // Check if both tiles have the same terrain properties
            tile_impassable[tile_idx] == tile_impassable[before_idx]
                && tile_water[tile_idx] == tile_water[before_idx]
        };

        // Second iterate, all the rest, small and thin area
        for tile in self.all_tiles() {
            // If the tile is already part of an area, skip it.
            if tile.area_id(self) != UNINITIALIZED_AREA_ID {
                continue;
            }

            let tiles_in_area = self.generate_tile_in_area_or_landmass(tile, check_tile);

            let area_size = tiles_in_area.len() as u32;

            //merge single-plot mountains / ice with the surrounding area
            if area_size < MIN_AREA_SIZE {
                // Convert `tiles_in_area` into a sorted vector `tiles_in_area_ordered` to ensure a consistent order,
                // that will help us to get the same largest area ID of the neighboring area each time
                // when more than one area has the same size.
                let mut tiles_in_area_ordered: Vec<_> = tiles_in_area.iter().cloned().collect();
                tiles_in_area_ordered.sort_unstable();

                let largest_neighbor_area_id = tiles_in_area_ordered
                    .iter()
                    .flat_map(|&tile| tile.neighbor_tiles(grid))
                    .filter(|neighbor| {
                        neighbor.area_id(self) != UNINITIALIZED_AREA_ID
                            && tile_water[neighbor.index()] == tile_water[tile.index()]
                    })
                    .map(|neighbor| neighbor.area_id(self))
                    .max_by_key(|&area_id| self.area_list[area_id].size);

                if let Some(largest_neighbor_area_id) = largest_neighbor_area_id {
                    // Merge the current small area with the largest neighbor area
                    // and update the area ID of the tiles in the current area.
                    self.area_list[largest_neighbor_area_id].size += area_size;

                    for tile in &tiles_in_area {
                        self.area_id_query[tile.index()] = largest_neighbor_area_id;
                    }
                    // Skip the rest of the loop since we have already merged the area
                    continue;
                }
            }

            // too large to merge or no change to merge
            // 1. If the area is too large to merge with any neighbor area,
            //    we assign a new area ID to it.
            // 2. If it is small enough, but it cannot be merged with any neighbor area,
            //    we assign a new area ID to it.
            let current_area_id = self.area_list.len();

            let area = Area {
                is_water: tile.is_water(self),
                is_mountain: tile.terrain_type(self) == TerrainType::Mountain,
                id: current_area_id,
                size: area_size,
            };

            self.area_list.push(area);

            for tile in tiles_in_area {
                self.area_id_query[tile.index()] = current_area_id;
            }
        }
    }

    fn calculate_landmasses(&mut self) {
        self.landmass_list.clear();

        let height = self.world_grid.size().height;
        let width = self.world_grid.size().width;

        let size = (height * width) as usize;

        // Initialize the landmass_id_query with `UNINITIALIZED_LANDMASS_ID`.
        // `UNINITIALIZED_LANDMASS_ID` means that the tile is not part of any landmass.
        self.landmass_id_query = vec![UNINITIALIZED_LANDMASS_ID; size];

        // Precompute tile properties to avoid borrowing `self` in the closure
        // `tile_water` is used to check if the tile is water or not.
        let tile_water: Vec<_> = self.all_tiles().map(|tile| tile.is_water(self)).collect();

        let check_tile = |tile: Tile, before_tile: Tile| {
            let tile_idx = tile.index();
            let before_idx = before_tile.index();
            tile_water[tile_idx] == tile_water[before_idx]
        };

        for tile in self.all_tiles() {
            // If the tile is already part of a landmass, skip it.
            if tile.landmass_id(self) != UNINITIALIZED_LANDMASS_ID {
                continue;
            }

            let tiles_in_landmass = self.generate_tile_in_area_or_landmass(tile, check_tile);

            let landmass_type = if tile.is_water(self) {
                LandmassType::Water
            } else {
                LandmassType::Land
            };

            let current_landmass_id = self.landmass_list.len();
            let landmass_size = tiles_in_landmass.len() as u32;

            let landmass = Landmass {
                landmass_type,
                id: current_landmass_id,
                size: landmass_size,
            };

            self.landmass_list.push(landmass);

            tiles_in_landmass.iter().for_each(|&tile| {
                self.landmass_id_query[tile.index()] = current_landmass_id;
            });
        }
    }

    fn generate_tile_in_area_or_landmass(
        &self,
        start_tile: Tile,
        check_tile: impl Fn(Tile, Tile) -> bool,
    ) -> HashSet<Tile> {
        let grid = self.world_grid.grid;

        // Store all the tiles that are part of the current area or landmass.
        let mut tiles_in_area_or_landmass = HashSet::new();
        // Store all the tiles that need to check whether their neighbors are in the current area or landmass within the following 'while {..}' loop.
        let mut queue = VecDeque::new();

        tiles_in_area_or_landmass.insert(start_tile);
        queue.push_back(start_tile);

        while let Some(current_tile) = queue.pop_front() {
            current_tile.neighbor_tiles(grid).for_each(|tile| {
                // NOTICE: Don't switch the order of `check_tile` and `tiles_in_area_or_landmass.insert(tile)`.
                if check_tile(tile, current_tile) && tiles_in_area_or_landmass.insert(tile) {
                    queue.push_back(tile);
                }
            });
        }

        tiles_in_area_or_landmass
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub is_water: bool,
    pub is_mountain: bool,
    /// Area ID. The ID is equal to the index of the area in the [`TileMap::area_list`].
    pub id: usize,
    /// Size of the area in tiles.
    pub size: u32,
}

/// Represents a landmass in the map.
/// A landmass is a contiguous area of land or water on the map.
pub struct Landmass {
    /// Landmass ID. The ID is equal to the index of the landmass in the [`TileMap::landmass_list`].
    pub id: usize,
    /// Size of the landmass in tiles.
    pub size: u32,
    pub landmass_type: LandmassType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the type of landmass.
pub enum LandmassType {
    Land,
    Water,
}
