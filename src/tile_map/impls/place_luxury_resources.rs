use std::{
    cmp::max,
    collections::{HashMap, HashSet},
};

use enum_map::Enum;
use rand::{
    Rng,
    distr::{Distribution, weighted::WeightedIndex},
    seq::{IndexedRandom, SliceRandom},
};

use crate::{
    grid::WorldSizeType,
    map_parameters::{MapParameters, ResourceSetting},
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    tile_map::{Layer, TileMap},
};

impl TileMap {
    // function AssignStartingPlots:PlaceLuxuries
    /// Place Luxury Resources on the map.
    /// Before running this function, [`TileMap::assign_luxury_roles`] function must be run.
    pub fn place_luxury_resources(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        let world_size = self.world_grid.world_size_type;
        let resource_setting = map_parameters.resource_setting;

        // Stores number of each luxury had extras handed out at civ starts because of low fertility.
        // The key is the luxury type, and the value is the number of extras handed out.
        let mut luxury_low_fert_compensation = HashMap::new();
        // Stores number of luxury compensation each region received because of low fertility.
        // The index of the vector corresponds to the index of the region, and the value is the number of compensation.
        let mut region_low_fert_compensation = vec![0; map_parameters.num_civilization as usize];

        /********** Process 1: Place Luxuries at civ start locations **********/
        // Determine basic number of luxuries to place at the start location according to `resource_setting`.
        let basic_num_to_place =
            if let ResourceSetting::LegendaryStart = map_parameters.resource_setting {
                2
            } else {
                1
            };

        // Replace the code `for region_index in 0..self.region_list.len()` with the following code.
        // `region_index` in the following code is same as `region_index` in the code above.
        for (region_index, current_region_low_fert_compensation) in
            region_low_fert_compensation.iter_mut().enumerate()
        {
            let region = &self.region_list[region_index];
            let terrain_statistic = &self.region_list[region_index].terrain_statistic;
            let starting_tile = self.region_list[region_index].starting_tile;
            let exclusive_luxury = self.region_list[region_index].exclusive_luxury.unwrap();
            // Determine number to place at the start location
            // `num_to_place` contains 2 parts:
            // Part 1. The basic number of luxuries to place at the start location according to `resource_setting`.
            // Part 2. The number of luxuries to place at the start location because of low fertility.
            let mut num_to_place = basic_num_to_place;
            // Low fertility per region rectangle plot, add a luxury.
            if region.average_fertility() < 2.5 {
                num_to_place += 1;
                *luxury_low_fert_compensation
                    .entry(exclusive_luxury.to_owned())
                    .or_insert(0) += 1;
                *current_region_low_fert_compensation += 1;
            }

            let region_land_num = terrain_statistic.terrain_type_num[TerrainType::Hill]
                + terrain_statistic.terrain_type_num[TerrainType::Flatland];

            // Low fertility per region land plot, add a luxury.
            if (region.fertility_sum as f64 / region_land_num as f64) < 4.0 {
                num_to_place += 1;
                *luxury_low_fert_compensation
                    .entry(exclusive_luxury.to_owned())
                    .or_insert(0) += 1;
                *current_region_low_fert_compensation += 1;
            }

            let priority_list_indices_of_luxury =
                self.get_indices_for_luxury_type(exclusive_luxury);
            let mut luxury_plot_lists =
                self.generate_luxury_tile_lists_at_city_site(starting_tile, 2);

            let mut num_left_to_place = num_to_place;

            // First pass, checking only first two rings with a 50% ratio.
            for &i in priority_list_indices_of_luxury.iter() {
                if num_left_to_place == 0 {
                    break;
                }
                luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                num_left_to_place = self.place_specific_number_of_resources(
                    exclusive_luxury,
                    1,
                    num_left_to_place,
                    0.5,
                    None,
                    0,
                    0,
                    &luxury_plot_lists[i],
                );
            }

            if num_left_to_place > 0 {
                let mut luxury_plot_lists =
                    self.generate_luxury_tile_lists_at_city_site(starting_tile, 3);

                // Second pass, checking three rings with a 100% ratio.
                for &i in priority_list_indices_of_luxury.iter() {
                    if num_left_to_place == 0 {
                        break;
                    }
                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        exclusive_luxury,
                        1,
                        num_left_to_place,
                        1.0,
                        None,
                        0,
                        0,
                        &luxury_plot_lists[i],
                    );
                }
            }

            if num_left_to_place > 0 {
                // `num_left_to_place > 0` means that we have not been able to place all of the civ exclusive luxury resources at the civ start.
                // Now we replce with `luxury_assigned_to_random` to fill the rest `num_left_to_place`.
                //
                // These `luxury_assigned_to_random` will affect Process 4. (Please view Process 4)
                //
                // About the remainder of the civ exclusive luxury resources, it will be placed in the same region somewhere.(Please view Process 3)
                *luxury_low_fert_compensation
                    .entry(exclusive_luxury)
                    .or_insert(0) -= num_left_to_place as i32;
                // Calculates the number of `num_to_place` (Part 2) resources placed at the civilization's start.
                // NOTICE: Assumes that `num_to_place` (Part 1) resources have been fully placed at the civilization's start.
                // We should subtract that in Process 3.
                // If that is negative, it indicates that even `num_to_place` (Part 1) resources
                // have not been fully placed at the civilization's start. In such a case, during Process 3,
                // we should adjust by "subtracting" this negative value, which effectively means adding extra luxury resources.
                *current_region_low_fert_compensation -= num_left_to_place as i32;

                let mut randoms_to_place = 1;
                let resource_assigned_to_random =
                    self.luxury_resource_role.luxury_assigned_to_random.clone();
                for &random_luxury in resource_assigned_to_random.iter() {
                    let priority_list_indices_of_luxury =
                        self.get_indices_for_luxury_type(random_luxury);

                    for &i in priority_list_indices_of_luxury.iter() {
                        if randoms_to_place == 0 {
                            break;
                        }
                        luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                        randoms_to_place = self.place_specific_number_of_resources(
                            random_luxury,
                            1,
                            1,
                            1.0,
                            None,
                            0,
                            0,
                            &luxury_plot_lists[i],
                        );
                    }
                }
            }
        }
        /********** Process 1: Place Luxuries at civ start locations **********/

        /********** Process 2: Place Luxuries at City States **********/
        // Candidate luxuries include luxuries exclusive to City States, the luxury assigned to this City State's region (if in a region), and the randoms.
        for i in 0..self.city_state_starting_tile_and_region_index.len() {
            let &(starting_tile, region_index) = &self.city_state_starting_tile_and_region_index[i];

            let allowed_luxuries =
                self.get_list_of_allowable_luxuries_at_city_site(starting_tile, 2);
            // Store the luxury types that can only be owned by city states and are allowed at this city state.
            // It should meet the following criteria:
            // 1. The luxury type is assigned to city states. (based on the luxury role)
            // 2. The luxury type is allowed at this city state. (based on the allowed luxuries)
            let city_state_luxury_types: Vec<_> = self
                .luxury_resource_role
                .luxury_assigned_to_city_state
                .iter()
                .filter(|luxury| allowed_luxuries.contains(luxury))
                .copied()
                .collect();

            // Store the luxury types the city state can own and the weight of each luxury type.
            // The luxury types contains as follows:
            // 1. The luxury type can only be owned by city states and is allowed at this city state.
            // 2. The luxury type can only be owned by regions and is allowed at this city state. (if the region is not null)
            // 3. The random luxury type is allowed at this city state.
            let mut luxury_for_city_state_and_weight = Vec::new();

            // Add the luxury types that can only be owned by city states and are allowed at this city state to the list.
            city_state_luxury_types.iter().for_each(|&luxury| {
                luxury_for_city_state_and_weight
                    .push((luxury, 75. / city_state_luxury_types.len() as f64));
            });

            let random_types_allowed: Vec<_> = self
                .luxury_resource_role
                .luxury_assigned_to_city_state
                .iter()
                .filter(|luxury| allowed_luxuries.contains(luxury))
                .copied()
                .collect();

            let mut num_allowed = random_types_allowed.len();

            // Add the luxury types that can only be owned by regions and are allowed at this city state to the list.
            if let Some(region_index) = region_index {
                // Adding the region type in to the mix with the random types.
                num_allowed += 1;
                let luxury = self.region_list[region_index].exclusive_luxury.unwrap();
                if allowed_luxuries.contains(&luxury) {
                    luxury_for_city_state_and_weight.push((luxury, 25. / num_allowed as f64));
                }
            }

            // Add the random luxury types that are allowed at this city state to the list.
            random_types_allowed.iter().for_each(|&luxury| {
                luxury_for_city_state_and_weight.push((luxury, 25. / num_allowed as f64));
            });

            if !luxury_for_city_state_and_weight.is_empty() {
                let dist =
                    WeightedIndex::new(luxury_for_city_state_and_weight.iter().map(|item| item.1))
                        .unwrap();
                // Choose luxury type.
                let luxury_resource = luxury_for_city_state_and_weight
                    [dist.sample(&mut self.random_number_generator)]
                .0;
                // Place luxury.
                let priority_list_indices_of_luxury =
                    self.get_indices_for_luxury_type(luxury_resource);
                let mut luxury_plot_lists =
                    self.generate_luxury_tile_lists_at_city_site(starting_tile, 2);

                let mut num_left_to_place = 1;

                for &i in priority_list_indices_of_luxury.iter() {
                    if num_left_to_place == 0 {
                        break;
                    }
                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        luxury_resource,
                        1,
                        num_left_to_place,
                        1.0,
                        None,
                        0,
                        0,
                        &luxury_plot_lists[i],
                    );
                }
            }
        }
        /********** Process 2: Place Luxuries at City States **********/

        /********** Process 3: Place Regional Luxuries **********/
        // In process 1, we have not been able to place all of the civ exclusive luxury resources at the civ start.
        // Now we place the remainder in the same region during this process.

        // Replace the code `for region_index in 0..self.region_list.len()` with the following code.
        // `region_index` in the following code is same as `region_index` in the code above.
        for (region_index, &current_region_low_fert_compensation) in
            region_low_fert_compensation.iter().enumerate()
        {
            let luxury_resource = self.region_list[region_index].exclusive_luxury.unwrap();
            let luxury_assign_to_region_count: u32 =
                self.luxury_assign_to_region_count[&luxury_resource];
            let priority_list_indices_of_luxury = self.get_indices_for_luxury_type(luxury_resource);

            let mut luxury_plot_lists = self.generate_luxury_tile_lists_in_region(region_index);

            let current_luxury_low_fert_compensation = *luxury_low_fert_compensation
                .entry(luxury_resource)
                .or_insert(0);

            // Calibrate the number of luxuries per region based on the world size and the number of civilizations.
            // The number of luxuries per region should be highest when the number of civilizations is closest to the "default" value for that map size.
            let target_list = get_region_luxury_target_numbers(world_size);
            let mut target_num = ((target_list[map_parameters.num_civilization as usize] as f64
                + 0.5 * current_luxury_low_fert_compensation as f64)
                / luxury_assign_to_region_count as f64) as i32;

            // `current_region_low_fert_compensation` is the number of `num_to_place` (Part 2) resources placed at the civilization's start.
            // NOTICE: Assumes that `num_to_place` (Part 1) resources have been fully placed at the civilization's start.
            // We should subtract that in this process.
            // If that is negative, it indicates that even `num_to_place` (Part 1) resources
            // have not been fully placed at the civilization's start. In such a case, during Process 3,
            // we should adjust by "subtracting" this negative value, which effectively means adding extra luxury resources.
            // View Process 1 for more details.
            target_num -= current_region_low_fert_compensation;

            match map_parameters.resource_setting {
                ResourceSetting::Sparse => target_num -= 1,
                ResourceSetting::Abundant => target_num += 1,
                _ => (),
            }

            // Always place at least one luxury resource in current region.
            let num_luxury_to_place = max(1, target_num) as u32;

            let mut num_left_to_place = num_luxury_to_place;

            const RATIO_AND_MAX_RADIUS: [(f64, u32); 4] = [(0.3, 3), (0.3, 3), (0.4, 2), (0.5, 2)];

            for (&i, &(ratio, max_radius)) in priority_list_indices_of_luxury
                .iter()
                .zip(RATIO_AND_MAX_RADIUS.iter())
            {
                if num_left_to_place == 0 {
                    break;
                }
                luxury_plot_lists[i].shuffle(&mut self.random_number_generator);

                num_left_to_place = self.place_specific_number_of_resources(
                    luxury_resource,
                    1,
                    num_left_to_place,
                    ratio,
                    Some(Layer::Luxury),
                    0,
                    max_radius,
                    &luxury_plot_lists[i],
                );
            }
        }
        /********** Process 3: Place Regional Luxuries **********/

        /********** Process 4: Place Random Luxuries **********/
        let num_random_luxury_types = self.luxury_resource_role.luxury_assigned_to_random.len();
        if num_random_luxury_types > 0 {
            // `num_random_luxury_target` is the number of random luxuries to place in the world during this process.
            // - It shouldn't contain `luxury_assigned_to_random` that have already been placed in the world.
            // - It should be adjusted by the number of civilizations, and add a random number of luxuries according to the number of civilizations.
            let [target_luxury, loop_target] =
                get_world_luxury_target_numbers(world_size, resource_setting);
            let extra_luxury = self
                .random_number_generator
                .random_range(0..map_parameters.num_civilization);
            let num_placed_luxuries = self.num_placed_luxury_resources(ruleset);
            let num_random_luxury_target = target_luxury + extra_luxury - num_placed_luxuries;

            let mut num_this_luxury_to_place;

            // This table weights the amount of random luxuries to place, with first-selected getting heavier weighting.
            let random_luxury_ratios_table = [
                vec![1.],
                vec![0.55, 0.45],
                vec![0.40, 0.33, 0.27],
                vec![0.35, 0.25, 0.25, 0.15],
                vec![0.25, 0.25, 0.20, 0.15, 0.15],
                vec![0.20, 0.20, 0.20, 0.15, 0.15, 0.10],
                vec![0.20, 0.20, 0.15, 0.15, 0.10, 0.10, 0.10],
                vec![0.20, 0.15, 0.15, 0.10, 0.10, 0.10, 0.10, 0.10],
            ];

            for i in 0..num_random_luxury_types {
                let luxury_resource = self.luxury_resource_role.luxury_assigned_to_random[i];

                let priority_list_indices_of_luxury =
                    self.get_indices_for_luxury_type(luxury_resource);

                // If calculated number of randoms is low, just place 3 of each radom luxury type.
                if num_random_luxury_types * 3 > num_random_luxury_target as usize {
                    num_this_luxury_to_place = 3;
                } else if num_random_luxury_types > 8 {
                    num_this_luxury_to_place = max(3, num_random_luxury_target.div_ceil(10));
                } else {
                    // num_random_luxury_types <= 8
                    let luxury_minimum = max(3, loop_target - i as u32);
                    let luxury_share_of_remaining = (num_random_luxury_target as f64
                        * random_luxury_ratios_table[num_random_luxury_types - 1][i])
                        .ceil() as u32;
                    num_this_luxury_to_place = max(luxury_minimum, luxury_share_of_remaining);
                }

                let mut current_list = self.generate_luxury_resource_tile_lists_in_map();
                // Place this luxury type.
                let mut num_left_to_place = num_this_luxury_to_place;

                const RATIO: [f64; 4] = [0.25, 0.25, 0.25, 0.3];

                for (&i, &ratio) in priority_list_indices_of_luxury.iter().zip(RATIO.iter()) {
                    if num_left_to_place == 0 {
                        break;
                    }
                    current_list[i].shuffle(&mut self.random_number_generator);

                    num_left_to_place = self.place_specific_number_of_resources(
                        luxury_resource,
                        1,
                        num_left_to_place,
                        ratio,
                        Some(Layer::Luxury),
                        4,
                        6,
                        &current_list[i],
                    );
                }
            }
        }
        /********** Process 4: Place Random Luxuries **********/

        /********** Process 5: Place Second Luxury Type at civ start locations **********/
        // For resource settings other than "Sparse", add a second luxury type at starting locations.
        // This second luxury type will be selected in the following order:
        //   1. Random types, if available.
        //   2. Special Case types, if resource setting is not "Strategic Balance".
        //   3. CS types, if no random or Special Case types are available.
        //   4. Types from other regions, if no random, Special Case, or CS types are available.
        if map_parameters.resource_setting != ResourceSetting::Sparse {
            for region_index in 0..self.region_list.len() {
                let starting_tile = self.region_list[region_index].starting_tile;
                let allowed_luxuries =
                    self.get_list_of_allowable_luxuries_at_city_site(starting_tile, 2);

                let mut candidate_luxury_types = Vec::new();

                // See if any Random types are eligible.
                for &luxury in self.luxury_resource_role.luxury_assigned_to_random.iter() {
                    if allowed_luxuries.contains(&luxury) {
                        candidate_luxury_types.push(luxury);
                    }
                }

                // Check to see if any Special Case luxuries are eligible. Disallow if Strategic Balance resource setting.
                if map_parameters.resource_setting != ResourceSetting::StrategicBalance {
                    for &luxury in self
                        .luxury_resource_role
                        .luxury_assigned_to_special_case
                        .iter()
                    {
                        if allowed_luxuries.contains(&luxury) {
                            candidate_luxury_types.push(luxury);
                        }
                    }
                }

                let mut use_this_luxury = None;

                if !candidate_luxury_types.is_empty() {
                    use_this_luxury =
                        candidate_luxury_types.choose(&mut self.random_number_generator);
                } else {
                    // No Random or Special Case luxuries available. See if any City State types are eligible.
                    for &luxury in self
                        .luxury_resource_role
                        .luxury_assigned_to_city_state
                        .iter()
                    {
                        if allowed_luxuries.contains(&luxury) {
                            candidate_luxury_types.push(luxury);
                        }
                    }

                    if !candidate_luxury_types.is_empty() {
                        use_this_luxury =
                            candidate_luxury_types.choose(&mut self.random_number_generator);
                    } else {
                        // No City State luxuries available. Use a type from another region.
                        let region_luxury =
                            self.region_list[region_index].exclusive_luxury.unwrap();
                        for &luxury in self.luxury_resource_role.luxury_assigned_to_regions.iter() {
                            if allowed_luxuries.contains(&luxury) && luxury != region_luxury {
                                candidate_luxury_types.push(luxury);
                            }
                        }
                        if !candidate_luxury_types.is_empty() {
                            use_this_luxury =
                                candidate_luxury_types.choose(&mut self.random_number_generator);
                        }
                    }
                }

                if let Some(&luxury) = use_this_luxury {
                    let priority_list_indices_of_luxury = self.get_indices_for_luxury_type(luxury);

                    let mut luxury_plot_lists =
                        self.generate_luxury_tile_lists_at_city_site(starting_tile, 2);

                    let mut num_left_to_place = 1;

                    for &i in priority_list_indices_of_luxury.iter() {
                        if num_left_to_place == 0 {
                            break;
                        }
                        luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                        num_left_to_place = self.place_specific_number_of_resources(
                            luxury,
                            1,
                            num_left_to_place,
                            1.,
                            None,
                            0,
                            0,
                            &luxury_plot_lists[i],
                        );
                    }
                }
            }
        }
        /********** Process 5: Place Second Luxury Type at civ start locations **********/

        /********** Process 6: Place Special Case Luxury Resources **********/
        if !self
            .luxury_resource_role
            .luxury_assigned_to_special_case
            .is_empty()
        {
            let luxury_list = self
                .luxury_resource_role
                .luxury_assigned_to_special_case
                .clone();
            for luxury in luxury_list {
                match luxury {
                    Resource::Marble => {
                        self.place_marble(map_parameters);
                    }
                    _ => {
                        panic!(
                            "{} is Special Case Luxury, you need to implement a custom placement method for it!",
                            luxury.as_str()
                        );
                    }
                }
            }
        }
        /********** Process 6: Place Special Case Luxury Resources **********/
    }

    fn place_marble(&mut self, map_parameters: &MapParameters) {
        let luxury_resource = Resource::Marble;
        let marble_already_placed: u32 = self.placed_resource_count(luxury_resource);

        let marble_target = match map_parameters.resource_setting {
            ResourceSetting::Sparse => (map_parameters.num_civilization as f32 * 0.5).ceil() as i32,
            ResourceSetting::Abundant => {
                (map_parameters.num_civilization as f32 * 0.9).ceil() as i32
            }
            _ => (map_parameters.num_civilization as f32 * 0.75).ceil() as i32,
        };

        let mut marble_tile_list = Vec::new();
        self.all_tiles().for_each(|tile| {
            let terrain_type = tile.terrain_type(self);
            let base_terrain = tile.base_terrain(self);
            let feature = tile.feature(self);

            match terrain_type {
                TerrainType::Water => {}
                TerrainType::Flatland => {
                    if feature.is_none() {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                if !tile.is_freshwater(self) {
                                    marble_tile_list.push(tile);
                                }
                            }
                            BaseTerrain::Desert => {
                                marble_tile_list.push(tile);
                            }
                            BaseTerrain::Plain => {
                                if !tile.is_freshwater(self) {
                                    marble_tile_list.push(tile);
                                }
                            }
                            BaseTerrain::Tundra => {
                                marble_tile_list.push(tile);
                            }
                            _ => {}
                        }
                    }
                }
                TerrainType::Mountain => {}
                TerrainType::Hill => {
                    if base_terrain != BaseTerrain::Snow && feature.is_none() {
                        marble_tile_list.push(tile);
                    }
                }
            }
        });

        let num_marble_to_place = max(2, marble_target - marble_already_placed as i32) as u32;

        let mut num_left_to_place = num_marble_to_place;
        if marble_tile_list.is_empty() {
            // println!("No eligible plots available to place Marble!");
            return;
        }

        marble_tile_list.shuffle(&mut self.random_number_generator);

        // Place the marble.
        for &tile in marble_tile_list.iter() {
            if num_left_to_place == 0 {
                break;
            }
            if tile.resource(self).is_none()
                && self.layer_data[Layer::Marble][tile.index()] == 0
                && self.layer_data[Layer::Luxury][tile.index()] == 0
            {
                // Placing this resource in this plot.
                tile.set_resource(self, luxury_resource, 1);
                num_left_to_place -= 1;
                // println!("Still need to place {} more units of Marble.", num_left_to_place);
                self.place_impact_and_ripples(tile, Layer::Marble, u32::MAX);
            }
        }

        if num_left_to_place > 0 {
            eprintln!("Failed to place {} units of Marble.", num_left_to_place);
        }
    }

    // function AssignStartingPlots:GenerateGlobalResourcePlotLists
    /// Generate the candidate tile lists for placing luxury resources on the entire map.
    ///
    /// Each `Vec` is shuffled to ensure randomness.
    ///
    /// # Returns
    ///
    /// - `[Vec<Tile>; 15]`: An array of vectors of tiles, where each inner vector represents a list of candidate tiles matching a specific criteria.
    ///   Each `Vec` is shuffled to ensure randomness.
    fn generate_luxury_resource_tile_lists_in_map(&mut self) -> [Vec<Tile>; 15] {
        let grid = self.world_grid.grid;

        let mut region_coast_next_to_land_tile_list = Vec::new();
        let mut region_hill_open_tile_list = Vec::new();
        let mut region_hill_jungle_tile_list = Vec::new();
        let mut region_hill_forest_tile_list = Vec::new();
        let mut region_hill_covered_tile_list = Vec::new();
        let mut region_tundra_flat_including_forest_tile_list = Vec::new();
        let mut region_forest_flat_but_not_tundra_tile_list = Vec::new();
        let mut region_desert_flat_no_feature_tile_list = Vec::new();
        let mut region_plain_flat_no_feature_tile_list = Vec::new();
        let mut region_fresh_water_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_dry_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_forest_flat_tile_list = Vec::new();
        let mut region_marsh_tile_list = Vec::new();
        let mut region_flood_plain_tile_list = Vec::new();
        let mut region_jungle_flat_tile_list = Vec::new();

        self.all_tiles().for_each(|tile| {
            if !self.player_collision_data[tile.index()] && tile.resource(self).is_none() {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                let feature = tile.feature(self);

                match terrain_type {
                    TerrainType::Water => {
                        if base_terrain == BaseTerrain::Coast
                            && feature != Some(Feature::Ice)
                            && feature != Some(Feature::Atoll)
                            && tile.neighbor_tiles(grid).any(|neighbor_tile| {
                                neighbor_tile.terrain_type(self) != TerrainType::Water
                            })
                        {
                            region_coast_next_to_land_tile_list.push(tile);
                        }
                    }
                    TerrainType::Flatland => {
                        if let Some(feature) = feature {
                            match feature {
                                Feature::Forest => {
                                    region_forest_flat_tile_list.push(tile);
                                    if base_terrain == BaseTerrain::Tundra {
                                        region_tundra_flat_including_forest_tile_list.push(tile);
                                    } else {
                                        region_forest_flat_but_not_tundra_tile_list.push(tile);
                                    }
                                }
                                Feature::Jungle => {
                                    region_jungle_flat_tile_list.push(tile);
                                }
                                Feature::Marsh => {
                                    region_marsh_tile_list.push(tile);
                                }
                                Feature::Floodplain => {
                                    region_flood_plain_tile_list.push(tile);
                                }
                                _ => {}
                            }
                        } else {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    if tile.is_freshwater(self) {
                                        region_fresh_water_grass_flat_no_feature_tile_list
                                            .push(tile);
                                    } else {
                                        region_dry_grass_flat_no_feature_tile_list.push(tile);
                                    }
                                }
                                BaseTerrain::Desert => {
                                    region_desert_flat_no_feature_tile_list.push(tile);
                                }
                                BaseTerrain::Plain => {
                                    region_plain_flat_no_feature_tile_list.push(tile);
                                }
                                BaseTerrain::Tundra => {
                                    region_tundra_flat_including_forest_tile_list.push(tile);
                                }
                                _ => {}
                            }
                        }
                    }
                    TerrainType::Mountain => {}
                    TerrainType::Hill => {
                        if base_terrain != BaseTerrain::Snow {
                            if feature.is_none() {
                                region_hill_open_tile_list.push(tile);
                            } else if feature == Some(Feature::Forest) {
                                region_hill_forest_tile_list.push(tile);
                                region_hill_covered_tile_list.push(tile);
                            } else if feature == Some(Feature::Jungle) {
                                region_hill_jungle_tile_list.push(tile);
                                region_hill_covered_tile_list.push(tile);
                            }
                        }
                    }
                }
            }
        });

        let mut lists = [
            region_coast_next_to_land_tile_list,
            region_marsh_tile_list,
            region_flood_plain_tile_list,
            region_hill_open_tile_list,
            region_hill_covered_tile_list,
            region_hill_jungle_tile_list,
            region_hill_forest_tile_list,
            region_jungle_flat_tile_list,
            region_forest_flat_tile_list,
            region_desert_flat_no_feature_tile_list,
            region_plain_flat_no_feature_tile_list,
            region_dry_grass_flat_no_feature_tile_list,
            region_fresh_water_grass_flat_no_feature_tile_list,
            region_tundra_flat_including_forest_tile_list,
            region_forest_flat_but_not_tundra_tile_list,
        ];

        // Shuffle each list. This is done to ensure that the order in which resources are placed is random.
        lists.iter_mut().for_each(|list| {
            list.shuffle(&mut self.random_number_generator);
        });

        lists
    }

    // AssignStartingPlots:GenerateLuxuryPlotListsAtCitySite
    /// Generate the candidate tile lists for placing luxury resources within the specified radius around a city site, excluding the city site itself.
    ///
    /// # Arguments
    ///
    /// - `city_site`: The tile representing the city site. This is the center of the radius.
    /// - `radius`: The radius within which to generate candidate tiles.
    ///   For example, if `radius` is 2, the function will consider tiles within a distance of 2 tiles from the city site, excluding the city site itself.
    ///   In original CIV5 code, the max radius which city site can extend is 5. So `radius` should be in `[1, 5]`.
    ///
    /// # Returns
    ///
    /// - `[Vec<Tile>; 15]`: An array of vectors of tiles, where each inner vector represents a list of candidate tiles matching a specific criteria.
    ///
    /// # Notice
    ///
    /// In the original code, `clear ice near city site` and `generate luxury plot lists at city site` are combined in one method.
    /// We have extracted the `clear ice near city site` into a separate method.
    /// If you want to clear ice near city site, you should use [`TileMap::clear_ice_near_city_site`].\
    /// TODO: Sometimes this function is used for strategic resources, so the name should be changed.
    pub fn generate_luxury_tile_lists_at_city_site(
        &self,
        city_site: Tile,
        radius: u32,
    ) -> [Vec<Tile>; 15] {
        let grid = self.world_grid.grid;

        let mut region_coast_tile_list = Vec::new();
        let mut region_hill_open_tile_list = Vec::new();
        let mut region_hill_jungle_tile_list = Vec::new();
        let mut region_hill_forest_tile_list = Vec::new();
        let mut region_hill_covered_tile_list = Vec::new();
        let mut region_tundra_flat_including_forest_tile_list = Vec::new();
        let mut region_forest_flat_but_not_tundra_tile_list = Vec::new();
        let mut region_desert_flat_no_feature_tile_list = Vec::new();
        let mut region_plain_flat_no_feature_tile_list = Vec::new();
        let mut region_fresh_water_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_dry_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_forest_flat_tile_list = Vec::new();
        let mut region_marsh_tile_list = Vec::new();
        let mut region_flood_plain_tile_list = Vec::new();
        let mut region_jungle_flat_tile_list = Vec::new();

        // In original CIV5 code, the max radius which city site can extend is 5.
        // So we only consider the tiles within the radius of 5 from the city site.
        if radius > 0 && radius < 6 {
            for ripple_radius in 1..=radius {
                city_site
                    .tiles_at_distance(ripple_radius, grid)
                    .for_each(|tile_at_distance| {
                        let terrain_type = tile_at_distance.terrain_type(self);
                        let base_terrain = tile_at_distance.base_terrain(self);
                        let feature = tile_at_distance.feature(self);

                        match terrain_type {
                            TerrainType::Water => {
                                if base_terrain == BaseTerrain::Coast
                                    && feature != Some(Feature::Ice)
                                    && feature != Some(Feature::Atoll)
                                {
                                    region_coast_tile_list.push(tile_at_distance);
                                }
                            }
                            TerrainType::Flatland => {
                                if let Some(feature) = feature {
                                    match feature {
                                        Feature::Forest => {
                                            region_forest_flat_tile_list.push(tile_at_distance);
                                            if base_terrain == BaseTerrain::Tundra {
                                                region_tundra_flat_including_forest_tile_list
                                                    .push(tile_at_distance);
                                            } else {
                                                region_forest_flat_but_not_tundra_tile_list
                                                    .push(tile_at_distance);
                                            }
                                        }
                                        Feature::Jungle => {
                                            region_jungle_flat_tile_list.push(tile_at_distance);
                                        }
                                        Feature::Marsh => {
                                            region_marsh_tile_list.push(tile_at_distance);
                                        }
                                        Feature::Floodplain => {
                                            region_flood_plain_tile_list.push(tile_at_distance);
                                        }
                                        _ => {}
                                    }
                                } else {
                                    match base_terrain {
                                        BaseTerrain::Grassland => {
                                            if tile_at_distance.is_freshwater(self) {
                                                region_fresh_water_grass_flat_no_feature_tile_list
                                                    .push(tile_at_distance);
                                            } else {
                                                region_dry_grass_flat_no_feature_tile_list
                                                    .push(tile_at_distance);
                                            }
                                        }
                                        BaseTerrain::Desert => {
                                            region_desert_flat_no_feature_tile_list
                                                .push(tile_at_distance);
                                        }
                                        BaseTerrain::Plain => {
                                            region_plain_flat_no_feature_tile_list
                                                .push(tile_at_distance);
                                        }
                                        BaseTerrain::Tundra => {
                                            region_tundra_flat_including_forest_tile_list
                                                .push(tile_at_distance);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            TerrainType::Mountain => {}
                            TerrainType::Hill => {
                                if base_terrain != BaseTerrain::Snow {
                                    if feature.is_none() {
                                        region_hill_open_tile_list.push(tile_at_distance);
                                    } else if feature == Some(Feature::Forest) {
                                        region_hill_forest_tile_list.push(tile_at_distance);
                                        region_hill_covered_tile_list.push(tile_at_distance);
                                    } else if feature == Some(Feature::Jungle) {
                                        region_hill_jungle_tile_list.push(tile_at_distance);
                                        region_hill_covered_tile_list.push(tile_at_distance);
                                    }
                                }
                            }
                        }
                    });
            }
        }

        [
            region_coast_tile_list,
            region_marsh_tile_list,
            region_flood_plain_tile_list,
            region_hill_open_tile_list,
            region_hill_covered_tile_list,
            region_hill_jungle_tile_list,
            region_hill_forest_tile_list,
            region_jungle_flat_tile_list,
            region_forest_flat_tile_list,
            region_desert_flat_no_feature_tile_list,
            region_plain_flat_no_feature_tile_list,
            region_dry_grass_flat_no_feature_tile_list,
            region_fresh_water_grass_flat_no_feature_tile_list,
            region_tundra_flat_including_forest_tile_list,
            region_forest_flat_but_not_tundra_tile_list,
        ]
    }

    // AssignStartingPlots:GenerateLuxuryPlotListsAtCitySite
    /// Clear [`Feature::Ice`] from the map within a given radius of the city site.
    ///
    /// # Notice
    ///
    /// In the original code, `clear ice near city site` and `generate luxury plot lists at city site` are combined in one method.
    /// We have extracted the `generate luxury plot lists at city site` into a separate method.
    /// If you want to generate luxury plot lists at city site, you need to call [`TileMap::generate_luxury_tile_lists_at_city_site`].
    pub fn clear_ice_near_city_site(&mut self, city_site: Tile, radius: u32) {
        let grid = self.world_grid.grid;

        for ripple_radius in 1..=radius {
            city_site
                .tiles_at_distance(ripple_radius, grid)
                .for_each(|tile_at_distance| {
                    let feature = tile_at_distance.feature(self);
                    if feature == Some(Feature::Ice) {
                        tile_at_distance.clear_feature(self);
                    }
                })
        }
    }

    // function AssignStartingPlots:GenerateLuxuryPlotListsInRegion
    /// Generate the candidate tile lists for placing luxury resources in a region.
    ///
    /// # Arguments
    ///
    /// - `region_index` - The index of the region to generate the candidate tile lists for.
    ///
    /// # Returns
    ///
    /// - `[Vec<Tile>; 15]`: An array of vectors of tiles, where each inner vector represents a list of candidate tiles matching a specific criteria.
    ///   NOTICE: We don't shuffle the lists here. We do that in the calling function.
    fn generate_luxury_tile_lists_in_region(&self, region_index: usize) -> [Vec<Tile>; 15] {
        let grid = self.world_grid.grid;

        let rectangle = self.region_list[region_index].rectangle;

        let landmass_id = self.region_list[region_index].area_id;

        let mut region_coast_next_to_land_tile_list = Vec::new();
        let mut region_hill_open_tile_list = Vec::new();
        let mut region_hill_jungle_tile_list = Vec::new();
        let mut region_hill_forest_tile_list = Vec::new();
        let mut region_hill_covered_tile_list = Vec::new();
        let mut region_tundra_flat_including_forest_tile_list = Vec::new();
        let mut region_forest_flat_but_not_tundra_tile_list = Vec::new();
        let mut region_desert_flat_no_feature_tile_list = Vec::new();
        let mut region_plain_flat_no_feature_tile_list = Vec::new();
        let mut region_fresh_water_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_dry_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_forest_flat_tile_list = Vec::new();
        let mut region_marsh_tile_list = Vec::new();
        let mut region_flood_plain_tile_list = Vec::new();
        let mut region_jungle_flat_tile_list = Vec::new();

        rectangle.all_tiles(grid).for_each(|tile| {
            let terrain_type = tile.terrain_type(self);
            let base_terrain = tile.base_terrain(self);
            let feature = tile.feature(self);

            match terrain_type {
                TerrainType::Water => {
                    if base_terrain == BaseTerrain::Coast
                        && feature != Some(Feature::Ice)
                        && feature != Some(Feature::Atoll)
                    {
                        if let Some(landmass_id) = landmass_id {
                            if tile
                                .neighbor_tiles(grid)
                                .any(|neighbor_tile| neighbor_tile.area_id(self) == landmass_id)
                            {
                                region_coast_next_to_land_tile_list.push(tile);
                            }
                        } else {
                            region_coast_next_to_land_tile_list.push(tile);
                        }
                    }
                }
                TerrainType::Flatland => {
                    if let Some(feature) = feature {
                        match feature {
                            Feature::Forest => {
                                region_forest_flat_tile_list.push(tile);
                                if base_terrain == BaseTerrain::Tundra {
                                    region_tundra_flat_including_forest_tile_list.push(tile);
                                } else {
                                    region_forest_flat_but_not_tundra_tile_list.push(tile);
                                }
                            }
                            Feature::Jungle => {
                                region_jungle_flat_tile_list.push(tile);
                            }
                            Feature::Marsh => {
                                region_marsh_tile_list.push(tile);
                            }
                            Feature::Floodplain => {
                                region_flood_plain_tile_list.push(tile);
                            }
                            _ => {}
                        }
                    } else {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                if tile.is_freshwater(self) {
                                    region_fresh_water_grass_flat_no_feature_tile_list.push(tile);
                                } else {
                                    region_dry_grass_flat_no_feature_tile_list.push(tile);
                                }
                            }
                            BaseTerrain::Desert => {
                                region_desert_flat_no_feature_tile_list.push(tile);
                            }
                            BaseTerrain::Plain => {
                                region_plain_flat_no_feature_tile_list.push(tile);
                            }
                            BaseTerrain::Tundra => {
                                region_tundra_flat_including_forest_tile_list.push(tile);
                            }
                            _ => {}
                        }
                    }
                }
                TerrainType::Mountain => {}
                TerrainType::Hill => {
                    if base_terrain != BaseTerrain::Snow {
                        if feature.is_none() {
                            region_hill_open_tile_list.push(tile);
                        } else if feature == Some(Feature::Forest) {
                            region_hill_forest_tile_list.push(tile);
                            region_hill_covered_tile_list.push(tile);
                        } else if feature == Some(Feature::Jungle) {
                            region_hill_jungle_tile_list.push(tile);
                            region_hill_covered_tile_list.push(tile);
                        }
                    }
                }
            }
        });

        [
            region_coast_next_to_land_tile_list,
            region_marsh_tile_list,
            region_flood_plain_tile_list,
            region_hill_open_tile_list,
            region_hill_covered_tile_list,
            region_hill_jungle_tile_list,
            region_hill_forest_tile_list,
            region_jungle_flat_tile_list,
            region_forest_flat_tile_list,
            region_desert_flat_no_feature_tile_list,
            region_plain_flat_no_feature_tile_list,
            region_dry_grass_flat_no_feature_tile_list,
            region_fresh_water_grass_flat_no_feature_tile_list,
            region_tundra_flat_including_forest_tile_list,
            region_forest_flat_but_not_tundra_tile_list,
        ]
    }

    // function AssignStartingPlots:GetListOfAllowableLuxuriesAtCitySite
    /// Get a list of allowable luxury resources that can be placed at a given city site within a specified radius.
    ///
    /// # Arguments
    ///
    /// - `city_site`: The tile representing the city site. This is the center of the radius.
    /// - `radius`: The radius within which to check for allowable luxury resources.
    ///   For example, if `radius` is 2, the function will consider tiles within a distance of 2 tiles from the city site, excluding the city site itself.
    fn get_list_of_allowable_luxuries_at_city_site(
        &self,
        city_site: Tile,
        radius: u32,
    ) -> HashSet<Resource> {
        let grid = self.world_grid.grid;

        let mut allowed_luxuries = HashSet::new();
        for ripple_radius in 1..=radius {
            city_site
                .tiles_at_distance(ripple_radius, grid)
                .for_each(|tile| {
                    let terrain_type = tile.terrain_type(self);
                    let base_terrain = tile.base_terrain(self);
                    let feature = tile.feature(self);
                    match terrain_type {
                        TerrainType::Water => {
                            if base_terrain == BaseTerrain::Coast
                                && feature != Some(Feature::Atoll)
                                && feature != Some(Feature::Ice)
                            {
                                allowed_luxuries.insert(Resource::Whales);
                                allowed_luxuries.insert(Resource::Pearls);
                            }
                        }
                        TerrainType::Flatland => {
                            if let Some(feature) = feature {
                                match feature {
                                    Feature::Forest => {
                                        allowed_luxuries.insert(Resource::Furs);
                                        allowed_luxuries.insert(Resource::Dyes);
                                        if base_terrain == BaseTerrain::Tundra {
                                            allowed_luxuries.insert(Resource::Silver);
                                        } else {
                                            allowed_luxuries.insert(Resource::Spices);
                                            allowed_luxuries.insert(Resource::Silk);
                                        }
                                    }
                                    Feature::Jungle => {
                                        allowed_luxuries.insert(Resource::Gems);
                                        allowed_luxuries.insert(Resource::Dyes);
                                        allowed_luxuries.insert(Resource::Spices);
                                        allowed_luxuries.insert(Resource::Silk);
                                        allowed_luxuries.insert(Resource::Sugar);
                                        allowed_luxuries.insert(Resource::Cocoa);
                                    }
                                    Feature::Marsh => {
                                        allowed_luxuries.insert(Resource::Dyes);
                                        allowed_luxuries.insert(Resource::Sugar);
                                    }
                                    Feature::Floodplain => {
                                        allowed_luxuries.insert(Resource::Cotton);
                                        allowed_luxuries.insert(Resource::Incense);
                                    }
                                    _ => {}
                                }
                            } else {
                                match base_terrain {
                                    BaseTerrain::Grassland => {
                                        if tile.is_freshwater(self) {
                                            allowed_luxuries.insert(Resource::Sugar);
                                            allowed_luxuries.insert(Resource::Cotton);
                                            allowed_luxuries.insert(Resource::Wine);
                                        } else {
                                            allowed_luxuries.insert(Resource::Marble);
                                            allowed_luxuries.insert(Resource::Ivory);
                                            allowed_luxuries.insert(Resource::Cotton);
                                            allowed_luxuries.insert(Resource::Wine);
                                        }
                                    }
                                    BaseTerrain::Desert => {
                                        allowed_luxuries.insert(Resource::GoldOre);
                                        allowed_luxuries.insert(Resource::Marble);
                                        allowed_luxuries.insert(Resource::Incense);
                                    }
                                    BaseTerrain::Plain => {
                                        allowed_luxuries.insert(Resource::Marble);
                                        allowed_luxuries.insert(Resource::Ivory);
                                        allowed_luxuries.insert(Resource::Wine);
                                        allowed_luxuries.insert(Resource::Incense);
                                    }
                                    BaseTerrain::Tundra => {
                                        allowed_luxuries.insert(Resource::Furs);
                                        allowed_luxuries.insert(Resource::Silver);
                                        allowed_luxuries.insert(Resource::Marble);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        TerrainType::Mountain => {}
                        TerrainType::Hill => {
                            if base_terrain != BaseTerrain::Snow {
                                allowed_luxuries.insert(Resource::GoldOre);
                                allowed_luxuries.insert(Resource::Silver);
                                allowed_luxuries.insert(Resource::Gems);
                                if feature.is_none() {
                                    allowed_luxuries.insert(Resource::Marble);
                                }
                            }
                        }
                    }
                })
        }
        allowed_luxuries
    }

    // function AssignStartingPlots:GetIndicesForLuxuryType
    /// Get a list of indices for a given luxury type.
    ///
    /// Before use this function's return value, make sure [`TileMap::generate_luxury_tile_lists_at_city_site`] has been run.
    ///
    /// [`TileMap::generate_luxury_tile_lists_at_city_site`] will generate an array of 15 vectors of tiles that are available for placing Luxury resources.
    /// This function will return a list of indices for the given luxury type.
    /// The indices are used to access the vectors in the array.
    /// The order of the indices is important, because we try to place the Luxury resources in the order of the indices.
    /// If the first index is not available, we will try to place the Luxury resource in the second index, and so on.
    ///
    /// # Arguments
    /// - `resource`: The name of the luxury resource.
    pub fn get_indices_for_luxury_type(&self, resource: Resource) -> Vec<usize> {
        match resource {
            Resource::Whales | Resource::Pearls => vec![0],
            Resource::GoldOre => vec![3, 9, 4],
            Resource::Silver => vec![3, 4, 13, 11],
            Resource::Gems => vec![5, 6, 3, 7],
            Resource::Marble => vec![11, 9, 10, 3],
            Resource::Ivory => vec![10, 11],
            Resource::Furs => vec![13, 14],
            Resource::Dyes => vec![8, 7, 1],
            Resource::Spices => vec![7, 14, 1],
            Resource::Silk => vec![14, 7],
            Resource::Sugar => vec![1, 7, 2, 12],
            Resource::Cotton => vec![2, 12, 11],
            Resource::Wine => vec![10, 11, 12],
            Resource::Incense => vec![9, 2, 10],
            Resource::Copper => vec![3, 4, 11, 13],
            Resource::Salt => vec![10, 9, 13, 8],
            Resource::Citrus => vec![7, 5, 14, 2],
            Resource::Truffles => vec![14, 7, 1, 4],
            Resource::Crab => vec![0],
            Resource::Cocoa => vec![7, 5, 14],
            _ => vec![],
        }
    }

    fn num_placed_luxury_resources(&self, ruleset: &Ruleset) -> u32 {
        (0..Resource::LENGTH)
            .map(Resource::from_usize)
            .filter(|res| ruleset.tile_resources[res.as_str()].resource_type == "Luxury")
            .map(|res| self.placed_resource_count(res))
            .sum()
    }
}

// TODO: This function will implement in file 'map_parameters.rs' in the future.
fn get_region_luxury_target_numbers(
    world_size_type: WorldSizeType,
) -> [u32; MapParameters::MAX_CIVILIZATION_NUM as usize] {
    // This data was separated out to allow easy replacement in map scripts.
    // This table, indexed by civ-count, provides the target amount of luxuries to place in each region.
    // These vector's length is 22, which is the maximum number of civilizations in the game.
    // Max is one per region for all player counts at this size.
    match world_size_type {
        WorldSizeType::Duel => [1; MapParameters::MAX_CIVILIZATION_NUM as usize],
        WorldSizeType::Tiny => [
            0, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
        WorldSizeType::Small => [
            0, 3, 3, 3, 4, 4, 4, 3, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
        WorldSizeType::Standard => [
            0, 3, 3, 4, 4, 5, 5, 6, 5, 4, 4, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1,
        ],
        WorldSizeType::Large => [
            0, 3, 4, 4, 5, 5, 5, 6, 6, 7, 6, 5, 5, 4, 4, 3, 3, 2, 2, 2, 2, 2,
        ],
        WorldSizeType::Huge => [
            0, 4, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 7, 6, 6, 5, 5, 4, 4, 3, 3, 2,
        ],
    }
}

// function AssignStartingPlots:GetWorldLuxuryTargetNumbers
/// Returns an array of 2 numbers according to the world size and resource setting.
///
/// The first number represents the target for the total number of luxuries in the world.
/// This does **not** include the "second type" of luxuries added at each civilization's start location.
/// The "second type" of luxuries is the luxuries which is placed during in Process 5 of [`TileMap::place_luxury_resources`] function.
///
/// The second number influences the minimum number of random luxuries that should be placed.
/// It is important to note that it is just one factor in the formula for placing luxuries,
/// meaning other elements (such as civilization count) also contribute to the final result.
fn get_world_luxury_target_numbers(
    world_size_type: WorldSizeType,
    resource_setting: ResourceSetting,
) -> [u32; 2] {
    match resource_setting {
        ResourceSetting::Sparse => match world_size_type {
            WorldSizeType::Duel => [14, 3],
            WorldSizeType::Tiny => [24, 4],
            WorldSizeType::Small => [36, 4],
            WorldSizeType::Standard => [48, 5],
            WorldSizeType::Large => [60, 5],
            WorldSizeType::Huge => [76, 6],
        },

        ResourceSetting::Abundant => match world_size_type {
            WorldSizeType::Duel => [24, 3],
            WorldSizeType::Tiny => [40, 4],
            WorldSizeType::Small => [60, 4],
            WorldSizeType::Standard => [80, 5],
            WorldSizeType::Large => [100, 5],
            WorldSizeType::Huge => [128, 6],
        },

        _ => match world_size_type {
            WorldSizeType::Duel => [18, 3],
            WorldSizeType::Tiny => [30, 4],
            WorldSizeType::Small => [45, 4],
            WorldSizeType::Standard => [60, 5],
            WorldSizeType::Large => [75, 5],
            WorldSizeType::Huge => [95, 6],
        },
    }
}
