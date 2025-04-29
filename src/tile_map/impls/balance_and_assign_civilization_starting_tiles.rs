use std::{cmp::max, collections::BTreeSet};

use rand::seq::{index::sample, SliceRandom};

use crate::{
    component::map_component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    map_parameters::{MapParameters, ResourceSetting},
    ruleset::Ruleset,
    tile_map::{Layer, TileMap},
};

use super::{
    assign_starting_tile::get_major_strategic_resource_quantity_values,
    generate_regions::{RegionType, StartLocationCondition},
};

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
    /// We have not implemented to create the team for the civilization.
    pub fn balance_and_assign_civilization_starting_tiles(
        &mut self,
        map_parameters: &MapParameters,
        ruleset: &Ruleset,
    ) {
        /***** That will implement in `map_parameters` file later *****/
        // Take the civilization randomly as the starting civilization in the map.
        let mut civilization_list = ruleset
            .nations
            .iter()
            .filter(|(_, nation)| {
                nation.city_state_type == ""
                    && nation.name != "Barbarians"
                    && nation.name != "Spectator"
            })
            .map(|(civilization, _)| civilization)
            .collect::<Vec<_>>();
        // We get the civilization in the order.
        // That make sure we get the same civilization list every time we run the game.
        // We use `sort_unstable` instead of `sort` because there are no duplicate elements in the list.
        civilization_list.sort_unstable();

        let mut start_civilization_list: Vec<_> = sample(
            &mut self.random_number_generator,
            civilization_list.len(),
            map_parameters.civilization_num as usize,
        )
        .into_iter()
        .map(|i| civilization_list[i])
        .collect();
        /***** That will implement in `map_parameters` file later *****/

        for region_index in 0..self.region_list.len() {
            let start_location_condition =
                self.normalize_civilization_starting_tile(map_parameters, region_index);
            self.region_list[region_index].start_location_condition = start_location_condition;
        }

        debug_assert!(self.region_list.len() == map_parameters.civilization_num as usize);

        let disable_start_bias = false;
        // If disbable_start_bias is true, then the starting tile will be chosen randomly.
        if disable_start_bias {
            start_civilization_list.shuffle(&mut self.random_number_generator);
            self.civilization_and_starting_tile = start_civilization_list
                .iter()
                .zip(self.region_list.iter())
                .map(|(civilization, region)| (civilization.to_string(), region.starting_tile))
                .collect();
            // todo!("Set the civilization to the team.");
            return;
        }

        let mut num_coastal_civs_remaining = 0;
        let mut civs_needing_coastal_start = Vec::new();

        let mut num_river_civs_remaining = 0;
        let mut civs_needing_river_start = Vec::new();

        let mut num_priority_civs_remaining = 0;
        let mut civs_needing_region_priority = Vec::new();

        let mut num_avoid_civs = 0;
        let mut num_avoid_civs_remaining = 0;
        let mut civs_needing_region_avoid = Vec::new();

        // Store all the regions' indices that have not been assigned a civilization.
        // If the region index has been assigned a civilization, then it will be removed from the list.
        let mut region_index_list = (0..self.region_list.len()).collect::<BTreeSet<_>>();

        for civilization in start_civilization_list.iter() {
            let nation = &ruleset.nations[*civilization];
            if nation.along_ocean {
                civs_needing_coastal_start.push(*civilization);
            } else {
                if nation.along_river {
                    civs_needing_river_start.push(*civilization);
                } else {
                    if !nation.region_type_priority.is_empty() {
                        num_priority_civs_remaining = num_priority_civs_remaining + 1;
                        civs_needing_region_priority.push(*civilization);
                    } else if !nation.avoid_region_type.is_empty() {
                        num_avoid_civs = num_avoid_civs + 1;
                        num_avoid_civs_remaining = num_avoid_civs_remaining + 1;
                        civs_needing_region_avoid.push(*civilization);
                    }
                }
            }
        }

        // Handle Coastal Start Bias
        if civs_needing_coastal_start.len() > 0 {
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

                if regions_with_coastal_start.len() > 0 {
                    regions_with_coastal_start.shuffle(&mut self.random_number_generator);
                }

                if regions_with_lake_start.len() > 0 {
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
                        self.civilization_and_starting_tile.insert(
                            civilization.to_string(),
                            self.region_list[region_index].starting_tile,
                        );
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    });
            }
        }

        // Handle River bias
        if civs_needing_river_start.len() > 0 || num_coastal_civs_remaining > 0 {
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

                if regions_with_river_start.len() > 0 {
                    regions_with_river_start.shuffle(&mut self.random_number_generator);
                }

                if regions_with_near_river_start.len() > 0 {
                    regions_with_near_river_start.shuffle(&mut self.random_number_generator);
                }

                // If `civs_needing_river_start.len() > regions_with_river_start.len() + regions_with_near_river_start.len()`,
                // that means there are not enough river and near river starting tiles,
                // so `civs_needing_river_start.len() - (regions_with_river_start.len() + regions_with_near_river_start.len())`,
                // if there are enough river and near river starting tiles, `num_river_civs_remaining = 0`.
                num_river_civs_remaining = max(
                    0,
                    civs_needing_river_start.len() as i32
                        - (regions_with_river_start.len() + regions_with_near_river_start.len())
                            as i32,
                ) as usize;

                civs_needing_river_start
                    .drain(..civs_needing_river_start.len() - num_river_civs_remaining)
                    .zip(
                        regions_with_river_start
                            .iter()
                            .chain(regions_with_near_river_start.iter()),
                    )
                    .for_each(|(civilization, &region_index)| {
                        self.civilization_and_starting_tile.insert(
                            civilization.to_string(),
                            self.region_list[region_index].starting_tile,
                        );
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

                    if fallbacks_with_river_start.len() > 0 {
                        fallbacks_with_river_start.shuffle(&mut self.random_number_generator);
                    }

                    if fallbacks_with_near_river_start.len() > 0 {
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
                            self.civilization_and_starting_tile.insert(
                                civilization.to_string(),
                                self.region_list[region_index].starting_tile,
                            );
                            // Remove region index that has been assigned from region index list
                            region_index_list.remove(&region_index);
                        });
                }
            }
        }

        // Handle Region Priority
        if civs_needing_region_priority.len() > 0 {
            let mut civs_needing_single_priority = Vec::new();
            let mut civs_needing_multi_priority = Vec::new();
            let mut civs_fallback_priority = Vec::new();

            for &civilization in civs_needing_region_priority.iter() {
                let nation = &ruleset.nations[civilization];
                if nation.region_type_priority.len() == 1 {
                    civs_needing_single_priority.push(civilization);
                } else {
                    civs_needing_multi_priority.push(civilization);
                }
            }

            if civs_needing_single_priority.len() > 0 {
                // Sort civs_needing_single_priority by the first element of nation.region_type_priority
                // Notice: region_type_priority always doesn't have 'RegionType::Undefined' as the element,
                // so we don't need to tackle the case that the first element is 'RegionType::Undefined'.
                civs_needing_single_priority.sort_by_key(|&civilization| {
                    let nation = &ruleset.nations[civilization];
                    nation.region_type_priority[0] as i32
                });

                for &civilization in civs_needing_single_priority.iter() {
                    let mut candidate_regions = Vec::new();
                    for &region_index in region_index_list.iter() {
                        let region_type_priority =
                            ruleset.nations[civilization].region_type_priority[0];
                        if self.region_list[region_index].region_type == region_type_priority {
                            candidate_regions.push(region_index);
                        }
                    }

                    if candidate_regions.len() > 0 {
                        let region_index = *candidate_regions
                            .choose(&mut self.random_number_generator)
                            .unwrap();
                        self.civilization_and_starting_tile.insert(
                            civilization.to_string(),
                            self.region_list[region_index].starting_tile,
                        );
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    } else {
                        civs_fallback_priority.push(civilization);
                    }
                }
            }

            if civs_needing_multi_priority.len() > 0 {
                // Sort `civs_needing_multi_priority` by the length of nation.region_type_priority
                civs_needing_multi_priority.sort_by_key(|&civilization| {
                    let nation = &ruleset.nations[civilization];
                    nation.region_type_priority.len()
                });

                for &civilization in civs_needing_multi_priority.iter() {
                    let mut candidate_regions = Vec::new();
                    for &region_index in region_index_list.iter() {
                        let region_type_priority_list =
                            &ruleset.nations[civilization].region_type_priority;
                        if region_type_priority_list
                            .contains(&self.region_list[region_index].region_type)
                        {
                            candidate_regions.push(region_index);
                        }
                    }

                    if candidate_regions.len() > 0 {
                        let region_index = *candidate_regions
                            .choose(&mut self.random_number_generator)
                            .unwrap();
                        self.civilization_and_starting_tile.insert(
                            civilization.to_string(),
                            self.region_list[region_index].starting_tile,
                        );
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    }
                }
            }

            // Fallbacks are done (if needed) after multiple-region priority is handled. The list is pre-sorted.
            if civs_fallback_priority.len() > 0 {
                for &civilization in civs_fallback_priority.iter() {
                    let region_type_priority =
                        ruleset.nations[civilization].region_type_priority[0];
                    let region_index = self.find_fallback_for_unmatched_region_priority(
                        region_type_priority,
                        &region_index_list,
                    );
                    if let Some(region_index) = region_index {
                        self.civilization_and_starting_tile.insert(
                            civilization.to_string(),
                            self.region_list[region_index].starting_tile,
                        );
                        // Remove region index that has been assigned from region index list
                        region_index_list.remove(&region_index);
                    }
                }
            }
        }

        // Handle Region Avoid
        if civs_needing_region_avoid.len() > 0 {
            // Sort `civs_needing_region_avoid` by the length of `nation.avoid_region_type`.
            civs_needing_region_avoid.sort_by_key(|civilization| {
                let nation = &ruleset.nations[*civilization];
                nation.avoid_region_type.len()
            });

            // process in reverse order, so most needs goes first.
            for civilization in civs_needing_region_avoid.iter().rev() {
                let mut candidate_regions = Vec::new();
                for &region_index in region_index_list.iter() {
                    let region_type_priority_list =
                        &ruleset.nations[*civilization].region_type_priority;
                    if !region_type_priority_list
                        .contains(&self.region_list[region_index].region_type)
                    {
                        candidate_regions.push(region_index);
                    }
                }

                if candidate_regions.len() > 0 {
                    let region_index = *candidate_regions
                        .choose(&mut self.random_number_generator)
                        .unwrap();
                    self.civilization_and_starting_tile.insert(
                        civilization.to_string(),
                        self.region_list[region_index].starting_tile,
                    );
                    // Remove region index that has been assigned from region index list
                    region_index_list.remove(&region_index);
                }
            }
        }

        // Assign remaining civs to start plots.
        // Remove the civilization from the list if it has already been assigned a starting tile
        // and retain the civilization in the list if it has not been assigned a starting tile.
        start_civilization_list.retain(|civilization| {
            !self
                .civilization_and_starting_tile
                .contains_key(*civilization)
        });

        start_civilization_list.shuffle(&mut self.random_number_generator);

        debug_assert!(start_civilization_list.len() == region_index_list.len());

        start_civilization_list
            .iter()
            .zip(region_index_list.iter())
            .for_each(|(civilization, &region_index)| {
                self.civilization_and_starting_tile.insert(
                    civilization.to_string(),
                    self.region_list[region_index].starting_tile,
                );
            });
        // todo!("Set the civilization to the team.");
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
    /// (it will contain forest in 1-2 radius when calculating the number of hammer).
    /// 4. If resource_setting is [`ResourceSetting::StrategicBalance`], call [`TileMap::add_strategic_balance_resources`] to add strategic resources to the starting tile's 1-3 radius.
    /// 5. Add bonus resource for compensation to city state location's 1-2 radius if it has not enough food.
    /// 6. Get information about the starting tile and its surroundings for placing the civilization.
    fn normalize_civilization_starting_tile(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> StartLocationCondition {
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
        self.clear_ice_near_city_site(map_parameters, starting_tile, 1);

        let mut along_ocean = false;
        let mut next_to_lake = false;
        let mut is_river = false;
        let mut near_river = false;
        let mut near_mountain = false;

        let mut forest_count = 0;
        let mut jungle_count = 0;

        let mut num_grassland = 0;
        let mut num_plain = 0;

        if starting_tile.is_coastal_land(self, map_parameters) {
            along_ocean = true;
        }

        if starting_tile.has_river(self, map_parameters) {
            is_river = true;
        }

        let mut neighbor_tiles = starting_tile.neighbor_tiles(map_parameters);

        neighbor_tiles.iter().for_each(|neighbor_tile| {
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

                    if neighbor_tile.has_river(self, map_parameters) {
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
                    } else if neighbor_tile.is_freshwater(self, map_parameters) {
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
                                if feature == None {
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
                                if feature == None {
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

        let mut tiles_at_distance_two = starting_tile.tiles_at_distance(2, map_parameters);

        tiles_at_distance_two
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
                        } else {
                            if feature == Some(Feature::Ice) {
                                outer_bad_tiles += 1;
                            } else {
                                outer_ocean += 1;
                                outer_can_have_bonus += 1;
                            }
                        }
                    }
                    _ => {
                        if feature == Some(Feature::Jungle) {
                            jungle_count += 1;
                            num_native_two_food_second_ring += 1;
                        } else if feature == Some(Feature::Forest) {
                            forest_count += 1;
                        }

                        if tile_at_distance_two.has_river(self, map_parameters) {
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
                        } else if tile_at_distance_two.is_freshwater(self, map_parameters) {
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
                                    if feature == None {
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
                                    if feature == None {
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

        // If drastic shortage, attempt to add a hill to first ring.
        if (outer_hammer_score < 8 && inner_hammer_score < 2) || inner_hammer_score == 0 {
            neighbor_tiles.shuffle(&mut self.random_number_generator);
            for &tile in neighbor_tiles.iter() {
                // Attempt to place a Hill at the currently chosen tile.
                let placed_hill = self.attempt_to_place_hill_at_tile(map_parameters, tile);
                if placed_hill {
                    inner_hammer_score += 4;
                    break;
                }
            }
        }

        if map_parameters.resource_setting == ResourceSetting::StrategicBalance {
            self.add_strategic_balance_resources(map_parameters, region_index);
        }

        // If early hammers will be too short, attempt to add a small Horse or Iron to second ring.
        if inner_hammer_score < 3 && early_hammer_score < 6 {
            tiles_at_distance_two.shuffle(&mut self.random_number_generator);
            for &tile in tiles_at_distance_two.iter() {
                let placed_strategic = self.attempt_to_place_small_strategic_at_plot(tile);
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

        if total_food_score < 4 && inner_food_score == 0 {
            num_food_bonus_needed = 5;
        } else if total_food_score < 6 {
            num_food_bonus_needed = 4;
        } else if total_food_score < 8 {
            num_food_bonus_needed = 3;
        } else if total_food_score < 12 && inner_food_score < 5 {
            num_food_bonus_needed = 3;
        } else if total_food_score < 17 && inner_food_score < 9 {
            num_food_bonus_needed = 2;
        } else if native_two_food_tiles <= 1 {
            num_food_bonus_needed = 2;
        } else if total_food_score < 24 && inner_food_score < 11 {
            num_food_bonus_needed = 1;
        } else if native_two_food_tiles == 2 || num_native_two_food_first_ring == 0 {
            num_food_bonus_needed = 1;
        } else if total_food_score < 20 {
            num_food_bonus_needed = 1;
        }

        // Check for Legendary Start resource option.
        if map_parameters.resource_setting == ResourceSetting::LegendaryStart {
            num_food_bonus_needed += 2;
        }

        if native_two_food_tiles == 0 && num_food_bonus_needed < 3 {
            let mut tile_list = Vec::new();

            for tile in neighbor_tiles.iter().chain(tiles_at_distance_two.iter()) {
                if tile.resource(self).is_none()
                    && tile.terrain_type(self) == TerrainType::Flatland
                    && tile.base_terrain(self) == BaseTerrain::Plain
                    && tile.feature(self).is_none()
                {
                    tile_list.push(*tile);
                }
            }

            if tile_list.is_empty() {
                num_food_bonus_needed = 3;
            } else {
                let conversion_tile = *tile_list.choose(&mut self.random_number_generator).unwrap();
                self.base_terrain_query[conversion_tile.index()] = BaseTerrain::Grassland;
                self.place_impact_and_ripples(
                    map_parameters,
                    conversion_tile,
                    Layer::Strategic,
                    Some(0),
                );
            }
        }

        if num_food_bonus_needed > 0 {
            let _max_bonuses_possible = inner_can_have_bonus + outer_can_have_bonus;
            let mut inner_placed = 0;
            let mut outer_placed = 0;

            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tiles.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tiles_at_distance_two.shuffle(&mut self.random_number_generator);

            // Create a new vector to store the tiles at distance 3, and shuffle it.
            let mut tiles_at_distance_three = starting_tile.tiles_at_distance(3, map_parameters);
            tiles_at_distance_three.shuffle(&mut self.random_number_generator);

            let mut first_ring_iter = neighbor_tiles.iter().peekable();
            let mut second_ring_iter = tiles_at_distance_two.iter().peekable();
            let mut third_ring_iter = tiles_at_distance_three.iter().peekable();

            let mut allow_oasis = true; // Permanent flag. (We don't want to place more than one Oasis per location).
            while num_food_bonus_needed > 0 {
                if ((inner_placed < 2 && inner_can_have_bonus > 0)
                    || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                        && inner_placed < 3
                        && inner_can_have_bonus > 0))
                    && first_ring_iter.peek().is_some()
                {
                    // Add bonus to inner ring.
                    while let Some(&tile) = first_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                tile,
                                allow_oasis,
                            );
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
                } else if ((inner_placed + outer_placed < 5 && outer_can_have_bonus > 0)
                    || (map_parameters.resource_setting == ResourceSetting::LegendaryStart
                        && inner_placed + outer_placed < 4
                        && outer_can_have_bonus > 0))
                    && second_ring_iter.peek().is_some()
                {
                    // Add bonus to second ring.
                    while let Some(&tile) = second_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                tile,
                                allow_oasis,
                            );
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
                    while let Some(&tile) = third_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                tile,
                                allow_oasis,
                            );
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
            // Store whether we have already placed a stone. If we have, we do not place another stone in the inner ring.
            let mut inner_placed = false;

            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tiles.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tiles_at_distance_two.shuffle(&mut self.random_number_generator);

            let mut first_ring_iter = neighbor_tiles.iter().peekable();
            let mut second_ring_iter = tiles_at_distance_two.iter().peekable();

            while num_stone_needed > 0 {
                if !inner_placed && first_ring_iter.peek().is_some() {
                    // Add bonus to inner ring.
                    while let Some(&tile) = first_ring_iter.next() {
                        let placed_bonus = self.attempt_to_place_stone_at_grass_plot(tile);
                        if placed_bonus {
                            inner_placed = true;
                            num_stone_needed -= 1;
                            break;
                        }
                    }
                } else if second_ring_iter.peek().is_some() {
                    // Add bonus to second ring.
                    while let Some(&tile) = second_ring_iter.next() {
                        let placed_bonus = self.attempt_to_place_stone_at_grass_plot(tile);
                        if placed_bonus {
                            num_stone_needed -= 1;
                            break;
                        }
                    }
                } else {
                    break;
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
    /// Adds the required Strategic Resources to civilization starting tile's `1-RADIUS` radius if `resource_setting` is [`ResourceSetting::StrategicBalance`].
    fn add_strategic_balance_resources(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) {
        let starting_tile = self.region_list[region_index].starting_tile;

        let mut iron_list = Vec::new();
        let mut horse_list = Vec::new();
        let mut oil_list = Vec::new();

        let mut iron_fallback = Vec::new();
        let mut horse_fallback = Vec::new();
        let mut oil_fallback = Vec::new();

        const RADIUS: u32 = 3;

        for ripple_radius in 1..=RADIUS {
            starting_tile
                .tiles_at_distance(ripple_radius, map_parameters)
                .into_iter()
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
                            if base_terrain != BaseTerrain::Snow && feature == None {
                                horse_fallback.push(tile_at_distance);
                            }
                        }
                        TerrainType::Flatland => {
                            if feature == None {
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

        if iron_list.len() > 0 {
            iron_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Iron".to_owned()),
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

        if horse_list.len() > 0 {
            horse_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Horses".to_owned()),
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

        if oil_list.len() > 0 {
            oil_list.shuffle(&mut self.random_number_generator);
            let num_left_to_place = self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Oil".to_owned()),
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

        if !placed_iron && iron_fallback.len() > 0 {
            iron_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Iron".to_owned()),
                iron_amt,
                1,
                1.0,
                None,
                0,
                0,
                &iron_fallback,
            );
        }

        if !placed_horse && horse_fallback.len() > 0 {
            horse_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Horses".to_owned()),
                horse_amt,
                1,
                1.0,
                None,
                0,
                0,
                &horse_fallback,
            );
        }

        if !placed_oil && oil_fallback.len() > 0 {
            oil_fallback.shuffle(&mut self.random_number_generator);
            self.place_specific_number_of_resources(
                map_parameters,
                Resource::Resource("Oil".to_owned()),
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
}
