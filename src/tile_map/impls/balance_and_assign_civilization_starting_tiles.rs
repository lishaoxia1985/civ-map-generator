use std::{cmp::max, collections::BTreeSet};

use enum_map::Enum;
use rand::{
    Rng,
    seq::{IndexedRandom, SliceRandom},
};

use crate::{
    map_parameters::{MapParameters, ResourceSetting},
    nation::Nation,
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{BaseTerrain, Feature, Resource, TerrainType},
    tile_map::{Layer, TileMap, get_major_strategic_resource_quantity_values},
};

use super::generate_regions::{RegionType, StartLocationCondition};

impl TileMap {
    // function AssignStartingPlots:BalanceAndAssign
    /// Balance and assign the starting tiles to civilizations.
    ///
    /// This function does 2 things:
    /// 1. Balance the starting tiles, such as add bonus/strategic resources, change neighbouring terrains, etc.
    ///    That will make each civilization have a fair chance to win the game.
    /// 2. Assign the starting tiles to civilizations according to civilization's bias.
    ///
    /// # Notice
    ///
    /// TODO: We have not implemented to create the team for the civilization.
    pub fn balance_and_assign_civilization_starting_tiles(
        &mut self,
        map_parameters: &MapParameters,
        ruleset: &Ruleset,
    ) {
        let civilization_list = (0..Nation::LENGTH)
            .map(Nation::from_usize)
            .filter(|&nation| {
                ruleset.nations[nation.as_str()].city_state_type.is_empty()
                    && nation != Nation::Barbarians
                    && nation != Nation::Spectator
            })
            .collect::<Vec<_>>();

        // Take the civilization randomly as the starting civilization in the map.
        let mut start_civilization_list: Vec<_> = civilization_list
            .choose_multiple(
                &mut self.random_number_generator,
                map_parameters.num_civilization as usize,
            )
            .copied()
            .collect();

        for region_index in 0..self.region_list.len() {
            let start_location_condition =
                self.normalize_civilization_starting_tile(map_parameters, region_index);
            self.region_list[region_index].start_location_condition = start_location_condition;
        }

        let disable_start_bias = false;
        // If disbable_start_bias is true, then the starting tile will be chosen randomly.
        if disable_start_bias {
            start_civilization_list.shuffle(&mut self.random_number_generator);
            self.starting_tile_and_civilization = start_civilization_list
                .iter()
                .zip(self.region_list.iter())
                .map(|(&civilization, region)| (region.starting_tile, civilization))
                .collect();
            // TODO: Set the civilization to the team in the future.
            return;
        }

        let mut num_coastal_civs_remaining = 0;
        let mut civs_needing_coastal_start = Vec::new();

        let mut _num_river_civs_remaining = 0;
        let mut civs_needing_river_start = Vec::new();

        let mut _num_priority_civs_remaining = 0;
        let mut civs_needing_region_priority = Vec::new();

        let mut _num_avoid_civs = 0;
        let mut _num_avoid_civs_remaining = 0;
        let mut civs_needing_region_avoid = Vec::new();

        // Store all the regions' indices that have not been assigned a civilization.
        // If the region index has been assigned a civilization, then it will be removed from the list.
        let mut region_index_list = (0..self.region_list.len()).collect::<BTreeSet<_>>();

        for &civilization in start_civilization_list.iter() {
            let nation_info = &ruleset.nations[civilization.as_str()];
            if nation_info.along_ocean {
                civs_needing_coastal_start.push(civilization);
            } else if nation_info.along_river {
                civs_needing_river_start.push(civilization);
            } else if !nation_info.region_type_priority.is_empty() {
                _num_priority_civs_remaining += 1;
                civs_needing_region_priority.push(civilization);
            } else if !nation_info.region_type_avoid.is_empty() {
                _num_avoid_civs += 1;
                _num_avoid_civs_remaining += 1;
                civs_needing_region_avoid.push(civilization);
            }
        }

        // Handle Coastal Start Bias
        if !civs_needing_coastal_start.is_empty() {
            let mut regions_with_coastal_start: Vec<usize> = Vec::new();
            let mut regions_with_lake_start: Vec<usize> = Vec::new();

            for &region_index in region_index_list.iter() {
                let region = &self.region_list[region_index];
                if region.start_location_condition.along_ocean {
                    regions_with_coastal_start.push(region_index);
                }
            }

            if regions_with_coastal_start.len() < civs_needing_coastal_start.len() {
                for &region_index in region_index_list.iter() {
                    let region = &self.region_list[region_index];
                    if region.start_location_condition.next_to_lake
                        && !region.start_location_condition.along_ocean
                    {
                        regions_with_lake_start.push(region_index);
                    }
                }
            }

            // Now assign those with coastal bias to start locations, where possible.
            if regions_with_coastal_start.len() + regions_with_lake_start.len() > 0 {
                civs_needing_coastal_start.shuffle(&mut self.random_number_generator);

                if !regions_with_coastal_start.is_empty() {
                    regions_with_coastal_start.shuffle(&mut self.random_number_generator);
                }

                if !regions_with_lake_start.is_empty() {
                    regions_with_lake_start.shuffle(&mut self.random_number_generator);
                }

                // If `civs_needing_coastal_start.len() > regions_with_coastal_start.len() + regions_with_lake_start.len()`,
                // that means there are not enough coastal and lake starting tiles,
                // so `num_coastal_civs_remaining = civs_needing_coastal_start.len() - (regions_with_coastal_start.len() + regions_with_lake_start.len())`,
                // if there are enough coastal and lake starting tiles, `num_coastal_civs_remaining = 0`.
                num_coastal_civs_remaining = max(
                    0,
                    civs_needing_coastal_start.len() as i32
                        - (regions_with_coastal_start.len() + regions_with_lake_start.len()) as i32,
                ) as usize;

                // Assign starting tile to civilizations with coastal bias or lake bias,
                // and remove the assigned civilizations from `civs_needing_coastal_start`.
                // When civilization should be along ocean, we assign starting tile to civilizations following these rules:
                // 1. At first, we assign starting tile to civilizations with coastal bias.
                // 2. If there are not enough coastal starting tiles, we assign starting tile to civilizations with lake bias.
                civs_needing_coastal_start
                    .drain(..civs_needing_coastal_start.len() - num_coastal_civs_remaining)
                    .zip(
                        regions_with_coastal_start
                            .iter()
                            .chain(regions_with_lake_start.iter()),
                    )
                    .for_each(|(civilization, &region_index)| {
                        self.starting_tile_and_civilization
                            .insert(self.region_list[region_index].starting_tile, civilization);
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    });
            }
        }

        // Handle River bias
        if !civs_needing_river_start.is_empty() || num_coastal_civs_remaining > 0 {
            let mut regions_with_river_start = Vec::new();
            let mut regions_with_near_river_start = Vec::new();

            for &region_index in region_index_list.iter() {
                let region = &self.region_list[region_index];
                if region.start_location_condition.is_river {
                    regions_with_river_start.push(region_index);
                }
            }

            for &region_index in region_index_list.iter() {
                let region = &self.region_list[region_index];
                if region.start_location_condition.near_river
                    && !region.start_location_condition.is_river
                {
                    regions_with_near_river_start.push(region_index);
                }
            }

            if regions_with_river_start.len() + regions_with_near_river_start.len() > 0 {
                civs_needing_river_start.shuffle(&mut self.random_number_generator);

                if !regions_with_river_start.is_empty() {
                    regions_with_river_start.shuffle(&mut self.random_number_generator);
                }

                if !regions_with_near_river_start.is_empty() {
                    regions_with_near_river_start.shuffle(&mut self.random_number_generator);
                }

                // If `civs_needing_river_start.len() > regions_with_river_start.len() + regions_with_near_river_start.len()`,
                // that means there are not enough river and near river starting tiles,
                // so `civs_needing_river_start.len() - (regions_with_river_start.len() + regions_with_near_river_start.len())`,
                // if there are enough river and near river starting tiles, `num_river_civs_remaining = 0`.
                _num_river_civs_remaining = max(
                    0,
                    civs_needing_river_start.len() as i32
                        - (regions_with_river_start.len() + regions_with_near_river_start.len())
                            as i32,
                ) as usize;

                civs_needing_river_start
                    .drain(..civs_needing_river_start.len() - _num_river_civs_remaining)
                    .zip(
                        regions_with_river_start
                            .iter()
                            .chain(regions_with_near_river_start.iter()),
                    )
                    .for_each(|(civilization, &region_index)| {
                        self.starting_tile_and_civilization
                            .insert(self.region_list[region_index].starting_tile, civilization);
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    });
            }

            // Now handle any fallbacks for unassigned coastal bias.
            if num_coastal_civs_remaining > 0
                && civs_needing_river_start.len()
                    < regions_with_river_start.len() + regions_with_near_river_start.len()
            {
                let mut fallbacks_with_river_start = Vec::new();
                let mut fallbacks_with_near_river_start = Vec::new();

                for &region_index in region_index_list.iter() {
                    let region = &self.region_list[region_index];
                    if region.start_location_condition.is_river {
                        fallbacks_with_river_start.push(region_index);
                    }
                }

                for &region_index in region_index_list.iter() {
                    let region = &self.region_list[region_index];
                    if region.start_location_condition.near_river
                        && !region.start_location_condition.is_river
                    {
                        fallbacks_with_near_river_start.push(region_index);
                    }
                }

                if fallbacks_with_river_start.len() + fallbacks_with_near_river_start.len() > 0 {
                    civs_needing_coastal_start.shuffle(&mut self.random_number_generator);

                    if !fallbacks_with_river_start.is_empty() {
                        fallbacks_with_river_start.shuffle(&mut self.random_number_generator);
                    }

                    if !fallbacks_with_near_river_start.is_empty() {
                        fallbacks_with_near_river_start.shuffle(&mut self.random_number_generator);
                    }

                    num_coastal_civs_remaining = max(
                        0,
                        civs_needing_coastal_start.len() as i32
                            - (fallbacks_with_river_start.len()
                                + fallbacks_with_near_river_start.len())
                                as i32,
                    ) as usize;

                    civs_needing_coastal_start
                        .drain(..civs_needing_coastal_start.len() - num_coastal_civs_remaining)
                        .zip(
                            fallbacks_with_river_start
                                .iter()
                                .chain(fallbacks_with_near_river_start.iter()),
                        )
                        .for_each(|(civilization, &region_index)| {
                            self.starting_tile_and_civilization
                                .insert(self.region_list[region_index].starting_tile, civilization);
                            // Remove region index that has been assigned from region index list
                            region_index_list.remove(&region_index);
                        });
                }
            }
        }

        // Handle Region Priority
        if !civs_needing_region_priority.is_empty() {
            let mut civs_needing_single_priority = Vec::new();
            let mut civs_needing_multi_priority = Vec::new();
            let mut civs_fallback_priority = Vec::new();

            for &civilization in civs_needing_region_priority.iter() {
                let nation_info = &ruleset.nations[civilization.as_str()];
                if nation_info.region_type_priority.len() == 1 {
                    civs_needing_single_priority.push(civilization);
                } else {
                    civs_needing_multi_priority.push(civilization);
                }
            }

            if !civs_needing_single_priority.is_empty() {
                // Sort civs_needing_single_priority by the first element of nation.region_type_priority
                // Notice: region_type_priority always doesn't have 'RegionType::Undefined' as the element,
                // so we don't need to tackle the case that the first element is 'RegionType::Undefined'.
                civs_needing_single_priority.sort_by_key(|&civilization| {
                    let nation_info = &ruleset.nations[civilization.as_str()];
                    nation_info.region_type_priority[0] as i32
                });

                for &civilization in civs_needing_single_priority.iter() {
                    let mut candidate_regions = Vec::new();
                    for &region_index in region_index_list.iter() {
                        let region_type_priority =
                            ruleset.nations[civilization.as_str()].region_type_priority[0];
                        if self.region_list[region_index].region_type == region_type_priority {
                            candidate_regions.push(region_index);
                        }
                    }

                    if !candidate_regions.is_empty() {
                        let region_index = *candidate_regions
                            .choose(&mut self.random_number_generator)
                            .unwrap();
                        self.starting_tile_and_civilization
                            .insert(self.region_list[region_index].starting_tile, civilization);
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    } else {
                        civs_fallback_priority.push(civilization);
                    }
                }
            }

            if !civs_needing_multi_priority.is_empty() {
                // Sort `civs_needing_multi_priority` by the length of nation.region_type_priority
                civs_needing_multi_priority.sort_by_key(|&civilization| {
                    let nation_info = &ruleset.nations[civilization.as_str()];
                    nation_info.region_type_priority.len()
                });

                for &civilization in civs_needing_multi_priority.iter() {
                    let mut candidate_regions = Vec::new();
                    for &region_index in region_index_list.iter() {
                        let region_type_priority_list =
                            &ruleset.nations[civilization.as_str()].region_type_priority;
                        if region_type_priority_list
                            .contains(&self.region_list[region_index].region_type)
                        {
                            candidate_regions.push(region_index);
                        }
                    }

                    if !candidate_regions.is_empty() {
                        let region_index = *candidate_regions
                            .choose(&mut self.random_number_generator)
                            .unwrap();
                        self.starting_tile_and_civilization
                            .insert(self.region_list[region_index].starting_tile, civilization);
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    }
                }
            }

            // Fallbacks are done (if needed) after multiple-region priority is handled. The list is pre-sorted.
            if !civs_fallback_priority.is_empty() {
                for &civilization in civs_fallback_priority.iter() {
                    let region_type_priority =
                        ruleset.nations[civilization.as_str()].region_type_priority[0];
                    let region_index = self.find_fallback_for_unmatched_region_priority(
                        region_type_priority,
                        &region_index_list,
                    );
                    if let Some(region_index) = region_index {
                        self.starting_tile_and_civilization
                            .insert(self.region_list[region_index].starting_tile, civilization);
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    }
                }
            }
        }

        // Handle Region Avoid
        if !civs_needing_region_avoid.is_empty() {
            // Sort `civs_needing_region_avoid` by the length of `nation.avoid_region_type`.
            civs_needing_region_avoid.sort_by_key(|civilization| {
                let nation_info = &ruleset.nations[civilization.as_str()];
                nation_info.region_type_avoid.len()
            });

            // process in reverse order, so most needs goes first.
            for &civilization in civs_needing_region_avoid.iter().rev() {
                let mut candidate_regions = Vec::new();
                for &region_index in region_index_list.iter() {
                    let region_type_priority_list =
                        &ruleset.nations[civilization.as_str()].region_type_priority;
                    if !region_type_priority_list
                        .contains(&self.region_list[region_index].region_type)
                    {
                        candidate_regions.push(region_index);
                    }
                }

                if !candidate_regions.is_empty() {
                    let region_index = *candidate_regions
                        .choose(&mut self.random_number_generator)
                        .unwrap();
                    self.starting_tile_and_civilization
                        .insert(self.region_list[region_index].starting_tile, civilization);
                    // Remove region index that has been assigned from region index list
                    region_index_list.remove(&region_index);
                }
            }
        }

        // Assign remaining civs to start plots.
        // Get remaining civilizations that have not been assigned a starting tile.
        let mut remaining_civilization_list: Vec<_> = start_civilization_list
            .into_iter()
            .filter(|civilization| {
                !self
                    .starting_tile_and_civilization
                    .values()
                    .any(|v| v == civilization)
            })
            .collect();

        remaining_civilization_list.shuffle(&mut self.random_number_generator);

        remaining_civilization_list
            .iter()
            .zip(region_index_list.iter())
            .for_each(|(&civilization, &region_index)| {
                self.starting_tile_and_civilization
                    .insert(self.region_list[region_index].starting_tile, civilization);
            });
        // TODO: Set the civilization to the team in the future.
    }

    // function AssignStartingPlots:FindFallbackForUnmatchedRegionPriority
    fn find_fallback_for_unmatched_region_priority(
        &self,
        region_type: RegionType,
        region_index_list: &BTreeSet<usize>,
    ) -> Option<usize> {
        let mut most_tundra = 0;
        let mut most_tundra_forest = 0;
        let mut most_jungle = 0;
        let mut most_forest = 0;
        let mut most_desert = 0;
        let mut most_hills = 0;
        let mut most_plains = 0;
        let mut most_grass = 0;
        let mut most_hybrid = 0;

        let mut best_tundra = None;
        let mut best_tundra_forest = None;
        let mut best_jungle = None;
        let mut best_forest = None;
        let mut best_desert = None;
        let mut best_hills = None;
        let mut best_plains = None;
        let mut best_grass = None;
        let mut best_hybrid = None;

        for &region_index in region_index_list.iter() {
            let region = &self.region_list[region_index];
            let terrain_statistic = &region.terrain_statistic;
            let hills_count = terrain_statistic.terrain_type_num[TerrainType::Hill];
            let peaks_count = terrain_statistic.terrain_type_num[TerrainType::Mountain];
            let grass_count = terrain_statistic.base_terrain_num[BaseTerrain::Grassland];
            let plains_count = terrain_statistic.base_terrain_num[BaseTerrain::Plain];
            let desert_count = terrain_statistic.base_terrain_num[BaseTerrain::Desert];
            let tundra_count = terrain_statistic.base_terrain_num[BaseTerrain::Tundra];
            let snow_count = terrain_statistic.base_terrain_num[BaseTerrain::Snow];
            let forest_count = terrain_statistic.feature_num[Feature::Forest];
            let jungle_count = terrain_statistic.feature_num[Feature::Jungle];
            let marsh_count = terrain_statistic.feature_num[Feature::Marsh];
            let floodplain_count = terrain_statistic.feature_num[Feature::Floodplain];
            let oasis_count = terrain_statistic.feature_num[Feature::Oasis];

            match region_type {
                RegionType::Undefined => unreachable!(),
                RegionType::Tundra => {
                    // Find fallback for Tundra priority
                    if tundra_count + snow_count > most_tundra {
                        best_tundra = Some(region_index);
                        most_tundra = tundra_count + snow_count;
                    }
                    if forest_count > most_tundra_forest && jungle_count == 0 {
                        best_tundra_forest = Some(region_index);
                        most_tundra_forest = forest_count;
                    }
                }
                RegionType::Jungle => {
                    // Find fallback for Jungle priority
                    if jungle_count > most_jungle {
                        best_jungle = Some(region_index);
                        most_jungle = jungle_count;
                    }
                }
                RegionType::Forest => {
                    // Find fallback for Forest priority
                    if forest_count > most_forest {
                        best_forest = Some(region_index);
                        most_forest = forest_count;
                    }
                }
                RegionType::Desert => {
                    // Find fallback for Desert priority
                    if desert_count + floodplain_count + oasis_count > most_desert {
                        best_desert = Some(region_index);
                        most_desert = desert_count + floodplain_count + oasis_count;
                    }
                }
                RegionType::Hill => {
                    // Find fallback for Hills priority
                    if hills_count + peaks_count > most_hills {
                        best_hills = Some(region_index);
                        most_hills = hills_count + peaks_count;
                    }
                }
                RegionType::Plain => {
                    // Find fallback for Plains priority
                    if plains_count > most_plains {
                        best_plains = Some(region_index);
                        most_plains = plains_count;
                    }
                }
                RegionType::Grassland => {
                    // Find fallback for Grass priority
                    if grass_count + marsh_count > most_grass {
                        best_grass = Some(region_index);
                        most_grass = grass_count + marsh_count;
                    }
                }
                RegionType::Hybrid => {
                    // Find fallback for Hybrid priority
                    if grass_count + plains_count > most_hybrid {
                        best_hybrid = Some(region_index);
                        most_hybrid = grass_count + plains_count;
                    }
                }
            }
        }

        match region_type {
            RegionType::Undefined => unreachable!(),
            RegionType::Tundra => best_tundra.or(best_tundra_forest),
            RegionType::Jungle => best_jungle,
            RegionType::Forest => best_forest,
            RegionType::Desert => best_desert,
            RegionType::Hill => best_hills,
            RegionType::Plain => best_plains,
            RegionType::Grassland => best_grass,
            RegionType::Hybrid => best_hybrid,
        }
    }

    // function AssignStartingPlots:NormalizeStartLocation
    /// Normalizes civilization starting tile.
    ///
    /// This function will do as follows:
    /// 1. Remove any feature Ice from 1 radius of the starting tile.
    /// 2. Add hills to the starting tile's 1 radius if it has not enough hammer.
    /// 3. Add a small `Horse` or `Iron` strategic resource to the starting tile's 2 radius if it has not enough hammer,
    ///    (it will contain forest in 1-2 radius when calculating the number of hammer).
    /// 4. If resource_setting is [`ResourceSetting::StrategicBalance`], call [`TileMap::add_strategic_balance_resources`] to add strategic resources to the starting tile's 1-3 radius.
    /// 5. Add bonus resource for compensation to city state location's 1-2 radius if it has not enough food.
    /// 6. Get information about the starting tile and its surroundings for placing the civilization.
    fn normalize_civilization_starting_tile(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> StartLocationCondition {
        let grid = self.world_grid.grid;

        let starting_tile = self.region_list[region_index].starting_tile;

        let mut inner_four_food = 0;
        let mut inner_three_food = 0;
        let mut inner_two_food = 0;
        let mut inner_hill = 0;
        let mut inner_forest = 0;
        let mut inner_one_hammer = 0;
        let mut inner_ocean = 0;

        let mut outer_four_food = 0;
        let mut outer_three_food = 0;
        let mut outer_two_food = 0;
        let mut outer_hill = 0;
        let mut outer_forest = 0;
        let mut outer_one_hammer = 0;
        let mut outer_ocean = 0;

        let mut inner_can_have_bonus = 0;
        let mut outer_can_have_bonus = 0;
        let mut inner_bad_tiles = 0;
        let mut outer_bad_tiles = 0;

        let mut num_food_bonus_needed = 0;
        let mut num_native_two_food_first_ring = 0;
        let mut num_native_two_food_second_ring = 0;

        // Remove any feature Ice from the first ring of the starting tile.
        self.clear_ice_near_city_site(starting_tile, 1);

        let mut along_ocean = false;
        let mut next_to_lake = false;
        let mut is_river = false;
        let mut near_river = false;
        let mut near_mountain = false;

        let mut forest_count = 0;
        let mut jungle_count = 0;

        let mut num_grassland = 0;
        let mut num_plain = 0;

        if starting_tile.is_coastal_land(self) {
            along_ocean = true;
        }

        if starting_tile.has_river(self) {
            is_river = true;
        }

        let mut neighbor_tile_list: Vec<Tile> = starting_tile.neighbor_tiles(grid).collect();

        neighbor_tile_list.iter().for_each(|neighbor_tile| {
            let terrain_type = neighbor_tile.terrain_type(self);
            let base_terrain = neighbor_tile.base_terrain(self);
            let feature = neighbor_tile.feature(self);
            match terrain_type {
                TerrainType::Mountain => {
                    near_mountain = true;
                    inner_bad_tiles += 1;
                }
                TerrainType::Water => {
                    if feature == Some(Feature::Ice) {
                        inner_bad_tiles += 1;
                    } else if base_terrain == BaseTerrain::Lake {
                        next_to_lake = true;
                        inner_two_food += 1;
                        num_native_two_food_first_ring += 1;
                    } else {
                        inner_ocean += 1;
                        inner_can_have_bonus += 1;
                    }
                }
                _ => {
                    if feature == Some(Feature::Jungle) {
                        jungle_count += 1;
                        num_native_two_food_first_ring += 1;
                    } else if feature == Some(Feature::Forest) {
                        forest_count += 1;
                    }

                    if neighbor_tile.has_river(self) {
                        near_river = true;
                    }

                    if terrain_type == TerrainType::Hill {
                        inner_hill += 1;
                        if feature == Some(Feature::Jungle) {
                            inner_two_food += 1;
                            inner_can_have_bonus += 1;
                        } else if feature == Some(Feature::Forest) {
                            inner_can_have_bonus += 1;
                        } else if base_terrain == BaseTerrain::Grassland {
                            num_grassland += 1;
                        } else if base_terrain == BaseTerrain::Plain {
                            num_plain += 1;
                        }
                    } else if feature == Some(Feature::Oasis) {
                        inner_three_food += 1;
                        num_native_two_food_first_ring += 1;
                    } else if neighbor_tile.is_freshwater(self) {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                inner_four_food += 1;
                                num_grassland += 1;
                                if feature != Some(Feature::Marsh) {
                                    inner_can_have_bonus += 1;
                                }
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                }
                                if feature.is_none() {
                                    num_native_two_food_first_ring += 1;
                                }
                            }
                            BaseTerrain::Desert => {
                                inner_can_have_bonus += 1;
                                if feature == Some(Feature::Floodplain) {
                                    inner_four_food += 1;
                                    num_native_two_food_first_ring += 1;
                                } else {
                                    inner_bad_tiles += 1;
                                }
                            }
                            BaseTerrain::Plain => {
                                inner_three_food += 1;
                                inner_can_have_bonus += 1;
                                num_plain += 1;
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                } else {
                                    inner_one_hammer += 1;
                                }
                            }
                            BaseTerrain::Tundra => {
                                inner_three_food += 1;
                                inner_can_have_bonus += 1;
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                }
                            }
                            BaseTerrain::Snow => {
                                inner_bad_tiles += 1;
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    } else {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                inner_three_food += 1;
                                num_grassland += 1;
                                if feature != Some(Feature::Marsh) {
                                    inner_can_have_bonus += 1;
                                }
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                }
                                if feature.is_none() {
                                    num_native_two_food_first_ring += 1;
                                }
                            }
                            BaseTerrain::Desert => {
                                inner_can_have_bonus += 1;
                                inner_bad_tiles += 1;
                            }
                            BaseTerrain::Plain => {
                                inner_two_food += 1;
                                inner_can_have_bonus += 1;
                                num_plain += 1;
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                } else {
                                    inner_one_hammer += 1;
                                }
                            }
                            BaseTerrain::Tundra => {
                                inner_can_have_bonus += 1;
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                } else {
                                    inner_bad_tiles += 1;
                                }
                            }
                            BaseTerrain::Snow => {
                                inner_bad_tiles += 1;
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    }
                }
            }
        });

        let mut tile_at_distance_two_list: Vec<Tile> =
            starting_tile.tiles_at_distance(2, grid).collect();

        tile_at_distance_two_list
            .iter()
            .for_each(|tile_at_distance_two| {
                let terrain_type = tile_at_distance_two.terrain_type(self);
                let base_terrain = tile_at_distance_two.base_terrain(self);
                let feature = tile_at_distance_two.feature(self);
                match terrain_type {
                    TerrainType::Mountain => {
                        near_mountain = true;
                        outer_bad_tiles += 1;
                    }
                    TerrainType::Water => {
                        if base_terrain == BaseTerrain::Lake {
                            next_to_lake = true;
                            if feature == Some(Feature::Ice) {
                                outer_bad_tiles += 1;
                            } else {
                                outer_two_food += 1;
                                num_native_two_food_second_ring += 1;
                            }
                        } else if feature == Some(Feature::Ice) {
                            outer_bad_tiles += 1;
                        } else {
                            outer_ocean += 1;
                            outer_can_have_bonus += 1;
                        }
                    }
                    _ => {
                        if feature == Some(Feature::Jungle) {
                            jungle_count += 1;
                            num_native_two_food_second_ring += 1;
                        } else if feature == Some(Feature::Forest) {
                            forest_count += 1;
                        }

                        if tile_at_distance_two.has_river(self) {
                            near_river = true;
                        }

                        if terrain_type == TerrainType::Hill {
                            outer_hill += 1;
                            if feature == Some(Feature::Jungle) {
                                outer_two_food += 1;
                                outer_can_have_bonus += 1;
                            } else if feature == Some(Feature::Forest) {
                                outer_can_have_bonus += 1;
                            } else if base_terrain == BaseTerrain::Grassland {
                                num_grassland += 1;
                            } else if base_terrain == BaseTerrain::Plain {
                                num_plain += 1;
                            }
                        } else if feature == Some(Feature::Oasis) {
                            outer_three_food += 1;
                            num_native_two_food_second_ring += 1;
                        } else if tile_at_distance_two.is_freshwater(self) {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    outer_four_food += 1;
                                    num_grassland += 1;
                                    if feature != Some(Feature::Marsh) {
                                        outer_can_have_bonus += 1;
                                    }
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    }
                                    if feature.is_none() {
                                        num_native_two_food_second_ring += 1;
                                    }
                                }
                                BaseTerrain::Desert => {
                                    outer_can_have_bonus += 1;
                                    if feature == Some(Feature::Floodplain) {
                                        outer_four_food += 1;
                                        num_native_two_food_second_ring += 1;
                                    } else {
                                        outer_bad_tiles += 1;
                                    }
                                }
                                BaseTerrain::Plain => {
                                    outer_three_food += 1;
                                    outer_can_have_bonus += 1;
                                    num_plain += 1;
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    } else {
                                        outer_one_hammer += 1;
                                    }
                                }
                                BaseTerrain::Tundra => {
                                    outer_three_food += 1;
                                    outer_can_have_bonus += 1;
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    }
                                }
                                BaseTerrain::Snow => {
                                    outer_bad_tiles += 1;
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        } else {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    outer_three_food += 1;
                                    num_grassland += 1;
                                    if feature != Some(Feature::Marsh) {
                                        outer_can_have_bonus += 1;
                                    }
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    }
                                    if feature.is_none() {
                                        num_native_two_food_second_ring += 1;
                                    }
                                }
                                BaseTerrain::Desert => {
                                    outer_can_have_bonus += 1;
                                    outer_bad_tiles += 1;
                                }
                                BaseTerrain::Plain => {
                                    outer_two_food += 1;
                                    outer_can_have_bonus += 1;
                                    num_plain += 1;
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    } else {
                                        outer_one_hammer += 1;
                                    }
                                }
                                BaseTerrain::Tundra => {
                                    outer_can_have_bonus += 1;
                                    if feature == Some(Feature::Forest) {
                                        outer_forest += 1;
                                    } else {
                                        outer_bad_tiles += 1;
                                    }
                                }
                                BaseTerrain::Snow => {
                                    outer_bad_tiles += 1;
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        }
                    }
                }
            });

        // Adjust the hammer situation, if needed.
        let mut inner_hammer_score = (4 * inner_hill) + (2 * inner_forest) + inner_one_hammer;
        let outer_hammer_score = (2 * outer_hill) + outer_forest + outer_one_hammer;
        let early_hammer_score =
            (2 * inner_forest) + outer_forest + inner_one_hammer + outer_one_hammer;

        // If drastic shortage of hammer, attempt to add a hill to first ring.
        if (outer_hammer_score < 8 && inner_hammer_score < 2) || inner_hammer_score == 0 {
            neighbor_tile_list.shuffle(&mut self.random_number_generator);
            for &tile in neighbor_tile_list.iter() {
                // Attempt to place a Hill at the currently chosen tile.
                let placed_hill = self.attempt_to_place_hill_at_tile(tile);
                if placed_hill {
                    inner_hammer_score += 4;
                    break;
                }
            }
        }

        // Add mandatory Iron, Horse, Oil to every start if Strategic Balance option is enabled.
        if map_parameters.resource_setting == ResourceSetting::StrategicBalance {
            self.add_strategic_balance_resources(map_parameters, region_index);
        }

        // If early hammers will be too short, attempt to add a small Horse or Iron to second ring.
        if inner_hammer_score < 3 && early_hammer_score < 6 {
            tile_at_distance_two_list.shuffle(&mut self.random_number_generator);
            for &tile in tile_at_distance_two_list.iter() {
                let placed_strategic = self.attempt_to_place_small_strategic_at_tile(tile);
                if placed_strategic {
                    break;
                }
            }
        }

        let inner_food_score = (4 * inner_four_food) + (2 * inner_three_food) + inner_two_food;
        let outer_food_score = (4 * outer_four_food) + (2 * outer_three_food) + outer_two_food;
        let total_food_score = inner_food_score + outer_food_score;
        let native_two_food_tiles =
            num_native_two_food_first_ring + num_native_two_food_second_ring;

        num_food_bonus_needed = if total_food_score < 4 && inner_food_score == 0 {
            5
        } else if total_food_score < 6 {
            4
        } else if total_food_score < 8 || (total_food_score < 12 && inner_food_score < 5) {
            3
        } else if (total_food_score < 17 && inner_food_score < 9) || native_two_food_tiles <= 1 {
            2
        } else if total_food_score < 24 && inner_food_score < 11
            || native_two_food_tiles == 2
            || num_native_two_food_first_ring == 0
            || total_food_score < 20
        {
            1
        } else {
            num_food_bonus_needed // or default value if needed
        };

        // If Legendary Start resource option is enabled, add more food bonuses needed.
        if map_parameters.resource_setting == ResourceSetting::LegendaryStart {
            num_food_bonus_needed += 2;
        }

        // If there are no tiles yielding 2 food in the first and second ring,
        // and `num_food_bonus_needed` is less than 3,
        // we will convert a plains tile to grassland to ensure at least one 2-food tile.
        // If there are no tiles to convert, we will set `num_food_bonus_needed` to 3 to compensate.
        if native_two_food_tiles == 0 && num_food_bonus_needed < 3 {
            // Find candidate tiles for conversion.
            let tile_list: Vec<Tile> = neighbor_tile_list
                .iter()
                .chain(tile_at_distance_two_list.iter())
                .filter(|tile| {
                    tile.resource(self).is_none()
                        && tile.terrain_type(self) == TerrainType::Flatland
                        && tile.base_terrain(self) == BaseTerrain::Plain
                        && tile.feature(self).is_none()
                })
                .copied()
                .collect();

            if let Some(&conversion_tile) = tile_list.choose(&mut self.random_number_generator) {
                conversion_tile.set_base_terrain(self, BaseTerrain::Grassland);
                // Forbid to place strategic resources on this tile
                self.place_impact_and_ripples(conversion_tile, Layer::Strategic, 0);
            } else {
                num_food_bonus_needed = 3;
            }
        }

        if num_food_bonus_needed > 0 {
            let _max_bonuses_possible = inner_can_have_bonus + outer_can_have_bonus;
            let mut inner_placed = 0;
            let mut outer_placed = 0;

            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tile_list.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tile_at_distance_two_list.shuffle(&mut self.random_number_generator);

            // Create a new vector to store the tiles at distance 3, and shuffle it.
            let mut tile_at_distance_three_list: Vec<Tile> =
                starting_tile.tiles_at_distance(3, grid).collect();
            tile_at_distance_three_list.shuffle(&mut self.random_number_generator);

            // Permanent flag. (We don't want to place more than one Oasis per location).
            // This is set to false after the first Oasis is placed.
            let mut allow_oasis = true;

            /* let mut first_ring_iter = neighbor_tile_list.iter().peekable();
            let mut second_ring_iter = tile_at_distance_two_list.iter().peekable();
            let mut third_ring_iter = tile_at_distance_three_list.iter().peekable();

            while num_food_bonus_needed > 0 {
                if inner_can_have_bonus > 0
                    && ((inner_placed < 2)
                        || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                            && inner_placed < 3))
                    && first_ring_iter.peek().is_some()
                {
                    // Add bonus to inner ring.
                    for &tile in first_ring_iter.by_ref() {
                        let (placed_bonus, placed_oasis) =
                            self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                        if placed_bonus {
                            if allow_oasis && placed_oasis {
                                // First oasis was placed on this pass, so change permission.
                                allow_oasis = false;
                            }
                            inner_placed += 1;
                            inner_can_have_bonus -= 1;
                            num_food_bonus_needed -= 1;
                            break;
                        }
                    }
                } else if outer_can_have_bonus > 0
                    && ((inner_placed + outer_placed < 5)
                        || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                            && inner_placed + outer_placed < 4))
                    && second_ring_iter.peek().is_some()
                {
                    // Add bonus to second ring.
                    for &tile in second_ring_iter.by_ref() {
                        let (placed_bonus, placed_oasis) =
                            self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                        if placed_bonus {
                            if allow_oasis && placed_oasis {
                                // First oasis was placed on this pass, so change permission.
                                allow_oasis = false;
                            }
                            outer_placed += 1;
                            outer_can_have_bonus -= 1;
                            num_food_bonus_needed -= 1;
                            break;
                        }
                    }
                } else if third_ring_iter.peek().is_some() {
                    // Add bonus to third ring.
                    for &tile in third_ring_iter.by_ref() {
                        let (placed_bonus, placed_oasis) =
                            self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                        if placed_bonus {
                            if allow_oasis && placed_oasis {
                                // First oasis was placed on this pass, so change permission.
                                allow_oasis = false;
                            }
                            num_food_bonus_needed -= 1;
                            break;
                        }
                    }
                } else {
                    break;
                }
            } */

            // The following code is equivalent to the commented code above, but it is faster.
            // At first we try to place the bonus resources in the inner ring.
            if num_food_bonus_needed > 0 {
                for &tile in neighbor_tile_list.iter() {
                    if num_food_bonus_needed == 0
                        || inner_can_have_bonus == 0
                        || !((inner_placed < 2)
                            || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                                && inner_placed < 3))
                    {
                        break;
                    }

                    let (placed_bonus, placed_oasis) =
                        self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                    if placed_bonus {
                        if allow_oasis && placed_oasis {
                            // First oasis was placed on this pass, so change permission.
                            allow_oasis = false;
                        }
                        inner_placed += 1;
                        inner_can_have_bonus -= 1;
                        num_food_bonus_needed -= 1;
                    }
                }
            }

            // If there are still bonus resources to place, try to place them on tiles at distance 2.
            if num_food_bonus_needed > 0 {
                for &tile in tile_at_distance_two_list.iter() {
                    if num_food_bonus_needed == 0
                        || outer_can_have_bonus == 0
                        || !((inner_placed + outer_placed < 5)
                            || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                                && inner_placed + outer_placed < 4))
                    {
                        break;
                    }

                    let (placed_bonus, placed_oasis) =
                        self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                    if placed_bonus {
                        if allow_oasis && placed_oasis {
                            // First oasis was placed on this pass, so change permission.
                            allow_oasis = false;
                        }
                        outer_placed += 1;
                        outer_can_have_bonus -= 1;
                        num_food_bonus_needed -= 1;
                    }
                }
            }

            // If there are still bonus resources to place, try to place them on tiles at distance 3.
            if num_food_bonus_needed > 0 {
                for &tile in tile_at_distance_three_list.iter() {
                    if num_food_bonus_needed == 0 {
                        break;
                    }

                    let (placed_bonus, placed_oasis) =
                        self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);
                    if placed_bonus {
                        if allow_oasis && placed_oasis {
                            // First oasis was placed on this pass, so change permission.
                            allow_oasis = false;
                        }
                        num_food_bonus_needed -= 1;
                    }
                }
            }
        }

        // Check for heavy grass and light plains. Adding Stone if grass count is high and plains count is low.
        let mut num_stone_needed = if num_grassland >= 9 && num_plain == 0 {
            2
        } else if num_grassland >= 6 && num_plain <= 4 {
            1
        } else {
            0
        };

        if num_stone_needed > 0 {
            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tile_list.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tile_at_distance_two_list.shuffle(&mut self.random_number_generator);

            // At first we try to place the stone in the inner ring.
            // The stone is placed in the inner ring at most once.
            if num_stone_needed > 0 {
                for tile in neighbor_tile_list.into_iter() {
                    let placed_bonus = self.attempt_to_place_stone_at_grass_tile(tile);
                    if placed_bonus {
                        num_stone_needed -= 1;
                        break;
                    }
                }
            }

            // And then if we still have stone to place, we will try to place all the remaining stones in the outer ring.
            if num_stone_needed > 0 {
                for tile in tile_at_distance_two_list.into_iter() {
                    let placed_bonus = self.attempt_to_place_stone_at_grass_tile(tile);
                    if placed_bonus {
                        num_stone_needed -= 1;
                        if num_stone_needed == 0 {
                            break;
                        }
                    }
                }
            }
        }

        StartLocationCondition {
            along_ocean,
            next_to_lake,
            is_river,
            near_river,
            near_mountain,
            forest_count,
            jungle_count,
        }
    }

    // function AssignStartingPlots:AddStrategicBalanceResources
    /// Adds 1 unit of Strategic Resources *Iron*, *Horses* and *Oil* to civilization starting tile's `1-RADIUS` radius if `resource_setting` is [`ResourceSetting::StrategicBalance`].
    ///
    /// `RADIUS` is default `3` by defined in original CIV5.
    fn add_strategic_balance_resources(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) {
        // `RADIUS` is relative to the tiles within the starting tile's `1-RADIUS` area.
        // This is default `3` by defined in original CIV5.
        const RADIUS: u32 = 3;

        let grid = self.world_grid.grid;

        let starting_tile = self.region_list[region_index].starting_tile;

        let mut iron_list = Vec::new();
        let mut horse_list = Vec::new();
        let mut oil_list = Vec::new();

        let mut iron_fallback = Vec::new();
        let mut horse_fallback = Vec::new();
        let mut oil_fallback = Vec::new();

        for ripple_radius in 1..=RADIUS {
            starting_tile
                .tiles_at_distance(ripple_radius, grid)
                .for_each(|tile_at_distance| {
                    let terrain_type = tile_at_distance.terrain_type(self);
                    let base_terrain = tile_at_distance.base_terrain(self);
                    let feature = tile_at_distance.feature(self);
                    match terrain_type {
                        TerrainType::Hill => {
                            if ripple_radius < 3 {
                                iron_list.push(tile_at_distance);
                            } else {
                                iron_fallback.push(tile_at_distance);
                            }
                            if base_terrain != BaseTerrain::Snow && feature.is_none() {
                                horse_fallback.push(tile_at_distance);
                            }
                        }
                        TerrainType::Flatland => {
                            if feature.is_none() {
                                match base_terrain {
                                    BaseTerrain::Plain | BaseTerrain::Grassland => {
                                        if ripple_radius < 3 {
                                            horse_list.push(tile_at_distance);
                                        } else {
                                            horse_fallback.push(tile_at_distance);
                                        }
                                        iron_fallback.push(tile_at_distance);
                                        oil_fallback.push(tile_at_distance);
                                    }
                                    BaseTerrain::Tundra | BaseTerrain::Desert => {
                                        if ripple_radius < 3 {
                                            oil_list.push(tile_at_distance);
                                        } else {
                                            oil_fallback.push(tile_at_distance);
                                        }
                                        iron_fallback.push(tile_at_distance);
                                        horse_fallback.push(tile_at_distance);
                                    }
                                    BaseTerrain::Snow => {
                                        if ripple_radius < 3 {
                                            oil_list.push(tile_at_distance);
                                        } else {
                                            oil_fallback.push(tile_at_distance);
                                        }
                                    }
                                    _ => {
                                        unreachable!()
                                    }
                                }
                            } else if feature == Some(Feature::Marsh) {
                                if ripple_radius < 3 {
                                    oil_list.push(tile_at_distance);
                                } else {
                                    oil_fallback.push(tile_at_distance);
                                }
                                iron_fallback.push(tile_at_distance);
                            } else if feature == Some(Feature::Floodplain) {
                                horse_fallback.push(tile_at_distance);
                                oil_fallback.push(tile_at_distance);
                            } else if feature == Some(Feature::Jungle)
                                || feature == Some(Feature::Forest)
                            {
                                iron_fallback.push(tile_at_distance);
                                oil_fallback.push(tile_at_distance);
                            }
                        }
                        _ => (),
                    }
                });
        }

        // These resource amount is the maximum number of every type resource that can be placed on the tile.
        let (_uran_amt, horse_amt, oil_amt, iron_amt, _coal_amtt, _alum_amt) =
            get_major_strategic_resource_quantity_values(map_parameters.resource_setting);

        let mut placed_iron = false;
        let mut placed_horse = false;
        let mut placed_oil = false;

        if !iron_list.is_empty() {
            iron_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                Resource::Iron,
                iron_amt,
                1,
                1.0,
                None,
                0,
                0,
                &iron_list,
            );
            if num_left_to_place == 0 {
                placed_iron = true;
            }
        }

        if !horse_list.is_empty() {
            horse_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                Resource::Horses,
                horse_amt,
                1,
                1.0,
                None,
                0,
                0,
                &horse_list,
            );
            if num_left_to_place == 0 {
                placed_horse = true;
            }
        }

        if !oil_list.is_empty() {
            oil_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                Resource::Oil,
                oil_amt,
                1,
                1.0,
                None,
                0,
                0,
                &oil_list,
            );
            if num_left_to_place == 0 {
                placed_oil = true;
            }
        }

        if !placed_iron && !iron_fallback.is_empty() {
            iron_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                Resource::Iron,
                iron_amt,
                1,
                1.0,
                None,
                0,
                0,
                &iron_fallback,
            );
        }

        if !placed_horse && !horse_fallback.is_empty() {
            horse_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                Resource::Horses,
                horse_amt,
                1,
                1.0,
                None,
                0,
                0,
                &horse_fallback,
            );
        }

        if !placed_oil && !oil_fallback.is_empty() {
            oil_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                Resource::Oil,
                oil_amt,
                1,
                1.0,
                None,
                0,
                0,
                &oil_fallback,
            );
        }
    }

    // function AssignStartingPlots:AttemptToPlaceSmallStrategicAtPlot
    /// Attempts to place a Small `Horses` or `Iron` Resource at the currently chosen tile.
    /// If successful, it returns `true`, otherwise it returns `false`.
    fn attempt_to_place_small_strategic_at_tile(&mut self, tile: Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) == TerrainType::Flatland
            && tile.feature(self).is_none()
        {
            if matches!(
                tile.base_terrain(self),
                BaseTerrain::Grassland | BaseTerrain::Plain
            ) {
                let mut resource = Resource::Horses;
                let diceroll = self.random_number_generator.random_range(0..4);
                if diceroll == 2 {
                    resource = Resource::Iron;
                }
                tile.set_resource(self, resource, 2);
            } else {
                tile.set_resource(self, Resource::Iron, 2);
            }
            true
        } else {
            false
        }
    }

    // function AssignStartingPlots:AttemptToPlaceStoneAtGrassPlot
    /// Attempts to place a stone at a grass plot.
    /// Returns `true` if Stone is placed. Otherwise returns `false`.
    fn attempt_to_place_stone_at_grass_tile(&mut self, tile: Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) == TerrainType::Flatland
            && tile.base_terrain(self) == BaseTerrain::Grassland
            && tile.feature(self).is_none()
        {
            tile.set_resource(self, Resource::Stone, 1);
            true
        } else {
            false
        }
    }
}
