use std::cmp::Reverse;

use std::collections::{BTreeMap, HashMap, HashSet};

use rand::prelude::SliceRandom;
use rand::Rng;

use crate::component::map_component::base_terrain::BaseTerrain;
use crate::component::map_component::feature::Feature;
use crate::component::map_component::natural_wonder::NaturalWonder;
use crate::component::map_component::terrain_type::TerrainType;
use crate::map_parameters::{MapParameters, WorldSize};
use crate::ruleset::unique::Unique;
use crate::ruleset::Ruleset;
use crate::tile::Tile;
use crate::tile_map::{Layer, TileMap};

impl TileMap {
    /// Generate natural wonders on the map.
    ///
    /// This function is like to Civ6's natural wonder generation. We edit it to fit our game which is like Civ5.
    /// # Notice
    /// - In CIV6, generating natural wonders is after generating features, before generating civilization start locations and placing city states.
    /// - In CIV5, generating natural wonders is after generating civilization start locations and before generating city states,
    /// so we should check if the tile is occupied by a civilization start location.
    pub fn place_natural_wonders(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        let world_size = map_parameters.map_size.world_size;
        // Get the number of natural wonders to place based on the world size
        let natural_wonder_target_number = get_world_natural_wonder_target_number(world_size);

        let natural_wonder_list: Vec<_> = ruleset.natural_wonders.keys().collect();

        // Replace HashMap with BTreeMap to ensure consistent order
        let mut natural_wonder_and_tile = BTreeMap::new();

        let mut area_id_and_terrain_type: HashMap<i32, HashSet<_>> = HashMap::new();

        self.iter_tiles().for_each(|tile| {
            let area_id = tile.area_id(self);
            let terrain_type = tile.terrain_type(self);
            area_id_and_terrain_type
                .entry(area_id)
                .or_default()
                .insert(terrain_type);
        });

        let only_water_terrain_type: HashSet<TerrainType> = HashSet::from([TerrainType::Water]);
        let only_mountain_terrain_type: HashSet<TerrainType> =
            HashSet::from([TerrainType::Mountain]);
        // Get all landmass ids
        // - landmasses are areas that don't have only water or only mountain tiles
        // - Filter out the areas that are only water or only mountains
        let mut landmass_id_and_size: Vec<_> = area_id_and_terrain_type
            .iter()
            .filter(|(_, terrain_types)| {
                terrain_types != &&only_water_terrain_type
                    && terrain_types != &&only_mountain_terrain_type
            })
            .map(|(&area_id, _)| (area_id, self.area_id_and_size[&area_id]))
            .collect();

        // First, sort by area_size in descending order using std::cmp::Reverse
        // If area_size is the same, sort by land_id in ascending order
        landmass_id_and_size.sort_by_key(|&(_, area_size)| (Reverse(area_size)));

        // When a natural wonder requires occupying 2 adjacent tiles,
        // we choose the current tile and one of its randomly selected adjacent tiles
        // as the location for placing the wonder.
        // This direction is the chosen adjacent tile's direction relative to the current tile.
        //
        // Notice: Now it is only used for `Great Barrier Reef`,
        //         in original game, neighbor_tile_direction is not randomly selected.
        //         it is always Direction::SouthEast.
        let neighbor_tile_direction = *map_parameters
            .edge_direction_array()
            .choose(&mut self.random_number_generator)
            .expect("Failed to choose a random direction");

        for tile in self.iter_tiles() {
            for &natural_wonder_name in &natural_wonder_list {
                let possible_natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                match natural_wonder_name.as_str() {
                    "Great Barrier Reef" => {
                        if let Some(neighbor_tile) =
                            tile.neighbor_tile(neighbor_tile_direction, map_parameters)
                        {
                            let mut all_neigbor_tiles = HashSet::new();

                            all_neigbor_tiles.extend(tile.neighbor_tiles(map_parameters));
                            all_neigbor_tiles.extend(neighbor_tile.neighbor_tiles(map_parameters));

                            // We only check neighbors of the current tile and the neighbor tile.
                            // So we remove them from the set of all neighbor tiles.
                            all_neigbor_tiles.remove(&tile);
                            all_neigbor_tiles.remove(&neighbor_tile);

                            // The tile should meet the following conditions:
                            // 1. All neighboring tiles exist
                            // 2. All neighboring tiles are water and not lake, not ice
                            // 3. At least 4 neighboring tiles are coast
                            if all_neigbor_tiles.len() == 8
                                && all_neigbor_tiles.iter().all(|&tile| {
                                    tile.terrain_type(self) == TerrainType::Water
                                        && tile.base_terrain(self) != BaseTerrain::Lake
                                        && tile.feature(self) != Some(Feature::Ice)
                                })
                                && all_neigbor_tiles
                                    .iter()
                                    .filter(|tile| tile.base_terrain(self) == BaseTerrain::Coast)
                                    .count()
                                    >= 4
                            {
                                natural_wonder_and_tile
                                    .entry(natural_wonder_name)
                                    .or_insert_with(Vec::new)
                                    .push(tile);
                            }
                        }
                    }
                    _ => {
                        if tile.is_freshwater(self, map_parameters)
                            != possible_natural_wonder.is_fresh_water
                        {
                            continue;
                        };

                        if !possible_natural_wonder
                            .occurs_on_type
                            .contains(&tile.terrain_type(self))
                            || !possible_natural_wonder
                                .occurs_on_base
                                .contains(&tile.base_terrain(self))
                        {
                            continue;
                        }

                        let check_unique_conditions =
                            possible_natural_wonder.uniques.iter().all(|unique| {
                                let unique = Unique::new(unique);
                                match unique.placeholder_text.as_str() {
                                    "Must be adjacent to [] [] tiles" => {
                                        let count = tile
                                            .neighbor_tiles(map_parameters)
                                            .iter()
                                            .filter(|tile| {
                                                self.matches_wonder_filter(
                                                    **tile,
                                                    unique.params[1].as_str(),
                                                )
                                            })
                                            .count();
                                        count == unique.params[0].parse::<usize>().unwrap()
                                    }
                                    "Must be adjacent to [] to [] [] tiles" => {
                                        let count = tile
                                            .neighbor_tiles(map_parameters)
                                            .iter()
                                            .filter(|tile| {
                                                self.matches_wonder_filter(
                                                    **tile,
                                                    unique.params[2].as_str(),
                                                )
                                            })
                                            .count();
                                        count >= unique.params[0].parse::<usize>().unwrap()
                                            && count <= unique.params[1].parse::<usize>().unwrap()
                                    }
                                    "Must not be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile isn't on the landmass with the given index
                                        !landmass_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| id == tile.area_id(self))
                                    }
                                    "Must be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile is on the landmass with the given index
                                        landmass_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| id == tile.area_id(self))
                                    }
                                    _ => true,
                                }
                            });
                        // end check unique conditions

                        if check_unique_conditions {
                            natural_wonder_and_tile
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push(tile);
                        }
                    }
                }
            }
        }

        // Get the natural wonders that can be placed
        // NOTICE: Because `natural_wonder_and_tile` is BTreeMap, so the order of the keys is sorted.
        let mut selected_natural_wonder_list: Vec<_> =
            natural_wonder_and_tile.keys().cloned().collect();

        // Sort the natural wonders by the number of tiles they can be placed
        // In CIV5, the natural wonders with lesser number of tiles will be placed first.
        selected_natural_wonder_list
            .sort_by_key(|natural_wonder| natural_wonder_and_tile[natural_wonder].len());

        // Store current how many natural wonders have been placed
        let mut j = 0;
        // Store the tile where the natural wonder has been placed
        let mut placed_natural_wonder_tiles: Vec<Tile> = Vec::new();

        // start to place wonder
        selected_natural_wonder_list
            .into_iter()
            .for_each(|natural_wonder_name| {
                if j < natural_wonder_target_number {
                    let tiles = natural_wonder_and_tile
                        .get_mut(natural_wonder_name)
                        .unwrap();

                    tiles.shuffle(&mut self.random_number_generator);

                    for &tile in tiles.iter() {
                        if self.layer_data[Layer::NaturalWonder][tile.index()] == 0 {
                            let natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                            // At first, we should remove feature from the tile
                            self.feature_query[tile.index()] = None;

                            match natural_wonder_name.as_str() {
                                "Great Barrier Reef" => {
                                    // The neighbor tile absolutely exists because we have checked it before.
                                    let neighbor_tile = tile
                                        .neighbor_tile(neighbor_tile_direction, map_parameters)
                                        .expect("Neighbor tile does not exist");

                                    // All related tiles should contain:
                                    // 1. Current tile
                                    // 2. Neighbor tile according to neighbor_tile_direction
                                    // 3. All neighbor tiles of current tile and neighbor tile
                                    let mut all_related_tiles = HashSet::new();

                                    all_related_tiles.extend(tile.neighbor_tiles(map_parameters));
                                    all_related_tiles
                                        .extend(neighbor_tile.neighbor_tiles(map_parameters));

                                    all_related_tiles.into_iter().for_each(|tile| {
                                        self.terrain_type_query[tile.index()] = TerrainType::Water;
                                        self.base_terrain_query[tile.index()] = BaseTerrain::Coast;
                                    });
                                    // place the natural wonder on the candidate position and its adjacent tile
                                    self.natural_wonder_query[tile.index()] = Some(
                                        NaturalWonder::NaturalWonder(natural_wonder_name.clone()),
                                    );
                                    self.natural_wonder_query[neighbor_tile.index()] = Some(
                                        NaturalWonder::NaturalWonder(natural_wonder_name.clone()),
                                    );
                                    // add the position of the placed natural wonder to the list of placed natural wonder positions
                                    placed_natural_wonder_tiles.push(tile);
                                    placed_natural_wonder_tiles.push(neighbor_tile);
                                }
                                "Rock of Gibraltar" => {
                                    let neighbor_tiles: Vec<_> =
                                        tile.neighbor_tiles(map_parameters);

                                    neighbor_tiles.into_iter().for_each(|neighbor_tile| {
                                        if neighbor_tile.terrain_type(self) == TerrainType::Water {
                                            self.base_terrain_query[neighbor_tile.index()] =
                                                BaseTerrain::Coast;
                                        } else {
                                            self.terrain_type_query[neighbor_tile.index()] =
                                                TerrainType::Mountain;
                                        }
                                    });
                                    // Edit the choice tile's terrain_type to match the natural wonder
                                    self.terrain_type_query[tile.index()] = TerrainType::Flatland;
                                    // Edit the choice tile's base_terrain to match the natural wonder
                                    self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
                                    // place the natural wonder on the candidate position
                                    self.natural_wonder_query[tile.index()] = Some(
                                        NaturalWonder::NaturalWonder(natural_wonder_name.clone()),
                                    );
                                    // add the position of the placed natural wonder to the list of placed natural wonder positions
                                    placed_natural_wonder_tiles.push(tile);
                                }
                                _ => {
                                    // Edit the choice tile's terrain_type to match the natural wonder
                                    if let Some(turn_into_terrain_type) =
                                        natural_wonder.turns_into_type
                                    {
                                        self.terrain_type_query[tile.index()] =
                                            turn_into_terrain_type;
                                    };
                                    // Edit the choice tile's base_terrain to match the natural wonder
                                    if let Some(turn_into_base_terrain) =
                                        natural_wonder.turns_into_base
                                    {
                                        self.base_terrain_query[tile.index()] =
                                            turn_into_base_terrain;
                                    }
                                    // place the natural wonder on the candidate position
                                    self.natural_wonder_query[tile.index()] = Some(
                                        NaturalWonder::NaturalWonder(natural_wonder_name.clone()),
                                    );
                                    // add the position of the placed natural wonder to the list of placed natural wonder positions
                                    placed_natural_wonder_tiles.push(tile);
                                }
                            }

                            self.place_resource_impact(
                                map_parameters,
                                tile,
                                Layer::NaturalWonder,
                                map_parameters.map_size.height as u32 / 5,
                            );
                            self.place_resource_impact(map_parameters, tile, Layer::Strategic, 1);
                            self.place_resource_impact(map_parameters, tile, Layer::Luxury, 1);
                            self.place_resource_impact(map_parameters, tile, Layer::Bonus, 1);
                            self.place_resource_impact(map_parameters, tile, Layer::CityState, 1);
                            self.place_resource_impact(map_parameters, tile, Layer::Marble, 1);

                            self.player_collision_data[tile.index()] = true;

                            j += 1;
                            break;
                        }
                    }
                }
            });

        // If the natural wonder is not a lake, and it has water neighbors, then change the water neighbor tiles to lake or coast.
        placed_natural_wonder_tiles.iter().for_each(|&tile| {
            if tile.terrain_type(self) != TerrainType::Water
                && tile
                    .neighbor_tiles(map_parameters)
                    .iter()
                    .any(|neighbor_tile| neighbor_tile.terrain_type(self) == TerrainType::Water)
            {
                let water_neighbor_tiles: Vec<_> = tile
                    .neighbor_tiles(map_parameters)
                    .into_iter()
                    .filter(|&neighbor_tile| neighbor_tile.terrain_type(self) == TerrainType::Water)
                    .collect();

                water_neighbor_tiles
                    .iter()
                    .for_each(|&water_neighbor_tile| {
                        let neighbor_neighbor_tiles =
                            water_neighbor_tile.neighbor_tiles(map_parameters);

                        if neighbor_neighbor_tiles
                            .iter()
                            .any(|&neighbor_neighbor_tile| {
                                neighbor_neighbor_tile.base_terrain(self) == BaseTerrain::Lake
                            })
                        {
                            self.base_terrain_query[water_neighbor_tile.index()] =
                                BaseTerrain::Lake;
                        } else {
                            self.base_terrain_query[water_neighbor_tile.index()] =
                                BaseTerrain::Coast;
                        };
                    });
            }
        });
    }

    /// Generate natural wonders on the map.
    ///
    /// This function is likely to Civ6's natural wonder generation. SO we don't use this function for the current game which is more like Civ5.
    /// # Notice
    /// - In CIV6, generating natural wonders is after generating features, before generating civilization start locations and placing city states.
    /// so we don't need to check if the tile is occupied by a civilization start location.
    /// -In CIV5, generating natural wonders is after generating civilization start locations and before generating city states,
    /// so we should check if the tile is occupied by a civilization start location.
    pub fn generate_natural_wonders(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        let world_size = map_parameters.map_size.world_size;
        // Get the number of natural wonders to place based on the world size
        let natural_wonder_target_number = get_world_natural_wonder_target_number(world_size);

        let natural_wonder_list: Vec<_> = ruleset.natural_wonders.keys().collect();

        // Replace HashMap with BTreeMap to ensure consistent order
        let mut natural_wonder_and_tile_and_score = BTreeMap::new();

        let mut area_id_and_terrain_type: HashMap<i32, HashSet<_>> = HashMap::new();

        self.iter_tiles().for_each(|tile| {
            let area_id = tile.area_id(self);
            let terrain_type = tile.terrain_type(self);
            area_id_and_terrain_type
                .entry(area_id)
                .or_default()
                .insert(terrain_type);
        });

        let only_water_terrain_type: HashSet<TerrainType> = HashSet::from([TerrainType::Water]);
        let only_mountain_terrain_type: HashSet<TerrainType> =
            HashSet::from([TerrainType::Mountain]);
        // Get all landmass ids
        // - landmasses are areas that don't have only water or only mountain tiles
        // - Filter out the areas that are only water or only mountains
        let mut landmass_id_and_size: Vec<_> = area_id_and_terrain_type
            .iter()
            .filter(|(_, terrain_types)| {
                terrain_types != &&only_water_terrain_type
                    && terrain_types != &&only_mountain_terrain_type
            })
            .map(|(&area_id, _)| (area_id, self.area_id_and_size[&area_id]))
            .collect();

        // First, sort by area_size in descending order using std::cmp::Reverse
        // If area_size is the same, sort by land_id in ascending order
        landmass_id_and_size.sort_by_key(|&(_, area_size)| (Reverse(area_size)));

        // When a natural wonder requires occupying 2 adjacent tiles,
        // we choose the current tile and one of its randomly selected adjacent tiles
        // as the location for placing the wonder.
        // This direction is the chosen adjacent tile's direction relative to the current tile.
        //
        // Notice: Now it is only used for `Great Barrier Reef`,
        //         in original game, neighbor_tile_direction is not randomly selected.
        //         it is always Direction::SouthEast.
        let neighbor_tile_direction = *map_parameters
            .edge_direction_array()
            .choose(&mut self.random_number_generator)
            .expect("Failed to choose a random direction");

        for tile in self.iter_tiles() {
            for &natural_wonder_name in &natural_wonder_list {
                let possible_natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                match natural_wonder_name.as_str() {
                    "Great Barrier Reef" => {
                        if let Some(neighbor_tile) =
                            tile.neighbor_tile(neighbor_tile_direction, map_parameters)
                        {
                            let mut all_neigbor_tiles = HashSet::new();

                            all_neigbor_tiles.extend(tile.neighbor_tiles(map_parameters));
                            all_neigbor_tiles.extend(neighbor_tile.neighbor_tiles(map_parameters));

                            // We only check neighbors of the current tile and the neighbor tile.
                            // So we remove them from the set of all neighbor tiles.
                            all_neigbor_tiles.remove(&tile);
                            all_neigbor_tiles.remove(&neighbor_tile);

                            // The tile should meet the following conditions:
                            // 1. All neighboring tiles exist
                            // 2. All neighboring tiles are water and not lake, not ice
                            // 3. At least 4 neighboring tiles are coast
                            if all_neigbor_tiles.len() == 8
                                && all_neigbor_tiles.iter().all(|&tile| {
                                    tile.terrain_type(self) == TerrainType::Water
                                        && tile.base_terrain(self) != BaseTerrain::Lake
                                        && tile.feature(self) != Some(Feature::Ice)
                                })
                                && all_neigbor_tiles
                                    .iter()
                                    .filter(|tile| tile.base_terrain(self) == BaseTerrain::Coast)
                                    .count()
                                    >= 4
                            {
                                natural_wonder_and_tile_and_score
                                    .entry(natural_wonder_name)
                                    .or_insert_with(Vec::new)
                                    .push((tile, 1));
                            }
                        }
                    }
                    _ => {
                        if tile.is_freshwater(self, map_parameters)
                            != possible_natural_wonder.is_fresh_water
                        {
                            continue;
                        };

                        if !possible_natural_wonder
                            .occurs_on_type
                            .contains(&tile.terrain_type(self))
                            || !possible_natural_wonder
                                .occurs_on_base
                                .contains(&tile.base_terrain(self))
                        {
                            continue;
                        }

                        let check_unique_conditions =
                            possible_natural_wonder.uniques.iter().all(|unique| {
                                let unique = Unique::new(unique);
                                match unique.placeholder_text.as_str() {
                                    "Must be adjacent to [] [] tiles" => {
                                        let count = tile
                                            .neighbor_tiles(map_parameters)
                                            .iter()
                                            .filter(|tile| {
                                                self.matches_wonder_filter(
                                                    **tile,
                                                    unique.params[1].as_str(),
                                                )
                                            })
                                            .count();
                                        count == unique.params[0].parse::<usize>().unwrap()
                                    }
                                    "Must be adjacent to [] to [] [] tiles" => {
                                        let count = tile
                                            .neighbor_tiles(map_parameters)
                                            .iter()
                                            .filter(|tile| {
                                                self.matches_wonder_filter(
                                                    **tile,
                                                    unique.params[2].as_str(),
                                                )
                                            })
                                            .count();
                                        count >= unique.params[0].parse::<usize>().unwrap()
                                            && count <= unique.params[1].parse::<usize>().unwrap()
                                    }
                                    "Must not be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile isn't on the landmass with the given index
                                        !landmass_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| id == tile.area_id(self))
                                    }
                                    "Must be on [] largest landmasses" => {
                                        // index is the ranking of the current landmass among all landmasses sorted by size from highest to lowest.
                                        let index = unique.params[0].parse::<usize>().unwrap();
                                        // Check if the tile is on the landmass with the given index
                                        landmass_id_and_size
                                            .get(index)
                                            .map_or(false, |&(id, _)| id == tile.area_id(self))
                                    }
                                    _ => true,
                                }
                            });
                        // end check unique conditions

                        if check_unique_conditions {
                            natural_wonder_and_tile_and_score
                                .entry(natural_wonder_name)
                                .or_insert_with(Vec::new)
                                .push((tile, 1));
                        }
                    }
                }
            }
        }

        // Get the natural wonders that can be placed
        // NOTICE: Because `natural_wonder_and_tile_and_score` is BTreeMap, so the order of the keys is sorted.
        let mut selected_natural_wonder_list: Vec<_> =
            natural_wonder_and_tile_and_score.keys().cloned().collect();

        // Shuffle the list that we can choose natural wonder randomly
        // NOTICE: It is different from CIV5.
        selected_natural_wonder_list.shuffle(&mut self.random_number_generator);

        // Store current how many natural wonders have been placed
        let mut j = 0;
        // Store the tile where the natural wonder has been placed
        let mut placed_natural_wonder_tiles: Vec<Tile> = Vec::new();

        // start to place wonder
        selected_natural_wonder_list
            .into_iter()
            .for_each(|natural_wonder_name| {
                if j < natural_wonder_target_number {
                    // For every natural wonder, give a score to the position where the natural wonder can place.
                    // The score is related to the min value of the distance from the position to all the placed natural wonders
                    // If no natural wonder has placed, we choose the random place where the current natural wonder can place for the current natural wonder

                    // the score method start
                    let tile_and_score = natural_wonder_and_tile_and_score
                        .get_mut(natural_wonder_name)
                        .unwrap();
                    for (tile_x_index, score) in tile_and_score.iter_mut() {
                        let closest_natural_wonder_dist = placed_natural_wonder_tiles
                            .iter()
                            .map(|tile_y_index| {
                                let position_x_hex = tile_x_index.to_hex_coordinate(map_parameters);
                                let position_y_hex = tile_y_index.to_hex_coordinate(map_parameters);
                                position_x_hex.distance_to(position_y_hex)
                            })
                            .min()
                            .unwrap_or(1000000);
                        *score = if closest_natural_wonder_dist <= 10 {
                            100 * closest_natural_wonder_dist
                        } else {
                            1000 + (closest_natural_wonder_dist - 10)
                        } + self.random_number_generator.gen_range(0..100);
                    }
                    // the score method end

                    // choose the max score position as the candidate position for the current natural wonder
                    let max_score_tile = tile_and_score
                        .iter()
                        .max_by_key(|&(_, score)| score)
                        .map(|&(index, _)| index)
                        .unwrap();

                    if !placed_natural_wonder_tiles.contains(&max_score_tile) {
                        let natural_wonder = &ruleset.natural_wonders[natural_wonder_name];

                        // At first, we should remove feature from the tile
                        self.feature_query[max_score_tile.index()] = None;

                        match natural_wonder_name.as_str() {
                            "Great Barrier Reef" => {
                                // The neighbor tile absolutely exists because we have checked it before.
                                let neighbor_tile = max_score_tile
                                    .neighbor_tile(neighbor_tile_direction, map_parameters)
                                    .expect("Neighbor tile does not exist");

                                // All related tiles should contain:
                                // 1. Current tile (`max_score_tile``)
                                // 2. Neighbor tile according to `neighbor_tile_direction`
                                // 3. All neighbor tiles of current tile and neighbor tile
                                let mut all_related_tiles = HashSet::new();

                                all_related_tiles
                                    .extend(max_score_tile.neighbor_tiles(map_parameters));
                                all_related_tiles
                                    .extend(neighbor_tile.neighbor_tiles(map_parameters));

                                all_related_tiles.into_iter().for_each(|tile| {
                                    self.terrain_type_query[tile.index()] = TerrainType::Water;
                                    self.base_terrain_query[tile.index()] = BaseTerrain::Coast;
                                });

                                // place the natural wonder on the candidate position and its adjacent tile
                                self.natural_wonder_query[max_score_tile.index()] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                self.natural_wonder_query[neighbor_tile.index()] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tiles.push(max_score_tile);
                                placed_natural_wonder_tiles.push(neighbor_tile);
                            }
                            "Rock of Gibraltar" => {
                                let neighbor_tiles: Vec<_> =
                                    max_score_tile.neighbor_tiles(map_parameters);

                                neighbor_tiles.into_iter().for_each(|neighbor_tile| {
                                    if neighbor_tile.terrain_type(self) == TerrainType::Water {
                                        self.base_terrain_query[neighbor_tile.index()] =
                                            BaseTerrain::Coast;
                                    } else {
                                        self.terrain_type_query[neighbor_tile.index()] =
                                            TerrainType::Mountain;
                                    }
                                });
                                // Edit the choice tile's terrain_type to match the natural wonder
                                self.terrain_type_query[max_score_tile.index()] =
                                    TerrainType::Flatland;
                                // Edit the choice tile's base_terrain to match the natural wonder
                                self.base_terrain_query[max_score_tile.index()] =
                                    BaseTerrain::Grassland;
                                // place the natural wonder on the candidate position
                                self.natural_wonder_query[max_score_tile.index()] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tiles.push(max_score_tile);
                            }
                            _ => {
                                // Edit the choice tile's terrain_type to match the natural wonder
                                if let Some(turn_into_terrain_type) = natural_wonder.turns_into_type
                                {
                                    self.terrain_type_query[max_score_tile.index()] =
                                        turn_into_terrain_type;
                                };
                                // Edit the choice tile's base_terrain to match the natural wonder
                                if let Some(turn_into_base_terrain) = natural_wonder.turns_into_base
                                {
                                    self.base_terrain_query[max_score_tile.index()] =
                                        turn_into_base_terrain;
                                }
                                // place the natural wonder on the candidate position
                                self.natural_wonder_query[max_score_tile.index()] =
                                    Some(NaturalWonder::NaturalWonder(natural_wonder_name.clone()));
                                // add the position of the placed natural wonder to the list of placed natural wonder positions
                                placed_natural_wonder_tiles.push(max_score_tile);
                            }
                        }
                        j += 1;
                    }
                }
            });

        // If the natural wonder is not a lake, and it has water neighbors, then change the water neighbor tiles to lake or coast.
        placed_natural_wonder_tiles.iter().for_each(|&tile| {
            if tile.terrain_type(self) != TerrainType::Water
                && tile
                    .neighbor_tiles(map_parameters)
                    .iter()
                    .any(|neighbor_tile| neighbor_tile.terrain_type(self) == TerrainType::Water)
            {
                let water_neighbor_tiles: Vec<_> = tile
                    .neighbor_tiles(map_parameters)
                    .into_iter()
                    .filter(|&neighbor_tile| neighbor_tile.terrain_type(self) == TerrainType::Water)
                    .collect();

                water_neighbor_tiles
                    .iter()
                    .for_each(|&water_neighbor_tile| {
                        let neighbor_neighbor_tiles =
                            water_neighbor_tile.neighbor_tiles(map_parameters);

                        if neighbor_neighbor_tiles
                            .iter()
                            .any(|&neighbor_neighbor_tile| {
                                neighbor_neighbor_tile.base_terrain(self) == BaseTerrain::Lake
                            })
                        {
                            self.base_terrain_query[water_neighbor_tile.index()] =
                                BaseTerrain::Lake;
                        } else {
                            self.base_terrain_query[water_neighbor_tile.index()] =
                                BaseTerrain::Coast;
                        };
                    });
            }
        });
    }

    fn matches_wonder_filter(&self, tile: Tile, filter: &str) -> bool {
        let terrain_type = tile.terrain_type(self);
        let base_terrain = tile.base_terrain(self);
        let feature = tile.feature(self);

        match filter {
            "Elevated" => matches!(terrain_type, TerrainType::Mountain | TerrainType::Hill),
            "Land" => terrain_type != TerrainType::Water,
            _ => {
                terrain_type.name() == filter
                    || base_terrain.name() == filter
                    || feature.map_or(false, |f| f.name() == filter)
            }
        }
    }
}

/// Get how many natural wonders should be placed on the map based on the world size.
fn get_world_natural_wonder_target_number(world_size: WorldSize) -> u32 {
    match world_size {
        WorldSize::Duel => 2,
        WorldSize::Tiny => 3,
        WorldSize::Small => 4,
        WorldSize::Standard => 5,
        WorldSize::Large => 6,
        WorldSize::Huge => 7,
    }
}
