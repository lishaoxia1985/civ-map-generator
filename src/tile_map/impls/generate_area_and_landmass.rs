use std::collections::{HashSet, VecDeque};

use crate::{ruleset::Ruleset, tile::Tile, tile_component::TerrainType, tile_map::TileMap};

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

        let grid = self.world_grid.grid;
        let height = grid.size.height;
        let width = grid.size.width;

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

            let tiles_in_area = self.generate_tile_in_area_or_landmass(tile, check_tile);

            let current_area_id = area_list.len();
            let area_size = tiles_in_area.len() as u32;

            if area_size >= MIN_AREA_SIZE {
                let area = Area {
                    is_water: tile.is_water(self),
                    is_mountain: tile.terrain_type(self) == TerrainType::Mountain,
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

            let tiles_in_area = self.generate_tile_in_area_or_landmass(tile, check_tile);

            let area_size = tiles_in_area.len() as u32;

            // Merge single-plot mountains / ice with the surrounding area
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
                        area_id_list[neighbor.index()] != UNINITIALIZED_AREA_ID
                            && neighbor.is_water(self) == tile.is_water(self)
                    })
                    .map(|neighbor| area_id_list[neighbor.index()])
                    .max_by_key(|&area_id| area_list[area_id].size);

                if let Some(largest_neighbor_area_id) = largest_neighbor_area_id {
                    // Merge the current small area with the largest neighbor area
                    // and update the area ID of the tiles in the current area.
                    area_list[largest_neighbor_area_id].size += area_size;

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

            let area = Area {
                is_water: tile.is_water(self),
                is_mountain: tile.terrain_type(self) == TerrainType::Mountain,
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

            let tiles_in_landmass = self.generate_tile_in_area_or_landmass(tile, check_tile);

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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Area {
    /// Whether all tiles in the area are [`TerrainType::Water`].
    pub is_water: bool,
    /// Whether all tiles in the area are [`TerrainType::Mountain`].
    pub is_mountain: bool,
    /// Area ID. The ID is equal to the index of the area in the [`TileMap::area_list`].
    pub id: usize,
    /// Size of the area in tiles.
    pub size: u32,
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
