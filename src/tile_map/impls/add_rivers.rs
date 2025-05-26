use rand::{seq::SliceRandom, Rng};

use crate::{
    component::map_component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    grid::{direction::Direction, hex_grid::hex::HexOrientation},
    map_parameters::HexGrid,
    tile::Tile,
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    pub fn add_rivers(&mut self, map_parameters: &MapParameters) {
        let grid = map_parameters.grid;

        let river_source_range_default = 4;
        let sea_water_range_default = 3;
        // tiles_per_river_edge specifies the number of tiles required before a river edge can appear.
        // When tiles_per_river_edge is set to 12, it indicates that for every 12 tiles, there can be 1 river edge.
        const TILES_PER_RIVER_EDGE: u32 = 12;

        (0..4).for_each(|index| {
            let (river_source_range, sea_water_range) = if index <= 1 {
                (river_source_range_default, sea_water_range_default)
            } else {
                (
                    (river_source_range_default / 2),
                    (sea_water_range_default / 2),
                )
            };

            self.iter_tiles().for_each(|tile| {
                let terrain_type = tile.terrain_type(self);
                let area_id = tile.area_id(self);
                // Check if the tile can be a river starting location.
                let pass_condition = match index {
                    0 => {
                        // Mountain and Hill are the 1st priority for river starting locations.
                        matches!(terrain_type, TerrainType::Mountain | TerrainType::Hill)
                    }
                    1 => {
                        // Land tiles that are not near the coast are the 2nd priority for river starting locations.
                        terrain_type != TerrainType::Water
                            && !tile.is_coastal_land(self, grid)
                            && self.random_number_generator.gen_range(0..8) == 0
                    }
                    2 => {
                        // If there are still not enough rivers generated, the algorithm should run again using Mountain and Hill as the river starting locations.
                        let num_tiles = self.area_list[area_id as usize].size;
                        let num_river_edges = self.river_edge_count(area_id);
                        matches!(terrain_type, TerrainType::Mountain | TerrainType::Hill)
                            && (num_river_edges <= num_tiles / TILES_PER_RIVER_EDGE)
                    }
                    3 => {
                        // At last if there are still not enough rivers generated, the algorithm should run again using any Land tiles as the river starting locations.
                        let num_tiles = self.area_list[area_id as usize].size;
                        let num_river_edges = self.river_edge_count(area_id);
                        terrain_type != TerrainType::Water
                            && (num_river_edges <= num_tiles / TILES_PER_RIVER_EDGE)
                    }
                    _ => unreachable!(),
                };

                // Tile should meet these conditions:
                // 1. It should meet the pass condition
                // 2. It should be not a natural wonder
                // 3. It should not be adjacent to a natural wonder
                // 4. all tiles around it in a given distance `river_source_range` (including self) should be not fresh water
                // 5. all tiles around it in a given distance `sea_water_range` (including self) should be not water
                if pass_condition
                    && tile.natural_wonder(self).is_none()
                    && !tile
                        .neighbor_tiles(grid)
                        .iter()
                        .any(|neighbor_tile| neighbor_tile.natural_wonder(self).is_some())
                    && !tile
                        .tiles_in_distance(river_source_range, grid)
                        .iter()
                        .any(|tile| tile.is_freshwater(self, grid))
                    && !tile
                        .tiles_in_distance(sea_water_range, grid)
                        .iter()
                        .any(|tile| tile.terrain_type(self) == TerrainType::Water)
                {
                    let start_tile = self.get_inland_corner(tile, map_parameters);
                    if let Some(start_tile) = start_tile {
                        self.do_river(start_tile, None, map_parameters);
                    }
                }
            });
        });
    }

    /// This function is called to create a river.
    ///
    /// # Arguments
    /// * `start_tile` - The tile where the river starts.
    /// * `original_flow_direction` - The original flow direction of the river.
    /// This is the original flow direction at the start of the river.
    /// * `map_parameters` - The map parameters.
    ///
    /// # Notice
    /// In original CIV5, the end of the river is water or the edge of the map.
    /// In this function, we have not implemented that the river flows the edge of the map yet.
    /// That because when we implement it, we should concern the map parameters.
    /// For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
    /// In original CIV5, we only need to consider the case where the map is WrapX and the hex is pointy.
    fn do_river(
        &mut self,
        start_tile: Tile,
        original_flow_direction: Option<Direction>,
        map_parameters: &MapParameters,
    ) {
        let grid = map_parameters.grid;
        // This array contains the list of tuples.
        // In this tuple, the elemment means as follows:
        // 1. The first element indicates the next possible flow direction of the river.
        // 2. The second element represents the direction of a neighboring tile relative to the current tile.
        //    We evaluate the weight value of these neighboring tiles using a certain algorithm and select the minimum one to determine the next flow direction of the river.
        let flow_direction_and_neighbor_tile_direction = match grid.hex_layout.orientation {
            HexOrientation::Pointy => [
                (Direction::North, Direction::NorthWest),
                (Direction::NorthEast, Direction::NorthEast),
                (Direction::SouthEast, Direction::East),
                (Direction::South, Direction::SouthWest),
                (Direction::SouthWest, Direction::West),
                (Direction::NorthWest, Direction::NorthWest),
            ],
            HexOrientation::Flat => [
                (Direction::East, Direction::NorthEast),
                (Direction::SouthEast, Direction::South),
                (Direction::SouthWest, Direction::SouthWest),
                (Direction::West, Direction::NorthWest),
                (Direction::NorthWest, Direction::NorthWest),
                (Direction::NorthEast, Direction::North),
            ],
        };

        /************ Do river start ************/

        // If the start plot have a river, exit the function
        // That will also prevent the river from forming a loop
        if self
            .river_list
            .iter()
            .flatten()
            .any(|&(tile, _)| tile == start_tile)
        {
            return;
        }

        // Create a new river and add it to the river list
        self.river_list.push(Vec::new());
        // Get the new river ID which is the index of the new river in the river list
        let river_id = self.river_list.len() - 1;

        let mut start_tile = start_tile;
        let mut original_flow_direction = original_flow_direction;
        let mut this_flow_direction = None;

        loop {
            let mut river_tile;
            if let Some(this_flow_direction) = this_flow_direction {
                match grid.hex_layout.orientation {
                    HexOrientation::Pointy => match this_flow_direction {
                        Direction::East | Direction::West => unreachable!(),
                        Direction::North => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::NorthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthWest,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                } else {
                                    river_tile = neighbor_tile;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::NorthEast => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::East, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::East,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthWest,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::SouthEast => {
                            if let Some(neighbor_tile) =
                                start_tile.neighbor_tile(Direction::East, grid)
                            {
                                river_tile = neighbor_tile
                            } else {
                                break;
                            };
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                            if let Some(neighbor_tile2) =
                                river_tile.neighbor_tile(Direction::SouthWest, grid)
                            {
                                if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile2.has_river_in_direction(
                                        Direction::East,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::South => {
                            if let Some(neighbor_tile) =
                                start_tile.neighbor_tile(Direction::SouthWest, grid)
                            {
                                river_tile = neighbor_tile
                            } else {
                                break;
                            };
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                            if let Some(neighbor_tile2) =
                                river_tile.neighbor_tile(Direction::East, grid)
                            {
                                if neighbor_tile2.has_river_in_direction(
                                    Direction::SouthWest,
                                    self,
                                    grid,
                                ) {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::SouthWest => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthWest, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::East,
                                        self,
                                        grid,
                                    )
                                    || river_tile.has_river_in_direction(
                                        Direction::SouthWest,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::NorthWest => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::West, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::East,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                } else {
                                    river_tile = neighbor_tile;
                                }
                            } else {
                                break;
                            }
                        }
                    },
                    HexOrientation::Flat => match this_flow_direction {
                        Direction::North | Direction::South => unreachable!(),
                        Direction::NorthEast => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::NorthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::NorthEast,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::South,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::East => {
                            if let Some(neighbor_tile) =
                                start_tile.neighbor_tile(Direction::NorthEast, grid)
                            {
                                river_tile = neighbor_tile
                            } else {
                                break;
                            };
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                            if let Some(neighbor_tile2) =
                                river_tile.neighbor_tile(Direction::South, grid)
                            {
                                if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile2.has_river_in_direction(
                                        Direction::NorthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::SouthEast => {
                            if let Some(neighbor_tile) =
                                start_tile.neighbor_tile(Direction::South, grid)
                            {
                                river_tile = neighbor_tile
                            } else {
                                break;
                            };
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthEast, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                            if let Some(neighbor_tile2) =
                                river_tile.neighbor_tile(Direction::NorthEast, grid)
                            {
                                if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile2.has_river_in_direction(
                                        Direction::South,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::SouthWest => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::South, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || river_tile.has_river_in_direction(
                                        Direction::South,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::NorthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::West => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::SouthWest, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::NorthEast,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                } else {
                                    river_tile = neighbor_tile;
                                }
                            } else {
                                break;
                            }
                        }
                        Direction::NorthWest => {
                            river_tile = start_tile;
                            self.river_list[river_id].push((river_tile, this_flow_direction));
                            if let Some(neighbor_tile) =
                                river_tile.neighbor_tile(Direction::North, grid)
                            {
                                if neighbor_tile.terrain_type(self) == TerrainType::Water
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::South,
                                        self,
                                        grid,
                                    )
                                    || neighbor_tile.has_river_in_direction(
                                        Direction::SouthEast,
                                        self,
                                        grid,
                                    )
                                {
                                    break;
                                } else {
                                    river_tile = neighbor_tile;
                                }
                            } else {
                                break;
                            }
                        }
                    },
                }
            } else {
                river_tile = start_tile;
            }

            if river_tile.terrain_type(self) == TerrainType::Water {
                break;
            }

            // Get next possible flow direction and relative neighbor tile iterator to calculate the best flow direction.
            let next_flow_direction_and_neighbor_tile_iter =
                flow_direction_and_neighbor_tile_direction
                    .into_iter()
                    .filter_map(|(flow_direction, direction)| {
                        // 1. If `this_flow_direction` is None, we can chooose 6 directions as the next flow direction.
                        // 2. If `this_flow_direction` is not None, we can choose at most 2 directions as the next flow direction.
                        //    The next flow direction should not be the opposite of the original flow direction.
                        if this_flow_direction.map_or(true, |this_flow_direction: Direction| {
                            next_flow_directions(this_flow_direction, grid)
                                .contains(&flow_direction)
                                && Some(flow_direction.opposite()) != original_flow_direction
                        }) {
                            river_tile
                                .neighbor_tile(direction, grid)
                                .map(|neighbor_tile| (flow_direction, neighbor_tile))
                        } else {
                            None
                        }
                    });

            // We always choose flow direction with the lowest value.
            let mut best_flow_direction = None;

            let mut best_value = i32::MAX;
            next_flow_direction_and_neighbor_tile_iter.for_each(
                |(flow_direction, neighbor_tile)| {
                    let mut value = self.river_value_at_tile(map_parameters, neighbor_tile);
                    // That will make `flow_direction` equal to `original_flow_direction` is more likely to be preferred.
                    if Some(flow_direction) == original_flow_direction {
                        value = (value * 3) / 4;
                    }
                    if value < best_value {
                        best_value = value;
                        best_flow_direction = Some(flow_direction);
                    }
                },
            );

            /* Tackle with the situation when river flows to the edge of map */

            // That will run when best_flow_direction is None.
            // When the river flows to the edge of the map, `flow_direction_and_neighbor_tile` will be empty,
            //  in this case, best_flow_direction will be None.

            /* TODO: This code handles the situation when the river flows to the edge of the map,
            but we have not implemented this part yet, so we will ignore it here.
            When we implement it, we should concern the map parameters.
            For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
            */

            /* End tackle with the situation when river flows to the edge of map */

            if best_flow_direction != None {
                original_flow_direction = original_flow_direction.or(best_flow_direction);
                start_tile = river_tile;
                this_flow_direction = best_flow_direction;
            } else {
                break;
            }
        }
        /************ Do river End ************/
        // Remove the river if it is empty
        if self.river_list[river_id].is_empty() {
            self.river_list.pop();
        }
    }

    /// Returns the value representing the suitability of flow direction for a river according to the tile.
    ///
    /// The lower the value, the more suitable the flow direction is.
    fn river_value_at_tile(&mut self, map_parameters: &MapParameters, tile: Tile) -> i32 {
        fn tile_elevation(tile_map: &TileMap, tile: Tile) -> i32 {
            match tile.terrain_type(tile_map) {
                TerrainType::Mountain => 4,
                TerrainType::Hill => 3,
                TerrainType::Water => 2,
                TerrainType::Flatland => 1,
            }
        }

        let grid = map_parameters.grid;

        // Check if the tile itself or any of its neighboring tiles are natural wonders.
        if tile.natural_wonder(self).is_some()
            || tile
                .neighbor_tiles(grid)
                .iter()
                .any(|&neighbor_tile| neighbor_tile.natural_wonder(self).is_some())
        {
            return -1;
        }

        let mut sum = tile_elevation(self, tile) * 20;

        let neighbor_tiles = tile.neighbor_tiles(grid);

        // Usually, the tile have 6 neighbors. If not, the sum increases by 40 for each missing neighbor of the tile.
        sum += 40 * (6 - neighbor_tiles.len() as i32);

        neighbor_tiles.iter().for_each(|&neighbor_tile| {
            sum += tile_elevation(self, neighbor_tile);
            if neighbor_tile.base_terrain(self) == BaseTerrain::Desert {
                sum += 4;
            }
        });

        sum += self.random_number_generator.gen_range(0..10);
        sum
    }

    /// Retrieves an inland corner tile based on the provided tile and map parameters.
    ///
    /// An inland corner is defined as a tile that has all its neighboring tiles in specific directions
    /// (0 to 3) not being water. The function will first collect the current tile and its neighbors
    /// located in specified edge directions (3 to 5), then filter out those that do not qualify
    /// as inland corners.
    ///
    /// # Parameters
    /// - `tile`: The current tile.
    /// - `map_parameters`: Parameters that define the map, including terrain types and edge directions.
    ///
    /// # Returns
    /// An `Option<TileIndex>`, which will be `Some(TileIndex)` if an inland corner is found,
    /// or `None` if no such corner exists.
    fn get_inland_corner(&mut self, tile: Tile, map_parameters: &MapParameters) -> Option<Tile> {
        let grid = map_parameters.grid;
        // We choose current tile and its `map_parameters.edge_direction_array()[3..6]` neighbors as the candidate inland corners

        // Initialize a list with the current tile
        let mut tile_list = vec![tile];

        // Collect valid neighbor tiles in edge directions [3..6]
        tile_list.extend(
            grid.edge_direction_array()[3..6]
                .iter()
                .filter_map(|&direction| tile.neighbor_tile(direction, grid)),
        );

        // Retain only those tiles that qualify as inland corners
        // An inland corner requires all neighbors in edge directions [0..3] to exist and not be water
        tile_list.retain(|&tile| {
            grid.edge_direction_array()[0..3].iter().all(|&direction| {
                let neighbor_tile = tile.neighbor_tile(direction, grid);
                if let Some(neighbor_tile) = neighbor_tile {
                    neighbor_tile.terrain_type(self) != TerrainType::Water
                } else {
                    false
                }
            })
        });

        // Choose a random corner if any exist
        tile_list.choose(&mut self.random_number_generator).copied()
    }

    /// Returns the number of river edges in the current area according to `area_id`
    fn river_edge_count(&self, current_area_id: usize) -> u32 {
        self.river_list
            .iter()
            .flatten()
            .filter(|(tile, _)| tile.area_id(self) == current_area_id)
            .count() as u32
    }
}

/// Returns the next possible flow directions of the river based on the current flow direction.
///
/// # Parameters
/// - `flow_direction`: The current direction of the river flow.
/// - `map_parameters`: A reference to the map parameters that include hex layout information.
///
/// # Returns
/// An array containing two `Direction` values:
/// - The first element represents the flow direction after a clockwise turn.
/// - The second element represents the flow direction after a counterclockwise turn.
fn next_flow_directions(flow_direction: Direction, grid: HexGrid) -> [Direction; 2] {
    let hex_orientation = grid.hex_layout.orientation;
    [
        hex_orientation.corner_clockwise(flow_direction), // turn_right_flow_direction
        hex_orientation.corner_counter_clockwise(flow_direction), // turn_left_flow_direction
    ]
}
