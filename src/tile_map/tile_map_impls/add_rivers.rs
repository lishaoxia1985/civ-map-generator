use rand::{seq::SliceRandom, Rng};

use crate::{
    component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    grid::{hex::HexOrientation, Direction},
    tile_map::{tile::Tile, MapParameters, TileMap},
};

impl TileMap {
    pub fn add_rivers(&mut self, map_parameters: &MapParameters) {
        let river_source_range_default = 4;
        let sea_water_range_default = 3;
        // tiles_per_river_edge specifies the number of tiles required before a river edge can appear.
        // When tiles_per_river_edge is set to 12, it indicates that for every 12 tiles, there can be 1 river edge.
        const TILES_PER_RIVER_EDGE: u32 = 12;

        let mut river_id = 0;

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
                let pass_condition = match index {
                    0 => {
                        // Mountain and Hill are the 1st priority for river starting locations.
                        matches!(
                            tile.terrain_type(self),
                            TerrainType::Mountain | TerrainType::Hill
                        )
                    }
                    1 => {
                        // Land tiles that are not near the coast are the 2nd priority for river starting locations.
                        tile.terrain_type(self) != TerrainType::Water
                            && !tile.is_coastal_land(self, map_parameters)
                            && self.random_number_generator.gen_range(0..8) == 0
                    }
                    2 => {
                        // If there are still not enough rivers generated, the algorithm should run again using Mountain and Hill as the river starting locations.
                        let num_tiles = self
                            .area_id_query
                            .iter()
                            .filter(|area_id| **area_id == tile.area_id(self))
                            .count() as u32;
                        let num_river_edges = self.river_edge_count(tile.area_id(self));
                        matches!(
                            tile.terrain_type(self),
                            TerrainType::Mountain | TerrainType::Hill
                        ) && (num_river_edges <= num_tiles / TILES_PER_RIVER_EDGE)
                    }
                    3 => {
                        // At last if there are still not enough rivers generated, the algorithm should run again using any Land tiles as the river starting locations.
                        let num_tiles = self
                            .area_id_query
                            .iter()
                            .filter(|area_id| **area_id == tile.area_id(self))
                            .count() as u32;
                        let num_river_edges = self.river_edge_count(tile.area_id(self));
                        tile.terrain_type(self) != TerrainType::Water
                            && (num_river_edges <= num_tiles / TILES_PER_RIVER_EDGE)
                    }
                    _ => panic!("Invalid index"),
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
                        .neighbor_tiles(map_parameters)
                        .iter()
                        .any(|neighbor_tile| neighbor_tile.natural_wonder(self).is_some())
                    && !tile
                        .tiles_in_distance(river_source_range, map_parameters)
                        .iter()
                        .any(|tile| tile.is_freshwater(self, map_parameters))
                    && !tile
                        .tiles_in_distance(sea_water_range, map_parameters)
                        .iter()
                        .any(|tile| tile.terrain_type(self) == TerrainType::Water)
                {
                    let start_tile = self.get_inland_corner(tile, map_parameters);
                    if let Some(start_tile) = start_tile {
                        self.do_river(
                            start_tile,
                            Direction::None,
                            Direction::None,
                            river_id,
                            map_parameters,
                        );
                        river_id += 1;
                    }
                }
            });
        });
    }

    /// This function is called to create a river.
    ///
    /// # Notice
    /// In original Civ V, the end of the river is water or the edge of the map.
    /// In this function, we have not implemented that the river flows the edge of the map yet.
    /// That because when we implement it, we should concern the map parameters.
    /// For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
    /// In original Civ V, we only need to consider the case where the map is WrapX and the hex is pointy.
    fn do_river(
        &mut self,
        start_tile: Tile,
        this_flow_direction: Direction,
        original_flow_direction: Direction,
        river_id: i32,
        map_parameters: &MapParameters,
    ) {
        // This array contains the list of tuples.
        // In this tuple, the elemment means as follows:
        // 1. The first element indicates the next possible flow direction of the river.
        // 2. The second element represents the direction of a neighboring tile relative to the current tile.
        //    We evaluate the weight value of these neighboring tiles using a certain algorithm and select the minimum one to determine the next flow direction of the river.
        let flow_direction_and_neighbor_tile_direction = match map_parameters.hex_layout.orientation
        {
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
            .values()
            .flatten()
            .any(|&(tile, _)| tile == start_tile)
        {
            return;
        }

        let mut start_tile = start_tile;
        let mut this_flow_direction = this_flow_direction;
        let mut original_flow_direction = original_flow_direction;

        loop {
            let mut river_tile;
            let mut best_flow_direction = Direction::None;
            match map_parameters.hex_layout.orientation {
                HexOrientation::Pointy => match this_flow_direction {
                    Direction::East | Direction::West => unreachable!(),
                    Direction::North => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::NorthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthWest,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile = neighbor_tile;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthEast => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::East, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::East,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthWest,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthEast => {
                        if let Some(neighbor_tile) =
                            start_tile.neighbor_tile(Direction::East, map_parameters)
                        {
                            river_tile = neighbor_tile
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_tile2) =
                            river_tile.neighbor_tile(Direction::SouthWest, map_parameters)
                        {
                            if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                || neighbor_tile2.has_river_in_direction(
                                    Direction::East,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::South => {
                        if let Some(neighbor_tile) =
                            start_tile.neighbor_tile(Direction::SouthWest, map_parameters)
                        {
                            river_tile = neighbor_tile
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_tile2) =
                            river_tile.neighbor_tile(Direction::East, map_parameters)
                        {
                            if neighbor_tile2.has_river_in_direction(
                                Direction::SouthWest,
                                self,
                                map_parameters,
                            ) {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthWest => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthWest, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || neighbor_tile.has_river_in_direction(
                                    Direction::East,
                                    self,
                                    map_parameters,
                                )
                                || river_tile.has_river_in_direction(
                                    Direction::SouthWest,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthWest => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::West, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || neighbor_tile.has_river_in_direction(
                                    Direction::East,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile = neighbor_tile;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::None => {
                        river_tile = start_tile;
                    }
                },
                HexOrientation::Flat => match this_flow_direction {
                    Direction::North | Direction::South => unreachable!(),
                    Direction::NorthEast => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::NorthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::South,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::East => {
                        if let Some(neighbor_tile) =
                            start_tile.neighbor_tile(Direction::NorthEast, map_parameters)
                        {
                            river_tile = neighbor_tile
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_tile2) =
                            river_tile.neighbor_tile(Direction::South, map_parameters)
                        {
                            if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                || neighbor_tile2.has_river_in_direction(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthEast => {
                        if let Some(neighbor_tile) =
                            start_tile.neighbor_tile(Direction::South, map_parameters)
                        {
                            river_tile = neighbor_tile
                        } else {
                            return;
                        };
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthEast, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                        if let Some(neighbor_tile2) =
                            river_tile.neighbor_tile(Direction::NorthEast, map_parameters)
                        {
                            if neighbor_tile2.terrain_type(self) == TerrainType::Water
                                || neighbor_tile2.has_river_in_direction(
                                    Direction::South,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::SouthWest => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::South, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || river_tile.has_river_in_direction(
                                    Direction::South,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::West => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::SouthWest, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || neighbor_tile.has_river_in_direction(
                                    Direction::NorthEast,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile = neighbor_tile;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::NorthWest => {
                        river_tile = start_tile;
                        self.river_list
                            .entry(river_id)
                            .or_default()
                            .push((river_tile, this_flow_direction));
                        if let Some(neighbor_tile) =
                            river_tile.neighbor_tile(Direction::North, map_parameters)
                        {
                            if neighbor_tile.terrain_type(self) == TerrainType::Water
                                || neighbor_tile.has_river_in_direction(
                                    Direction::South,
                                    self,
                                    map_parameters,
                                )
                                || neighbor_tile.has_river_in_direction(
                                    Direction::SouthEast,
                                    self,
                                    map_parameters,
                                )
                            {
                                return;
                            } else {
                                river_tile = neighbor_tile;
                            }
                        } else {
                            return;
                        }
                    }
                    Direction::None => {
                        river_tile = start_tile;
                    }
                },
            }

            if river_tile.terrain_type(self) == TerrainType::Water {
                return;
            }

            fn river_value_at_tile(
                tile: Tile,
                tile_map: &mut TileMap,
                map_parameters: &MapParameters,
            ) -> i32 {
                fn tile_elevation(tile: Tile, tile_map: &TileMap) -> i32 {
                    match tile.terrain_type(tile_map) {
                        TerrainType::Mountain => 4,
                        TerrainType::Hill => 3,
                        TerrainType::Water => 2,
                        TerrainType::Flatland => 1,
                    }
                }

                // Check if the tile itself or any of its neighboring tiles are natural wonders.
                if tile.natural_wonder(tile_map).is_some()
                    || tile
                        .neighbor_tiles(map_parameters)
                        .iter()
                        .any(|&neighbor_tile| neighbor_tile.natural_wonder(tile_map).is_some())
                {
                    return -1;
                }

                let mut sum = tile_elevation(tile, tile_map) * 20;

                let neighbor_tiles = tile.neighbor_tiles(map_parameters);

                // Usually, the tile have 6 neighbors. If not, the sum increases by 40 for each missing neighbor of the tile.
                sum += 40 * (6 - neighbor_tiles.len() as i32);

                neighbor_tiles.iter().for_each(|&neighbor_tile| {
                    sum += tile_elevation(neighbor_tile, tile_map);
                    if neighbor_tile.base_terrain(tile_map) == BaseTerrain::Desert {
                        sum += 4;
                    }
                });

                sum += tile_map.random_number_generator.gen_range(0..10);
                sum
            }

            // This vec contains the list of tuples.
            // In this tuple, the elemment means as follows:
            // 1. The first element indicates the next possible flow direction of the river.
            // 2. The second element represents the index of the tile neighboring to the current tile.
            //    We evaluate the weight value of these neighboring tiles using a certain algorithm and select the minimum one to determine the next flow direction of the river.
            //    The neighbor should meet the following conditions:
            //    1. The next flow direction is not the opposite of the original flow direction.
            //    2. The next flow direction is None (when this_flow_direction is `Direction::None`) or one of the possible flow directions of the current tile.
            let flow_direction_and_neighbor_tile = flow_direction_and_neighbor_tile_direction
                .into_iter()
                .filter_map(|(flow_direction, direction)| {
                    if flow_direction.opposite_direction() != original_flow_direction // The next flow direction is not the opposite of the original flow direction.
                            && (this_flow_direction == Direction::None
                                || next_flow_directions(this_flow_direction, map_parameters)
                                    .contains(&flow_direction))
                    {
                        river_tile
                            .neighbor_tile(direction, map_parameters)
                            .map(|neighbor_index| (flow_direction, neighbor_index))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if best_flow_direction == Direction::None {
                let mut best_value = i32::MAX;
                flow_direction_and_neighbor_tile.into_iter().for_each(
                    |(flow_direction, neighbor_tile)| {
                        let mut value = river_value_at_tile(neighbor_tile, self, map_parameters);
                        if flow_direction == original_flow_direction {
                            value = (value * 3) / 4;
                        }
                        if value < best_value {
                            best_value = value;
                            best_flow_direction = flow_direction;
                        }
                    },
                );
                /* Tackle with the situation when river flows to the edge of map */

                /* TODO: This code handles the situation when the river flows to the edge of the map,
                but we have not implemented this part yet, so we will ignore it here.
                When we implement it, we should concern the map parameters.
                For example, hex is Flat or Pointy, map is wrapx or not, map is wrapy or not, etc.
                */

                /* End tackle with the situation when river flows to the edge of map */
            }

            if best_flow_direction != Direction::None {
                if original_flow_direction == Direction::None {
                    original_flow_direction = best_flow_direction;
                }
                start_tile = river_tile;
                this_flow_direction = best_flow_direction;
            } else {
                return;
            }
        }
        /************ Do river End ************/
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
        // We choose current tile and its `map_parameters.edge_direction_array()[3..6]` neighbors as the candidate inland corners

        // Initialize a list with the current tile
        let mut tile_list = vec![tile];

        // Collect valid neighbor tiles in edge directions [3..6]
        tile_list.extend(
            map_parameters.edge_direction_array()[3..6]
                .iter()
                .filter_map(|&direction| tile.neighbor_tile(direction, map_parameters)),
        );

        // Retain only those tiles that qualify as inland corners
        // An inland corner requires all neighbors in edge directions [0..3] to exist and not be water
        tile_list.retain(|&tile| {
            map_parameters.edge_direction_array()[0..3]
                .iter()
                .all(|&direction| {
                    let neighbor_index = tile.neighbor_tile(direction, map_parameters);
                    if let Some(neighbor_tile) = neighbor_index {
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
    fn river_edge_count(&self, current_area_id: i32) -> u32 {
        self.river_list
            .values()
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
const fn next_flow_directions(
    flow_direction: Direction,
    map_parameters: &MapParameters,
) -> [Direction; 2] {
    let hex_orientation = map_parameters.hex_layout.orientation;
    [
        hex_orientation.corner_clockwise(flow_direction), // turn_right_flow_direction
        hex_orientation.corner_counter_clockwise(flow_direction), // turn_left_flow_direction
    ]
}
