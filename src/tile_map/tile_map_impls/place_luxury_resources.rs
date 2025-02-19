use std::{cmp::max, collections::BTreeMap};

#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use rand::{distributions::WeightedIndex, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    tile_map::{tile::Tile, MapParameters, ResourceSetting, TileMap},
};

use crate::tile_map::Layer;

impl TileMap {
    // function AssignStartingPlots:PlaceLuxuries
    pub fn place_luxury_resources(&mut self, map_parameters: &MapParameters) {
        let mut luxury_low_fert_compensation = HashMap::new();
        let mut region_low_fert_compensation = vec![0; map_parameters.civilization_num as usize];

        // Place Luxuries at civ start locations.
        for region_index in 0..self.region_list.len() {
            let region = &self.region_list[region_index];
            let terrain_statistic = &self.region_list[region_index].terrain_statistic;
            let starting_tile = self.region_list[region_index].starting_tile.clone();
            let luxury_resource = self.region_list[region_index].luxury_resource.to_owned();
            // Determine number to place at the start location
            let mut num_to_place =
                if let ResourceSetting::LegendaryStart = map_parameters.resource_setting {
                    2
                } else {
                    1
                };
            // Low fertility per region rectangle plot, add a lux.
            if region.average_fertility() < 2.5 {
                num_to_place += 1;
                *luxury_low_fert_compensation
                    .entry(luxury_resource.to_owned())
                    .or_insert(0) += 1;
                region_low_fert_compensation[region_index] += 1;
            }

            let region_land_num = terrain_statistic.terrain_type_sum[&TerrainType::Hill]
                + terrain_statistic.terrain_type_sum[&TerrainType::Flatland];

            if (region.fertility_sum as f64 / region_land_num as f64) < 4.0 {
                num_to_place += 1;
                *luxury_low_fert_compensation
                    .entry(luxury_resource.to_owned())
                    .or_insert(0) += 1;
                region_low_fert_compensation[region_index] += 1;
            }

            let priority_list_indices_of_luxury =
                self.get_indices_for_luxury_type(&luxury_resource);
            let mut luxury_plot_lists = self.generate_luxury_plot_lists_at_city_site(
                map_parameters,
                starting_tile,
                2,
                false,
            );

            let mut priority_list_indices_iter = priority_list_indices_of_luxury.iter().peekable();

            let mut num_left_to_place = num_to_place;

            // First pass, checking only first two rings with a 50% ratio.
            while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                let i = *priority_list_indices_iter.next().unwrap();

                luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                num_left_to_place = self.place_specific_number_of_resources(
                    map_parameters,
                    Resource::Resource(luxury_resource.to_owned()),
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
                let mut luxury_plot_lists = self.generate_luxury_plot_lists_at_city_site(
                    map_parameters,
                    starting_tile,
                    3,
                    false,
                );

                let mut priority_list_indices_iter =
                    priority_list_indices_of_luxury.iter().peekable();

                // Second pass, checking three rings with a 100% ratio.
                while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                    let i = *priority_list_indices_iter.next().unwrap();

                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        map_parameters,
                        Resource::Resource(luxury_resource.to_owned()),
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
                *luxury_low_fert_compensation
                    .entry(luxury_resource.to_owned())
                    .or_insert(0) -= num_left_to_place as i32;
                region_low_fert_compensation[region_index] -= num_left_to_place as i32;
            }

            if num_left_to_place > 0 {
                // We'll attempt to place one source of a Luxury type assigned to random distribution.
                let mut randoms_to_place = 1;
                let resource_assigned_to_random = self
                    .luxury_resource_role
                    .resource_assigned_to_random
                    .clone();
                for luxury_resource in resource_assigned_to_random.iter() {
                    let priority_list_indices_of_luxury =
                        self.get_indices_for_luxury_type(&luxury_resource);

                    let mut priority_list_indices_iter =
                        priority_list_indices_of_luxury.iter().peekable();

                    while randoms_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                        let i = *priority_list_indices_iter.next().unwrap();

                        luxury_plot_lists[i as usize].shuffle(&mut self.random_number_generator);
                        randoms_to_place = self.place_specific_number_of_resources(
                            map_parameters,
                            Resource::Resource(luxury_resource.to_owned()),
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

        // Place Luxuries at City States.
        for i in 0..self.city_state_starting_tile_and_region_index.len() {
            let &(starting_tile, region_index) = &self.city_state_starting_tile_and_region_index[i];

            let allowed_luxuries =
                self.get_list_of_allowable_luxuries_at_city_site(map_parameters, starting_tile, 2);
            let mut lux_possible_for_cs = BTreeMap::new();
            let mut cs_only_types = Vec::new();
            for luxury in self
                .luxury_resource_role
                .resource_assigned_to_city_state
                .iter()
            {
                if allowed_luxuries.contains(luxury.as_str()) {
                    cs_only_types.push(luxury);
                }
            }

            cs_only_types.iter().for_each(|luxury| {
                lux_possible_for_cs.insert(luxury.to_string(), 75. / cs_only_types.len() as f64);
            });

            if self.luxury_resource_role.resource_assigned_to_random.len() > 0
                || region_index.is_some()
            {
                let mut random_types_allowed = Vec::new();
                for luxury in self
                    .luxury_resource_role
                    .resource_assigned_to_city_state
                    .iter()
                {
                    if allowed_luxuries.contains(luxury.as_str()) {
                        random_types_allowed.push(luxury);
                    }
                }

                if let Some(region_index) = region_index {
                    // Adding the region type in to the mix with the random types.
                    let num_allowed = random_types_allowed.len() + 1;
                    let luxury = &self.region_list[region_index].luxury_resource;
                    if allowed_luxuries.contains(luxury.as_str()) {
                        lux_possible_for_cs.insert(luxury.to_string(), 25. / num_allowed as f64);
                    }
                }

                random_types_allowed.iter().for_each(|luxury| {
                    lux_possible_for_cs
                        .insert(luxury.to_string(), 25. / random_types_allowed.len() as f64);
                });
            }

            if lux_possible_for_cs.len() > 0 {
                let lux_possible_for_cs: Vec<_> = lux_possible_for_cs.into_iter().collect();
                let dist =
                    WeightedIndex::new(lux_possible_for_cs.iter().map(|item| item.1)).unwrap();
                // Choose luxury type.
                let luxury_resource = lux_possible_for_cs
                    [dist.sample(&mut self.random_number_generator)]
                .0
                .to_owned();
                // Place luxury.
                let priority_list_indices_of_luxury =
                    self.get_indices_for_luxury_type(&luxury_resource);
                let mut luxury_plot_lists = self.generate_luxury_plot_lists_at_city_site(
                    map_parameters,
                    starting_tile,
                    2,
                    false,
                );

                let mut priority_list_indices_iter =
                    priority_list_indices_of_luxury.iter().peekable();

                let mut num_left_to_place = 1;

                while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                    let i = *priority_list_indices_iter.next().unwrap();

                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        map_parameters,
                        Resource::Resource(luxury_resource.to_owned()),
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

        // Place Regional Luxuries
        let world_size = 2; // TODO: This should be a parameter.
        for region_index in 0..self.region_list.len() {
            let luxury_resource = self.region_list[region_index].luxury_resource.clone();
            let luxury_assignment_count: u32 =
                self.luxury_assign_to_region_count[&luxury_resource.to_string()];
            let priority_list_indices_of_luxury =
                self.get_indices_for_luxury_type(&luxury_resource);

            let mut luxury_plot_lists =
                self.generate_luxury_plot_lists_in_region(map_parameters, region_index);

            let current_luxury_low_fert_compensation = *luxury_low_fert_compensation
                .entry(luxury_resource.to_string())
                .or_insert(0);

            let mut priority_list_indices_iter = priority_list_indices_of_luxury.iter().peekable();

            // Calibrate the number of luxuries per region based on the world size and the number of civilizations.
            // The number of luxuries per region should be highest when the number of civilizations is closest to the "default" value for that map size.
            let target_list = get_region_luxury_target_numbers(world_size);
            let mut target_num = ((target_list[map_parameters.civilization_num as usize] as f64
                + 0.5 * current_luxury_low_fert_compensation as f64)
                / luxury_assignment_count as f64) as i32;

            target_num -= region_low_fert_compensation[region_index];

            match map_parameters.resource_setting {
                ResourceSetting::Sparse => target_num -= 1,
                ResourceSetting::Abundant => target_num += 1,
                _ => (),
            }

            let num_this_luxury_to_place = max(1, target_num) as u32;
            let mut num_left_to_place = num_this_luxury_to_place;

            while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                let i = *priority_list_indices_iter.next().unwrap();

                luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                num_left_to_place = self.place_specific_number_of_resources(
                    map_parameters,
                    Resource::Resource(luxury_resource.to_owned()),
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

        // Place Random Luxuries
        let num_random_luxury_types = self.luxury_resource_role.resource_assigned_to_random.len();
        if num_random_luxury_types > 0 {
            // This table defines the target total number of luxuries placed in the world, excluding
            // the "extra types" of luxuries placed at start locations. These targets are approximate.
            // A random factor is added based on the number of civilizations.
            // Any difference between regional and city-state luxuries placed and the target
            // is compensated by randomly placed luxuries that are distributed.
            let world_size_data = get_region_luxury_target_numbers(world_size);
            let target_luxury_for_this_world_size = world_size_data[0];
            let loop_target = world_size_data[1];
            let extra_luxury = self
                .random_number_generator
                .gen_range(0..map_parameters.civilization_num);
            // TODO: Should be edited in the future.
            let num_random_luxury_target = target_luxury_for_this_world_size + extra_luxury /* - self.totalLuxPlacedSoFar */;

            let mut num_this_luxury_to_place;

            // This table weights the amount of random luxuries to place, with first-selected getting heavier weighting.
            let random_lux_ratios_table = [
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
                let luxury_resource =
                    self.luxury_resource_role.resource_assigned_to_random[i].clone();

                let priority_list_indices_of_luxury =
                    self.get_indices_for_luxury_type(&luxury_resource);

                if self.luxury_resource_role.resource_assigned_to_random.len() * 3
                    > num_random_luxury_target as usize
                {
                    num_this_luxury_to_place = 3;
                } else if self.luxury_resource_role.resource_assigned_to_random.len() > 8 {
                    num_this_luxury_to_place =
                        max(3, (num_random_luxury_target as f64 / 10.).ceil() as u32);
                } else {
                    let lux_minimum = max(3, loop_target - i as u32);
                    let luxury_share_of_remaining = (num_random_luxury_target as f64
                        * random_lux_ratios_table[num_random_luxury_types - 1][i])
                        .ceil() as u32;
                    num_this_luxury_to_place = max(lux_minimum, luxury_share_of_remaining);
                }

                let mut priority_list_indices_iter =
                    priority_list_indices_of_luxury.iter().peekable();

                let mut current_list = self.generate_global_resource_plot_lists(map_parameters);
                // Place this luxury type.
                let mut num_left_to_place = num_this_luxury_to_place;

                while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                    let i = *priority_list_indices_iter.next().unwrap();

                    current_list[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        map_parameters,
                        Resource::Resource(luxury_resource.to_string()),
                        1,
                        num_left_to_place,
                        0.25,
                        Some(Layer::Luxury),
                        4,
                        6,
                        &current_list[i],
                    );
                }
            }
        }

        // For resource settings other than "Sparse", add a second luxury type at starting locations.
        // This second luxury type will be selected in the following order:
        //   1. Random types, if available.
        //   2. CS types, if necessary.
        //   3. Types from other regions, as a final fallback.
        // Marble is included in the list of possible types to be placed.
        if map_parameters.resource_setting != ResourceSetting::Sparse {
            for region_index in 0..self.region_list.len() {
                let starting_tile = self.region_list[region_index].starting_tile;
                let allowed_luxuries = self.get_list_of_allowable_luxuries_at_city_site(
                    map_parameters,
                    starting_tile,
                    2,
                );

                let mut candidate_luxury_types = Vec::new();

                // See if any Random types are eligible.
                for luxury in self.luxury_resource_role.resource_assigned_to_random.iter() {
                    if allowed_luxuries.contains(luxury) {
                        candidate_luxury_types.push(luxury.to_string());
                    }
                }

                // Check to see if any Special Case luxuries are eligible. Disallow if Strategic Balance resource setting.
                if map_parameters.resource_setting != ResourceSetting::StrategicBalance {
                    for luxury in self
                        .luxury_resource_role
                        .resource_assigned_to_special_case
                        .iter()
                    {
                        if allowed_luxuries.contains(luxury) {
                            candidate_luxury_types.push(luxury.to_string());
                        }
                    }
                }

                let mut use_this_luxury = None;

                if candidate_luxury_types.len() > 0 {
                    use_this_luxury =
                        candidate_luxury_types.choose(&mut self.random_number_generator);
                } else {
                    // No Random or Special Case luxuries available. See if any City State types are eligible.
                    for luxury in self
                        .luxury_resource_role
                        .resource_assigned_to_city_state
                        .iter()
                    {
                        if allowed_luxuries.contains(luxury) {
                            candidate_luxury_types.push(luxury.to_string());
                        }
                    }

                    if candidate_luxury_types.len() > 0 {
                        use_this_luxury =
                            candidate_luxury_types.choose(&mut self.random_number_generator);
                    } else {
                        // No City State luxuries available. Use a type from another region.
                        let region_luxury = &self.region_list[region_index].luxury_resource;
                        for luxury in self
                            .luxury_resource_role
                            .resource_assigned_to_regions
                            .iter()
                        {
                            if allowed_luxuries.contains(luxury) && luxury != region_luxury {
                                candidate_luxury_types.push(luxury.to_string());
                            }
                        }
                        if candidate_luxury_types.len() > 0 {
                            use_this_luxury =
                                candidate_luxury_types.choose(&mut self.random_number_generator);
                        }
                    }
                }

                if let Some(luxury) = use_this_luxury {
                    let priority_list_indices_of_luxury = self.get_indices_for_luxury_type(&luxury);

                    let mut luxury_plot_lists = self.generate_luxury_plot_lists_at_city_site(
                        map_parameters,
                        starting_tile,
                        2,
                        false,
                    );

                    let mut priority_list_indices_iter =
                        priority_list_indices_of_luxury.iter().peekable();

                    let mut num_left_to_place = 1;
                    while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                        let i = *priority_list_indices_iter.next().unwrap();

                        luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                        num_left_to_place = self.place_specific_number_of_resources(
                            map_parameters,
                            Resource::Resource(luxury.to_string()),
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

        if self
            .luxury_resource_role
            .resource_assigned_to_special_case
            .len()
            > 0
        {
            let luxury_list = self
                .luxury_resource_role
                .resource_assigned_to_special_case
                .clone();
            for luxury in luxury_list {
                match luxury.as_str() {
                    "Marble" => {
                        self.place_marble(map_parameters);
                    }
                    _ => {}
                }
            }
        }
    }

    fn place_marble(&mut self, map_parameters: &MapParameters) {
        let luxury_resource = "Marble".to_string();
        let marble_already_placed: u32 = self.placed_resource_count(&luxury_resource);

        let marble_target = match map_parameters.resource_setting {
            ResourceSetting::Sparse => (map_parameters.civilization_num as f32 * 0.5).ceil() as i32,
            ResourceSetting::Abundant => {
                (map_parameters.civilization_num as f32 * 0.9).ceil() as i32
            }
            _ => (map_parameters.civilization_num as f32 * 0.75).ceil() as i32,
        };

        let mut marble_tile_list = Vec::new();
        self.iter_tiles().for_each(|tile| {
            let terrain_type = tile.terrain_type(self);
            let base_terrain = tile.base_terrain(self);
            let feature = tile.feature(self);

            match terrain_type {
                TerrainType::Water => {}
                TerrainType::Flatland => {
                    if feature == None {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                if !tile.is_freshwater(self, map_parameters) {
                                    marble_tile_list.push(tile);
                                }
                            }
                            BaseTerrain::Desert => {
                                marble_tile_list.push(tile);
                            }
                            BaseTerrain::Plain => {
                                if !tile.is_freshwater(self, map_parameters) {
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
                    if base_terrain != BaseTerrain::Snow {
                        if feature == None {
                            marble_tile_list.push(tile);
                        }
                    }
                }
            }
        });

        let num_marble_to_place = max(2, marble_target - marble_already_placed as i32) as u32;

        let mut num_left_to_place = num_marble_to_place;
        if marble_tile_list.len() == 0 {
            // println!("No eligible plots available to place Marble!");
            return;
        }

        marble_tile_list.shuffle(&mut self.random_number_generator);

        let mut marble_tile_list_iter = marble_tile_list.iter().peekable();

        // Place the marble.
        while num_left_to_place > 0 && marble_tile_list_iter.peek().is_some() {
            let tile = *marble_tile_list_iter.next().unwrap();
            if self.resource_query[tile.index()] == None
                && self.layer_data[&Layer::Marble][tile.index()] == 0
                && self.layer_data[&Layer::Luxury][tile.index()] == 0
            {
                // Placing this resource in this plot.
                self.resource_query[tile.index()] =
                    Some((Resource::Resource(luxury_resource.to_string()), 1));
                // self.total_lux_placed_so_far += 1;
                num_left_to_place -= 1;
                // println!("Still need to place {} more units of Marble.", num_left_to_place);
                self.place_resource_impact(map_parameters, tile, Layer::Luxury, 1);
                self.place_resource_impact(map_parameters, tile, Layer::Marble, 6);
            }
        }

        if num_left_to_place > 0 {
            println!("Failed to place {} units of Marble.", num_left_to_place);
        }
    }

    // function AssignStartingPlots:GenerateGlobalResourcePlotLists
    // TODO: We will rename this function in the future because it is only used for luxury resources.
    // Whether we should shuffle every vec in the array before returning is a problem that needs to be considered.
    fn generate_global_resource_plot_lists(
        &self,
        map_parameters: &MapParameters,
    ) -> [Vec<Tile>; 15] {
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

        self.iter_tiles().for_each(|tile| {
            if !self.player_collision_data[tile.index()] && tile.resource(self).is_none() {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                let feature = tile.feature(self);

                match terrain_type {
                    TerrainType::Water => {
                        if base_terrain == BaseTerrain::Coast
                            && feature != Some(Feature::Ice)
                            && feature != Some(Feature::Atoll)
                        {
                            if tile
                                .neighbor_tiles(map_parameters)
                                .iter()
                                .any(|neighbor_tile| {
                                    neighbor_tile.terrain_type(self) != TerrainType::Water
                                })
                            {
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
                                    if tile.is_freshwater(self, map_parameters) {
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
                            if feature == None {
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

    // AssignStartingPlots:GenerateLuxuryPlotListsAtCitySite
    /// Removes the feature ice from the tile and returns the region info for luxury.
    /// TODO: `Remove the feature ice` should be implemented as a separate method, rather than being included in the current method.
    /// TODO: Sometimes this function is used for strategic resources, so the name should be changed.
    pub fn generate_luxury_plot_lists_at_city_site(
        &mut self,
        map_parameters: &MapParameters,
        tile: Tile,
        radius: u32,
        remove_feature_ice: bool,
    ) -> [Vec<Tile>; 15] {
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
                tile
                .tiles_at_distance(ripple_radius, map_parameters)
                .into_iter()
                .for_each(|tile_at_distance| {
                    let terrain_type = tile_at_distance.terrain_type(self);
                    let base_terrain = tile_at_distance.base_terrain(self);
                    let feature = tile_at_distance.feature(self);
                    // If Ice removal is enabled, then remove ice from this tile.
                    if remove_feature_ice && feature == Some(Feature::Ice) {
                        self.feature_query[tile_at_distance.index()] = None;
                    } else {
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
                                                region_tundra_flat_including_forest_tile_list.push(tile_at_distance);
                                            } else {
                                                region_forest_flat_but_not_tundra_tile_list.push(tile_at_distance);
                                            }
                                        },
                                        Feature::Jungle => {
                                            region_jungle_flat_tile_list.push(tile_at_distance);
                                        },
                                        Feature::Marsh => {
                                            region_marsh_tile_list.push(tile_at_distance);
                                        },
                                        Feature::Floodplain => {
                                            region_flood_plain_tile_list.push(tile_at_distance);
                                        },
                                        _ => {}
                                    }
                                } else {
                                    match base_terrain {
                                        BaseTerrain::Grassland => {
                                            if tile_at_distance.is_freshwater(self, map_parameters){
                                                region_fresh_water_grass_flat_no_feature_tile_list.push(tile_at_distance);
                                            } else {
                                                region_dry_grass_flat_no_feature_tile_list.push(tile_at_distance);
                                            }
                                        },
                                        BaseTerrain::Desert => {
                                            region_desert_flat_no_feature_tile_list.push(tile_at_distance);
                                        },
                                        BaseTerrain::Plain => {
                                            region_plain_flat_no_feature_tile_list.push(tile_at_distance);
                                        },
                                        BaseTerrain::Tundra => {
                                            region_tundra_flat_including_forest_tile_list.push(tile_at_distance);
                                        },
                                        _ => {}
                                    }
                                }
                            }
                            TerrainType::Mountain => {}
                            TerrainType::Hill => {
                                if base_terrain != BaseTerrain::Snow {
                                    if feature == None {
                                        region_hill_open_tile_list.push(tile_at_distance);
                                    } else if feature == Some(Feature::Forest) {
                                        region_hill_forest_tile_list
                                            .push(tile_at_distance);
                                        region_hill_covered_tile_list
                                            .push(tile_at_distance);
                                    } else if feature == Some(Feature::Jungle) {
                                        region_hill_jungle_tile_list
                                            .push(tile_at_distance);
                                        region_hill_covered_tile_list
                                            .push(tile_at_distance);
                                    }
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

    // function AssignStartingPlots:GenerateLuxuryPlotListsInRegion
    fn generate_luxury_plot_lists_in_region(
        &self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> [Vec<Tile>; 15] {
        let rectangle = self.region_list[region_index].rectangle;

        let landmass_id = self.region_list[region_index].landmass_id;

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

        rectangle.iter_tiles(map_parameters).for_each(|tile| {
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
                                .neighbor_tiles(map_parameters)
                                .iter()
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
                                if tile.is_freshwater(self, map_parameters) {
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
                        if feature == None {
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
    fn get_list_of_allowable_luxuries_at_city_site(
        &self,
        map_parameters: &MapParameters,
        city_site: Tile,
        radius: u32,
    ) -> HashSet<String> {
        let mut allowed_luxuries = HashSet::new();
        for ripple_radius in 1..=radius {
            city_site
                .tiles_at_distance(ripple_radius, map_parameters)
                .iter()
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
                                allowed_luxuries.insert("Whales".to_string());
                                allowed_luxuries.insert("Pearls".to_string());
                            }
                        }
                        TerrainType::Flatland => {
                            if let Some(feature) = feature {
                                match feature {
                                    Feature::Forest => {
                                        allowed_luxuries.insert("Furs".to_string());
                                        allowed_luxuries.insert("Dyes".to_string());
                                        if base_terrain == BaseTerrain::Tundra {
                                            allowed_luxuries.insert("Silver".to_string());
                                        } else {
                                            allowed_luxuries.insert("Spices".to_string());
                                            allowed_luxuries.insert("Silk".to_string());
                                        }
                                    }
                                    Feature::Jungle => {
                                        allowed_luxuries.insert("Gems".to_string());
                                        allowed_luxuries.insert("Dyes".to_string());
                                        allowed_luxuries.insert("Spices".to_string());
                                        allowed_luxuries.insert("Silk".to_string());
                                        allowed_luxuries.insert("Sugar".to_string());
                                        allowed_luxuries.insert("Cocoa".to_string());
                                    }
                                    Feature::Marsh => {
                                        allowed_luxuries.insert("Dyes".to_string());
                                        allowed_luxuries.insert("Sugar".to_string());
                                    }
                                    Feature::Floodplain => {
                                        allowed_luxuries.insert("Cotton".to_string());
                                        allowed_luxuries.insert("Incense".to_string());
                                    }
                                    _ => {}
                                }
                            } else {
                                match base_terrain {
                                    BaseTerrain::Grassland => {
                                        if tile.is_freshwater(self, map_parameters) {
                                            allowed_luxuries.insert("Sugar".to_string());
                                            allowed_luxuries.insert("Cotton".to_string());
                                            allowed_luxuries.insert("Wine".to_string());
                                        } else {
                                            allowed_luxuries.insert("Marble".to_string());
                                            allowed_luxuries.insert("Ivory".to_string());
                                            allowed_luxuries.insert("Cotton".to_string());
                                            allowed_luxuries.insert("Wine".to_string());
                                        }
                                    }
                                    BaseTerrain::Desert => {
                                        allowed_luxuries.insert("Gold Ore".to_string());
                                        allowed_luxuries.insert("Marble".to_string());
                                        allowed_luxuries.insert("Incense".to_string());
                                    }
                                    BaseTerrain::Plain => {
                                        allowed_luxuries.insert("Marble".to_string());
                                        allowed_luxuries.insert("Ivory".to_string());
                                        allowed_luxuries.insert("Wine".to_string());
                                        allowed_luxuries.insert("Incense".to_string());
                                    }
                                    BaseTerrain::Tundra => {
                                        allowed_luxuries.insert("Furs".to_string());
                                        allowed_luxuries.insert("Silver".to_string());
                                        allowed_luxuries.insert("Marble".to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        TerrainType::Mountain => {}
                        TerrainType::Hill => {
                            if base_terrain != BaseTerrain::Snow {
                                allowed_luxuries.insert("Gold Ore".to_string());
                                allowed_luxuries.insert("Silver".to_string());
                                allowed_luxuries.insert("Gems".to_string());
                                if feature.is_none() {
                                    allowed_luxuries.insert("Marble".to_string());
                                }
                            }
                        }
                    }
                })
        }
        allowed_luxuries
    }

    // function AssignStartingPlots:GetIndicesForLuxuryType
    /// Before running this function, make sure [`TileMap::generate_luxury_plot_lists_at_city_site`] has been run.
    /// Running [`TileMap::generate_luxury_plot_lists_at_city_site`] will generate the lists of plots that are available for placing Luxury resources.
    /// The lists are stored in `luxury_plot_lists`， which is vectors of vectors of `TileIndex`.
    /// And then this function's purpose is to get the indices of the vectors in `luxury_plot_lists` that contain the plots that are available for placing the Luxury resource.
    /// The returned indices are used to access the vectors in `luxury_plot_lists` and get the plots that are available for placing the Luxury resource.
    /// The order of the indices is important, because the first index is the primary index, the second index is the secondary index, the third index is the tertiary index, and the fourth index is the quaternary index.
    pub fn get_indices_for_luxury_type(&self, resource: &str) -> Vec<usize> {
        let vec = match resource {
            "Whales" | "Pearls" => vec![0],
            "Gold Ore" => vec![3, 9, 4],
            "Silver" => vec![3, 4, 13, 11],
            "Gems" => vec![5, 6, 3, 7],
            "Marble" => vec![11, 9, 10, 3],
            "Ivory" => vec![10, 11],
            "Furs" => vec![13, 14],
            "Dyes" => vec![8, 7, 1],
            "Spices" => vec![7, 14, 1],
            "Silk" => vec![14, 7],
            "Sugar" => vec![1, 7, 2, 12],
            "Cotton" => vec![2, 12, 11],
            "Wine" => vec![10, 11, 12],
            "Incense" => vec![9, 2, 10],
            "Copper" => vec![3, 4, 11, 13],
            "Salt" => vec![10, 9, 13, 8],
            "Citrus" => vec![7, 5, 14, 2],
            "Truffles" => vec![14, 7, 1, 4],
            "Crab" => vec![0],
            "Cocoa" => vec![7, 5, 14],
            _ => vec![],
        };

        vec
    }
}

/// TODO: This function will implement in file 'map_parameters.rs' in the future.
fn get_region_luxury_target_numbers(world_size: i32) -> Vec<u32> {
    // This data was separated out to allow easy replacement in map scripts.
    // This table, indexed by civ-count, provides the target amount of luxuries to place in each region.
    // These vector's length is 22, which is the maximum number of civilizations in the game.

    let duel_values = vec![1; 22]; // Max is one per region for all player counts at this size.

    let tiny_values = vec![
        0, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ];

    let small_values = vec![
        0, 3, 3, 3, 4, 4, 4, 3, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ];

    let standard_values = vec![
        0, 3, 3, 4, 4, 5, 5, 6, 5, 4, 4, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1,
    ];

    let large_values = vec![
        0, 3, 4, 4, 5, 5, 5, 6, 6, 7, 6, 5, 5, 4, 4, 3, 3, 2, 2, 2, 2, 2,
    ];

    let huge_values = vec![
        0, 4, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 7, 6, 6, 5, 5, 4, 4, 3, 3, 2,
    ];

    // Map the world size ID to the corresponding target values
    let worldsizes: HashMap<i32, Vec<u32>> = vec![
        (1, duel_values),
        (2, tiny_values),
        (3, small_values),
        (4, standard_values),
        (5, large_values),
        (6, huge_values),
    ]
    .into_iter()
    .collect();

    // Return the target list based on the provided world size
    if let Some(target_list) = worldsizes.get(&world_size) {
        target_list.clone()
    } else {
        Vec::new() // Return an empty vector if the world size is not found
    }
}
