use std::collections::{BTreeSet, VecDeque};

use crate::{
    MapParameters,
    ruleset::{Ruleset, enums::TerrainType},
    tile::Tile,
    tile_map::TileMap,
};
use bitflags::bitflags;

pub const UNINITIALIZED_AREA_ID: usize = usize::MAX;
pub const UNINITIALIZED_LANDMASS_ID: usize = usize::MAX;

impl TileMap {
    /// Recalculates Area and Landmass in the map.
    ///
    /// This function is called when the map is generated or when the [`TerrainType`] of certain tiles changes.
    pub fn recalculate_areas(&mut self, map_parameters: &MapParameters) {
        self.calculate_areas(map_parameters);
        self.calculate_landmasses();
    }

    fn calculate_areas(&mut self, map_parameters: &MapParameters) {
        const MIN_AREA_SIZE: u32 = 7;

        let grid = self.world_grid.grid;
        let height = grid.size.height;
        let width = grid.size.width;

        let ruleset = &map_parameters.ruleset;

        let size = (height * width) as usize;

        // Define the area id for each tile and initialize it to `UNINITIALIZED_AREA_ID`.
        // `UNINITIALIZED_AREA_ID` means that the tile is not part of any area.
        let mut area_id_list = vec![UNINITIALIZED_AREA_ID; size];
        // Define area list and initialize it to an empty vector.
        // Each area's ID is its index in the vector.
        let mut area_list = Vec::new();

        // Check if the current tile has the same impassable state and water state as the before tile.
        // And then check their common neighbors to see if they have the same impassable state and same water state as the before tile.
        // If they do, add the current tile to the area.
        let check_tile = |tile: Tile, before_tile: Tile| {
            // Check if both tiles have the same terrain properties
            if tile.is_impassable(self, ruleset) != before_tile.is_impassable(self, ruleset)
                || tile.is_water(self) != before_tile.is_water(self)
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
                neighbor.is_impassable(self, ruleset) == before_tile.is_impassable(self, ruleset)
                    && tile.is_water(self) == before_tile.is_water(self)
            })
        };

        // First iterate, wide area
        for tile in self.all_tiles() {
            // If the tile is already part of an area, skip it.
            if area_id_list[tile.index()] != UNINITIALIZED_AREA_ID {
                continue;
            }

            let tiles_in_area = self.flood_fill_connected_tiles(tile, check_tile);

            let current_area_id = area_list.len();
            let area_size = tiles_in_area.len() as u32;

            if area_size >= MIN_AREA_SIZE {
                let area_flags = match tile.terrain_type(self) {
                    TerrainType::Water => AreaFlags::Water,
                    TerrainType::Mountain => AreaFlags::Mountain,
                    TerrainType::Flatland | TerrainType::Hill => AreaFlags::FlatlandOrHill,
                };

                let area = Area {
                    area_flags,
                    id: current_area_id,
                    size: area_size,
                };

                area_list.push(area);

                tiles_in_area.iter().for_each(|&tile| {
                    area_id_list[tile.index()] = current_area_id;
                });
            }
        }

        // Check if the current tile has the same impassable and water properties as the before tile. If so, add it to the area.
        let check_tile = |tile: Tile, before_tile: Tile| {
            // Check if both tiles have the same terrain properties
            tile.is_impassable(self, ruleset) == before_tile.is_impassable(self, ruleset)
                && tile.is_water(self) == before_tile.is_water(self)
        };

        // Second iterate, all the rest, small and thin area
        for tile in self.all_tiles() {
            // If the tile is already part of an area, skip it.
            if area_id_list[tile.index()] != UNINITIALIZED_AREA_ID {
                continue;
            }

            let tiles_in_area = self.flood_fill_connected_tiles(tile, check_tile);

            let area_size = tiles_in_area.len() as u32;

            // Merge single-tile mountains / ice with the surrounding area
            if area_size < MIN_AREA_SIZE {
                let largest_neighbor_area_id = tiles_in_area
                    .iter()
                    .flat_map(|&tile| tile.neighbor_tiles(grid))
                    .filter(|neighbor| {
                        area_id_list[neighbor.index()] != UNINITIALIZED_AREA_ID
                            && neighbor.is_water(self) == tile.is_water(self)
                    })
                    .map(|neighbor| area_id_list[neighbor.index()])
                    .max_by_key(|&area_id| area_list[area_id].size);

                if let Some(largest_neighbor_area_id) = largest_neighbor_area_id {
                    // Merge the current small area with the largest neighbor area
                    // and update the area ID of the tiles in the current area.
                    area_list[largest_neighbor_area_id].size += area_size;

                    // It often happens that the largest neighbor area is flatland and hill area,
                    // and the current area is a small pure mountain area.
                    // In this case, we merge the current area into the largest neighbor area,
                    // and update the area flags of the latest merged area containing the flags of the current area.
                    area_list[largest_neighbor_area_id].area_flags |= match tile.terrain_type(self)
                    {
                        TerrainType::Water => AreaFlags::Water,
                        TerrainType::Mountain => AreaFlags::Mountain,
                        TerrainType::Flatland | TerrainType::Hill => AreaFlags::FlatlandOrHill,
                    };

                    for tile in &tiles_in_area {
                        area_id_list[tile.index()] = largest_neighbor_area_id;
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
            let current_area_id = area_list.len();

            let area_flags = match tile.terrain_type(self) {
                TerrainType::Water => AreaFlags::Water,
                TerrainType::Mountain => AreaFlags::Mountain,
                TerrainType::Flatland | TerrainType::Hill => AreaFlags::FlatlandOrHill,
            };

            let area = Area {
                area_flags,
                id: current_area_id,
                size: area_size,
            };

            area_list.push(area);

            for tile in tiles_in_area {
                area_id_list[tile.index()] = current_area_id;
            }
        }

        // Update the area ID list and area list
        self.area_id_list = area_id_list;
        self.area_list = area_list;
    }

    fn calculate_landmasses(&mut self) {
        let height = self.world_grid.size().height;
        let width = self.world_grid.size().width;

        let size = (height * width) as usize;

        // Initialize the landmass_id_query with `UNINITIALIZED_LANDMASS_ID`.
        // `UNINITIALIZED_LANDMASS_ID` means that the tile is not part of any landmass.
        let mut landmass_id_list = vec![UNINITIALIZED_LANDMASS_ID; size];
        let mut landmass_list = Vec::new();

        // Check if the current tile has the same water status as the previous tile.
        // If it does, it means that the current tile is part of the same landmass as the previous tile.
        let check_tile =
            |tile: Tile, before_tile: Tile| tile.is_water(self) == before_tile.is_water(self);

        for tile in self.all_tiles() {
            // If the tile is already part of a landmass, skip it.
            if landmass_id_list[tile.index()] != UNINITIALIZED_LANDMASS_ID {
                continue;
            }

            let tiles_in_landmass = self.flood_fill_connected_tiles(tile, check_tile);

            let landmass_type = if tile.is_water(self) {
                LandmassType::Water
            } else {
                LandmassType::Land
            };

            let current_landmass_id = landmass_list.len();
            let landmass_size = tiles_in_landmass.len() as u32;

            let landmass = Landmass {
                landmass_type,
                id: current_landmass_id,
                size: landmass_size,
            };

            landmass_list.push(landmass);

            tiles_in_landmass.iter().for_each(|&tile| {
                landmass_id_list[tile.index()] = current_landmass_id;
            });
        }

        // Update the landmass ID list and landmass list.
        self.landmass_id_list = landmass_id_list;
        self.landmass_list = landmass_list;
    }

    /// Performs a flood-fill algorithm to collect all connected tiles that satisfy a given condition.
    ///
    /// This function starts from `start_tile` and explores all neighboring tiles using breadth-first search (BFS).
    /// It uses the `check_tile` closure to determine whether a neighbor tile should be included in the result set.
    /// The function continues expanding until no more qualifying neighbors can be found.
    ///
    /// # Arguments
    ///
    /// - `start_tile`: The initial tile from which to begin the flood-fill operation.
    /// - `check_tile`: A closure that takes two tiles (neighbor tile, current tile) and returns true if the neighbor
    ///   should be included in the area/landmass. This allows custom logic for connectivity checks.
    ///
    /// # Returns
    ///
    /// A `BTreeSet` containing all tiles that are part of the connected component starting from `start_tile`.
    /// The set is ordered, ensuring consistent iteration order.
    ///
    /// # Algorithm
    ///
    /// 1. Initialize with the start tile in both the result set and the processing queue.
    /// 2. While the queue is not empty:
    ///    - Dequeue a tile and examine all its neighbors.
    ///    - For each neighbor, check if it satisfies the condition AND hasn't been visited yet.
    ///    - If both conditions are met, add it to the result set and enqueue it for further exploration.
    fn flood_fill_connected_tiles(
        &self,
        start_tile: Tile,
        check_tile: impl Fn(Tile, Tile) -> bool,
    ) -> BTreeSet<Tile> {
        let grid = self.world_grid.grid;

        // Collection to store all tiles that belong to the current connected component (area or landmass).
        // Using BTreeSet ensures ordered iteration and prevents duplicate insertions.
        let mut connected_tiles = BTreeSet::new();

        // Queue for BFS traversal. Contains tiles whose neighbors still need to be examined.
        let mut queue = VecDeque::new();

        // Initialize: Add the starting tile to both the result set and the processing queue.
        connected_tiles.insert(start_tile);
        queue.push_back(start_tile);

        // BFS loop: Process each tile in the queue until all reachable tiles have been explored.
        while let Some(current_tile) = queue.pop_front() {
            // Examine all neighboring tiles of the current tile.
            current_tile.neighbor_tiles(grid).for_each(|tile| {
                // IMPORTANT: The order of conditions matters!
                // 1. First check if the neighbor satisfies the connectivity condition.
                // 2. Then attempt to insert it into the set (returns false if already present).
                // This prevents re-processing tiles that have already been visited.
                if check_tile(tile, current_tile) && connected_tiles.insert(tile) {
                    // Neighbor qualifies and is new, so add it to the queue for further exploration.
                    queue.push_back(tile);
                }
            });
        }

        // Return all tiles that form the connected component.
        connected_tiles
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Area {
    /// Area flags. See [`AreaFlags`] for details.
    pub area_flags: AreaFlags,
    /// Area ID. The ID is equal to the index of the area in the [`TileMap::area_list`].
    pub id: usize,
    /// Size of the area in tiles.
    pub size: u32,
}

bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct AreaFlags: u32 {
        /// This implies that all tiles in the area are water.
        ///
        /// # Note
        ///
        /// This flag is mutually exclusive with `Mountain` and `FlatlandOrHill`.
        const Water = 1 << 0;
        /// This implies that all tiles in the area are mountain.
        ///
        /// # Note
        ///
        /// This flag is mutually exclusive with `Water`.
        const Mountain = 1 << 1;
        /// This implies that all tiles in the area are flatland, hill, or mixed flatland and hill.
        ///
        /// # Note
        ///
        /// This flag is mutually exclusive with `Water`.
        const FlatlandOrHill = 1 << 2;
    }
}

/// Represents a landmass in the map.
/// A landmass is a contiguous area of land or water on the map.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Landmass {
    /// Landmass ID. The ID is equal to the index of the landmass in the [`TileMap::landmass_list`].
    pub id: usize,
    /// Size of the landmass in tiles.
    pub size: u32,
    /// The type of the landmass.
    pub landmass_type: LandmassType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the type of landmass.
pub enum LandmassType {
    /// All tiles in the landmass are land, land includes [`TerrainType::Flatland`], [`TerrainType::Hill`] and [`TerrainType::Mountain`].
    Land,
    /// All tiles in the landmass are [`TerrainType::Water`].
    Water,
}
