use std::{
    cmp::{max, min},
    collections::BTreeSet,
};

use std::collections::{HashMap, HashSet};

use rand::{distributions::WeightedIndex, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    grid::{
        hex::{HexOrientation, Offset},
        OffsetCoordinate,
    },
    ruleset::Ruleset,
    tile_map::{tile::Tile, Layer, MapParameters, ResourceSetting, TileMap},
};

use super::generate_regions::{Rectangle, Region, RegionType, StartLocationCondition, TileType};

impl TileMap {
    pub fn start_plot_system(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.choose_locations(map_parameters);

        self.balance_and_assign(map_parameters, ruleset);

        self.place_natural_wonders(map_parameters, ruleset);

        self.place_resources_and_city_states(map_parameters, ruleset);
    }

    // function AssignStartingPlots:PlaceResourcesAndCityStates
    pub fn place_resources_and_city_states(
        &mut self,
        map_parameters: &MapParameters,
        ruleset: &Ruleset,
    ) {
        self.assign_luxury_roles(map_parameters);

        self.place_city_states(map_parameters, ruleset);

        /* -- Generate global plot lists for resource distribution.
        self:GenerateGlobalResourcePlotLists() */

        self.place_luxury_resources(map_parameters, ruleset);

        self.place_strategic_resources(map_parameters);

        self.place_bonus_resources(map_parameters);

        self.normalize_city_state_locations(map_parameters);

        self.fix_sugar_jungles();

        self.recalculate_areas(map_parameters);
    }

    /// Fix Sugar graphics. That because in origin CIV5, `Sugar` could not be made visible enough in jungle, so turn any sugar jungle to marsh.
    ///
    /// Change all the terrain which both has feature `Jungle` and resource `Sugar` to a `Flatland` terrain
    /// which has base terrain `Grassland` and feature `Marsh`.
    fn fix_sugar_jungles(&mut self) {
        self.iter_tiles().for_each(|tile| {
            if tile
                .resource(self)
                .map_or(false, |(resource, _)| resource.name() == "Sugar")
                && tile.feature(self) == Some(Feature::Jungle)
            {
                self.terrain_type_query[tile.index()] = TerrainType::Flatland;
                self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
                self.feature_query[tile.index()] = Some(Feature::Marsh);
            }
        })
    }

    fn normalize_city_state_locations(&mut self, map_parameters: &MapParameters) {
        let starting_tile_list: Vec<Tile> = self
            .city_state_and_starting_tile
            .values()
            .map(|&starting_tile| starting_tile)
            .collect();
        for starting_tile in starting_tile_list {
            self.normalize_city_state(map_parameters, starting_tile);
        }
    }

    // function AssignStartingPlots:NormalizeCityState
    fn normalize_city_state(&mut self, map_parameters: &MapParameters, tile: Tile) {
        let mut inner_four_food = 0;
        let mut inner_three_food = 0;
        let mut inner_two_food = 0;
        let mut inner_hills = 0;
        let mut inner_forest = 0;
        let mut inner_one_hammer = 0;
        let mut inner_ocean = 0;

        let mut outer_four_food = 0;
        let mut outer_three_food = 0;
        let mut outer_two_food = 0;
        let mut outer_ocean = 0;

        let mut inner_can_have_bonus = 0;
        let mut outer_can_have_bonus = 0;
        let mut inner_bad_tiles = 0;
        let mut outer_bad_tiles = 0;

        let mut num_food_bonus_needed = 0;

        // Data Chart for early game tile potentials
        //
        // 4F: Flood Plains, Grass on fresh water (includes forest and marsh).
        // 3F: Dry Grass, Plains on fresh water (includes forest and jungle), Tundra on fresh water (includes forest), Oasis.
        // 2F: Dry Plains, Lake, all remaining Jungles.
        //
        // 1H: Plains, Jungle on Plains

        // Evaluate First Ring
        let mut neighbor_tiles = tile.neighbor_tiles(map_parameters);

        neighbor_tiles.iter().for_each(|neighbor_tile| {
            let terrain_type = neighbor_tile.terrain_type(self);
            let base_terrain = neighbor_tile.base_terrain(self);
            let feature = neighbor_tile.feature(self);
            match terrain_type {
                TerrainType::Mountain => {
                    inner_bad_tiles += 1;
                }
                TerrainType::Water => {
                    if feature == Some(Feature::Ice) {
                        inner_bad_tiles += 1;
                    } else if base_terrain == BaseTerrain::Lake {
                        inner_two_food += 1;
                    } else if base_terrain == BaseTerrain::Coast {
                        inner_ocean += 1;
                        inner_can_have_bonus += 1;
                    }
                }
                _ => {
                    if terrain_type == TerrainType::Hill {
                        inner_hills += 1;
                        if feature == Some(Feature::Jungle) {
                            inner_two_food += 1;
                            inner_can_have_bonus += 1;
                        } else if feature == Some(Feature::Forest) {
                            inner_can_have_bonus += 1;
                        }
                    } else if tile.is_freshwater(self, map_parameters) {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                inner_four_food += 1;
                                if feature != Some(Feature::Marsh) {
                                    inner_can_have_bonus += 1;
                                }
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                }
                            }
                            BaseTerrain::Desert => {
                                inner_can_have_bonus += 1;
                                if feature == Some(Feature::Floodplain) {
                                    inner_four_food += 1;
                                } else {
                                    inner_bad_tiles += 1;
                                }
                            }
                            BaseTerrain::Plain => {
                                inner_three_food += 1;
                                inner_can_have_bonus += 1;
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
                        // Dry Flatlands
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                inner_three_food += 1;
                                if feature != Some(Feature::Marsh) {
                                    inner_can_have_bonus += 1;
                                }
                                if feature == Some(Feature::Forest) {
                                    inner_forest += 1;
                                }
                            }
                            BaseTerrain::Desert => {
                                inner_bad_tiles += 1;
                                inner_can_have_bonus += 1;
                            }
                            BaseTerrain::Plain => {
                                inner_two_food += 1;
                                inner_can_have_bonus += 1;
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

        // Evaluate Second Ring
        let mut tiles_at_distance_two = tile.tiles_at_distance(2, map_parameters);

        tiles_at_distance_two
            .iter()
            .for_each(|tile_at_distance_two| {
                let terrain_type = tile_at_distance_two.terrain_type(self);
                let base_terrain = tile_at_distance_two.base_terrain(self);
                let feature = tile_at_distance_two.feature(self);
                match terrain_type {
                    TerrainType::Mountain => {
                        outer_bad_tiles += 1;
                    }
                    TerrainType::Water => {
                        if feature == Some(Feature::Ice) {
                            outer_bad_tiles += 1;
                        } else if base_terrain == BaseTerrain::Lake {
                            outer_two_food += 1;
                        } else if base_terrain == BaseTerrain::Coast {
                            outer_ocean += 1;
                            outer_can_have_bonus += 1;
                        }
                    }
                    _ => {
                        if terrain_type == TerrainType::Hill {
                            if feature == Some(Feature::Jungle) {
                                outer_two_food += 1;
                                outer_can_have_bonus += 1;
                            } else if feature == Some(Feature::Forest) {
                                outer_can_have_bonus += 1;
                            }
                        } else if tile_at_distance_two.is_freshwater(self, map_parameters) {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    outer_four_food += 1;
                                    if feature != Some(Feature::Marsh) {
                                        outer_can_have_bonus += 1;
                                    }
                                }
                                BaseTerrain::Desert => {
                                    outer_can_have_bonus += 1;
                                    if feature == Some(Feature::Floodplain) {
                                        outer_four_food += 1;
                                    } else {
                                        outer_bad_tiles += 1;
                                    }
                                }
                                BaseTerrain::Plain => {
                                    outer_three_food += 1;
                                    outer_can_have_bonus += 1;
                                }
                                BaseTerrain::Tundra => {
                                    outer_three_food += 1;
                                    outer_can_have_bonus += 1;
                                }
                                BaseTerrain::Snow => {
                                    outer_bad_tiles += 1;
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        } else {
                            // Dry Flatlands
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    outer_three_food += 1;
                                    if feature != Some(Feature::Marsh) {
                                        outer_can_have_bonus += 1;
                                    }
                                }
                                BaseTerrain::Desert => {
                                    outer_bad_tiles += 1;
                                    outer_can_have_bonus += 1;
                                }
                                BaseTerrain::Plain => {
                                    outer_two_food += 1;
                                    outer_can_have_bonus += 1;
                                }
                                BaseTerrain::Tundra => {
                                    outer_can_have_bonus += 1;
                                    if feature != Some(Feature::Forest) {
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
        let mut hammer_score = (4 * inner_hills) + (2 * inner_forest) + inner_one_hammer;
        if hammer_score < 4 {
            neighbor_tiles.shuffle(&mut self.random_number_generator);
            for tile in neighbor_tiles.iter() {
                // Attempt to place a Hill at the currently chosen tile.
                let placed_hill = self.attempt_to_place_hill_at_tile(map_parameters, tile);
                if placed_hill {
                    hammer_score += 4;
                    break;
                }
            }
        }

        let inner_food_score = (4 * inner_four_food) + (2 * inner_three_food) + inner_two_food;
        let outer_food_score = (4 * outer_four_food) + (2 * outer_three_food) + outer_two_food;
        let total_food_score = inner_food_score + outer_food_score;

        if total_food_score < 12 || inner_food_score < 4 {
            num_food_bonus_needed = 2;
        } else if total_food_score < 16 && inner_food_score < 9 {
            num_food_bonus_needed = 1;
        }

        if num_food_bonus_needed > 0 {
            let max_bonuses_possible = inner_can_have_bonus + outer_can_have_bonus;
            let mut inner_placed = 0;
            let mut outer_placed = 0;

            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tiles.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tiles_at_distance_two.shuffle(&mut self.random_number_generator);

            let mut first_ring_iter = neighbor_tiles.iter().peekable();
            let mut second_ring_iter = tiles_at_distance_two.iter().peekable();

            let mut allow_oasis = true; // Permanent flag. (We don't want to place more than one Oasis per location).
            while num_food_bonus_needed > 0 {
                if inner_placed < 2 && inner_can_have_bonus > 0 && first_ring_iter.peek().is_some()
                {
                    // Add bonus to inner ring.
                    while let Some(&tile) = first_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                &tile,
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
                } else if (inner_placed + outer_placed < 4 && outer_can_have_bonus > 0)
                    && second_ring_iter.peek().is_some()
                {
                    // Add bonus to second ring.
                    while let Some(tile) = second_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                &tile,
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
                } else {
                    break;
                }
            }
        }
    }

    // function AssignStartingPlots:ProcessResourceList
    /// Placing bonus or strategic resources on the map based on the given parameters.
    /// It iterates through the list of plots and places resources on eligible plots based on the
    /// resource type, quantity, and radius.
    ///
    /// # Arguments
    ///
    /// * `map_parameters` - A reference to the map parameters.
    /// * `frequency` - The frequency of resource placement.
    /// * `layer` - The layer on which the resource will be placed.
    /// * `plot_list` - A vector of tiles representing the plots where resources can be placed. Before using this argument, make sure the vector has been shuffled.
    /// * `resource_list_to_place` - A vector of resource to place, which contains the resource type,
    /// quantity, minimum radius, and maximum radius for each resource.
    ///
    /// # Panics
    ///
    /// This function will panic if the layer is not `Layer::Bonus` or `Layer::Strategic`. That means if you place luxury resources, it will panic.
    ///
    /// # Notice
    ///
    /// Although in the original CIV5, this function has some code about placing luxury resources, but in fact, it is never used to place luxury resources. So, we forbid placing luxury resources in this function.
    /// If you want to place luxury resources, please use [`TileMap::place_specific_number_of_resources`].
    pub fn process_resource_list(
        &mut self,
        map_parameters: &MapParameters,
        frequency: f64,
        layer: Layer,
        plot_list: &[Tile],
        resource_list_to_place: &[ResourceToPlace],
    ) {
        if plot_list.is_empty() {
            return;
        }

        assert!(layer == Layer::Bonus || layer == Layer::Strategic, "This function is only used to place strategic and bonus resources on the map, not luxury resources.");

        let resource_weight = resource_list_to_place
            .iter()
            .map(|resource| resource.weight)
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(resource_weight).unwrap();

        let num_total_plots = plot_list.len();
        let num_resources_to_place = (num_total_plots as f64 / frequency).ceil() as u32;

        let mut plot_list_iter = plot_list.iter();

        // Main loop
        for _ in 0..num_resources_to_place {
            let current_resource_to_place =
                &resource_list_to_place[dist.sample(&mut self.random_number_generator)];
            let resource = &current_resource_to_place.resource;
            let quantity = current_resource_to_place.quantity;
            let min_radius = current_resource_to_place.min_radius;
            let max_radius = current_resource_to_place.max_radius;
            let radius = self
                .random_number_generator
                .gen_range(min_radius..=max_radius);
            // First pass: Seek the first eligible 0 value on impact matrix
            while let Some(&tile) = plot_list_iter.next() {
                if self.layer_data[&layer][tile.index()] == 0 && tile.resource(self).is_none() {
                    self.resource_query[tile.index()] =
                        Some((Resource::Resource(resource.to_string()), quantity));
                    self.place_resource_impact(map_parameters, tile, layer, radius);
                    break;
                }
            }

            // Completed first pass of plot_list, now change to seeking lowest value instead of zero value.
            // If no eligible 0 value is found, second pass: Seek the lowest value (value < 98) on the impact matrix
            if plot_list_iter.next().is_none() {
                let best_plot = plot_list
                    .iter()
                    .filter(|&&tile| {
                        self.layer_data[&layer][tile.index()] < 98 && tile.resource(self).is_none()
                    })
                    .min_by_key(|&&tile| self.layer_data[&layer][tile.index()])
                    .cloned();
                if let Some(tile) = best_plot {
                    self.resource_query[tile.index()] =
                        Some((Resource::Resource(resource.to_string()), quantity));
                    self.place_resource_impact(map_parameters, tile, layer, radius);
                }
            }
        }
    }

    // function AssignStartingPlots:GetMajorStrategicResourceQuantityValues
    // TODO: This function should be implemented in future.
    /// Determines the quantity per tile for each strategic resource's major deposit size.
    ///
    /// # Notice
    /// In some maps, If we cannot place oil in the sea, we should increase the resource amounts on land to compensate.
    pub fn get_major_strategic_resource_quantity_values(
        resource_setting: ResourceSetting,
    ) -> (u32, u32, u32, u32, u32, u32) {
        let (uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt) = match resource_setting {
            ResourceSetting::Sparse => (2, 4, 5, 4, 5, 5),
            ResourceSetting::Abundant => (4, 6, 9, 9, 10, 10),
            _ => (4, 4, 7, 6, 7, 8), // Default
        };

        (uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt)
    }

    // function AssignStartingPlots:AssignLuxuryRoles
    /// Assigns luxury resources roles. Every luxury type has a role, the role should be one of the following:
    /// * Special case. For example, Marble. We need to implement a dedicated placement function to handle it.
    /// * Exclusively Assigned to a region. Each region gets an individual Luxury type assigned to it. These types are limited to 8 in original CIV5.
    /// * Exclusively Assigned to a city state. These luxury types are exclusive to city states. These types is limited to 3 in original CIV5.
    /// * Not exclusively assigned to any region or city state, and not special case too. we will place it randomly. That means it can be placed in any region or city state.
    /// * Disabled. We will not place it on the map.
    ///
    /// Assigns a Luxury resource according the rules below:
    /// * first, assign to regions
    /// * then, assign to city states
    /// * then, radomly assign
    /// * then, disable
    /// # Notice
    /// Luxury roles must be assigned before placing City States.
    /// This is because civs who are forced to share their luxury type with other
    /// civs may get extra city states placed in their region to compensate. View [`TileMap::assign_city_states_to_regions_or_uninhabited_landmasses`] for more information.
    pub fn assign_luxury_roles(&mut self, map_parameters: &MapParameters) {
        // Sort the regions by their type, with `RegionType::Undefined` being sorted last.
        // Notice: In original code, the region which has the same type should be shuffled. But here we don't do that. We will implement it in the future.
        self.region_list.sort_by_key(|region| {
            let region_type = region.region_type;
            if region_type == RegionType::Undefined {
                9 // Place undefined regions at the end
            } else {
                region_type as i32 // Otherwise, use the region type value for sorting
            }
        });

        let mut resource_assigned_to_regions = HashSet::new();
        for region_index in 0..self.region_list.len() {
            let resource = self.assign_luxury_to_region(map_parameters, region_index);
            // TODO: Should be edited in the future
            self.region_list[region_index].luxury_resource = resource.name().to_string();
            resource_assigned_to_regions.insert(resource.name().to_string());
            *self
                .luxury_assign_to_region_count
                .entry(resource.name().to_string())
                .or_insert(0) += 1;
        }

        let luxury_city_state_weights: Vec<(Resource, usize)> = vec![
            (Resource::Resource("Whales".to_string()), 15),
            (Resource::Resource("Pearls".to_string()), 15),
            (Resource::Resource("Gold Ore".to_string()), 10),
            (Resource::Resource("Silver".to_string()), 10),
            (Resource::Resource("Gems".to_string()), 10),
            (Resource::Resource("Ivory".to_string()), 10),
            (Resource::Resource("Furs".to_string()), 15),
            (Resource::Resource("Dyes".to_string()), 10),
            (Resource::Resource("Spices".to_string()), 15),
            (Resource::Resource("Silk".to_string()), 15),
            (Resource::Resource("Sugar".to_string()), 10),
            (Resource::Resource("Cotton".to_string()), 10),
            (Resource::Resource("Wine".to_string()), 10),
            (Resource::Resource("Incense".to_string()), 15),
            (Resource::Resource("Copper".to_string()), 10),
            (Resource::Resource("Salt".to_string()), 10),
            (Resource::Resource("Citrus".to_string()), 15),
            (Resource::Resource("Truffles".to_string()), 15),
            (Resource::Resource("Crab".to_string()), 15),
            (Resource::Resource("Cocoa".to_string()), 10),
        ];

        // Assign three of the remaining resources to be exclusive to City States.
        // Get the list of resources and their weight that are not assigned to regions.
        let (mut resource_list, mut resource_weight_list): (Vec<_>, Vec<usize>) =
            luxury_city_state_weights
                .iter()
                .filter(|(luxury_resource, _)| {
                    !resource_assigned_to_regions.contains(luxury_resource.name())
                })
                .map(|(luxury_resource, weight)| (luxury_resource.name(), weight))
                .unzip();

        let mut resource_assigned_to_city_state = Vec::new();
        for _ in 0..3 {
            // Choose a random resource from the list.
            let dist: WeightedIndex<usize> =
                WeightedIndex::new(resource_weight_list.clone()).unwrap();
            let index = dist.sample(&mut self.random_number_generator);
            let resource = resource_list[index];
            // Remove it from the list and assign it to the city state.
            resource_assigned_to_city_state.push(resource.to_string());
            resource_weight_list.remove(index);
            resource_list.remove(index);
        }

        // Assign Marble to special casing.
        let resource_assigned_to_special_case = vec!["Marble".to_string()];

        // Assign appropriate amount to be Disabled, then assign the rest to be Random.

        // The amount of disabled resources should be determined by the map size.
        // Please view `AssignStartingPlots:GetDisabledLuxuriesTargetNumber` for more information.
        // TODO: Implement this as one field of the map_parameters in the map_parameters.rs file in the future.
        let num_disabled_luxury_resource = 0;

        // Get the list of resources that are not assigned to regions or city states.
        let mut remaining_resource_list = luxury_city_state_weights
            .iter()
            .filter(|(luxury_resource, _)| {
                !resource_assigned_to_regions.contains(luxury_resource.name())
                    && !resource_assigned_to_city_state
                        .contains(&luxury_resource.name().to_string())
            })
            .map(|(luxury_resource, _)| luxury_resource.name().to_string())
            .collect::<Vec<_>>();

        remaining_resource_list.shuffle(&mut self.random_number_generator);

        let mut resource_not_being_used = Vec::new();
        let mut resource_assigned_to_random = Vec::new();

        for resource in remaining_resource_list {
            if resource_not_being_used.len() < num_disabled_luxury_resource {
                resource_not_being_used.push(resource);
            } else {
                resource_assigned_to_random.push(resource);
            }
        }

        self.luxury_resource_role = LuxuryResourceRole {
            luxury_assigned_to_regions: resource_assigned_to_regions,
            luxury_assigned_to_city_state: resource_assigned_to_city_state,
            luxury_assigned_to_special_case: resource_assigned_to_special_case,
            luxury_assigned_to_random: resource_assigned_to_random,
            luxury_not_being_used: resource_not_being_used,
        };
    }

    // function AssignStartingPlots:AssignLuxuryToRegion
    /// Assigns a luxury type to a region, ensuring no resource is assigned to more than 3 regions and no more than 8 resources are assigned to regions.
    ///
    /// # Why we need to ensure no resource is assigned to more than 3 regions and no more than 8 resources are assigned to regions?
    /// Because in original CIV5, the maximum number of civilizations is 22, 3 * 8  = 24, it's enough for all civilizations.
    pub fn assign_luxury_to_region(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> Resource {
        // The maximum number of luxury types that can be assigned to regions.
        // TODO: Implement this as one field of the map_parameters in the map_parameters.rs file in the future.
        // TODO: We should edit this value in the future for the number of civilizations > 22.
        const NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS: usize = 8;

        // The maximum number of regions that can be allocated to each luxury type.
        // TODO: Implement this as one field of the map_parameters in the map_parameters.rs file in the future.
        // TODO: We should edit this value in the future for the number of civilizations > 22.
        const MAX_REGIONS_PER_LUXURY_TYPE: u32 = 3;

        let region = &self.region_list[region_index];
        let region_type = region.region_type;

        /* let luxury_city_state_weights = vec![
            (Resource::Resource("Whales".to_string()), 15),
            (Resource::Resource("Pearls".to_string()), 15),
            (Resource::Resource("Gold Ore".to_string()), 10),
            (Resource::Resource("Silver".to_string()), 10),
            (Resource::Resource("Gems".to_string()), 10),
            (Resource::Resource("Ivory".to_string()), 10),
            (Resource::Resource("Furs".to_string()), 15),
            (Resource::Resource("Dyes".to_string()), 10),
            (Resource::Resource("Spices".to_string()), 15),
            (Resource::Resource("Silk".to_string()), 15),
            (Resource::Resource("Sugar".to_string()), 10),
            (Resource::Resource("Cotton".to_string()), 10),
            (Resource::Resource("Wine".to_string()), 10),
            (Resource::Resource("Incense".to_string()), 15),
            (Resource::Resource("Copper".to_string()), 10),
            (Resource::Resource("Salt".to_string()), 10),
            (Resource::Resource("Citrus".to_string()), 15),
            (Resource::Resource("Truffles".to_string()), 15),
            (Resource::Resource("Crab".to_string()), 15),
            (Resource::Resource("Cocoa".to_string()), 10),
        ]; */

        let luxury_fallback_weights = vec![
            (Resource::Resource("Whales".to_string()), 10),
            (Resource::Resource("Pearls".to_string()), 10),
            (Resource::Resource("Gold Ore".to_string()), 10),
            (Resource::Resource("Silver".to_string()), 5),
            (Resource::Resource("Gems".to_string()), 10),
            (Resource::Resource("Ivory".to_string()), 5),
            (Resource::Resource("Furs".to_string()), 10),
            (Resource::Resource("Dyes".to_string()), 5),
            (Resource::Resource("Spices".to_string()), 5),
            (Resource::Resource("Silk".to_string()), 5),
            (Resource::Resource("Sugar".to_string()), 5),
            (Resource::Resource("Cotton".to_string()), 5),
            (Resource::Resource("Wine".to_string()), 5),
            (Resource::Resource("Incense".to_string()), 5),
            (Resource::Resource("Copper".to_string()), 5),
            (Resource::Resource("Salt".to_string()), 5),
            (Resource::Resource("Citrus".to_string()), 5),
            (Resource::Resource("Truffles".to_string()), 5),
            (Resource::Resource("Crab".to_string()), 10),
            (Resource::Resource("Cocoa".to_string()), 5),
        ];

        let luxury_candidates = match region_type {
            RegionType::Undefined => luxury_fallback_weights.clone(),
            RegionType::Tundra => vec![
                (Resource::Resource("Furs".to_string()), 40),
                (Resource::Resource("Whales".to_string()), 35),
                (Resource::Resource("Crab".to_string()), 30),
                (Resource::Resource("Silver".to_string()), 25),
                (Resource::Resource("Copper".to_string()), 15),
                (Resource::Resource("Salt".to_string()), 15),
                (Resource::Resource("Gems".to_string()), 5),
                (Resource::Resource("Dyes".to_string()), 5),
            ],
            RegionType::Jungle => vec![
                (Resource::Resource("Cocoa".to_string()), 35),
                (Resource::Resource("Citrus".to_string()), 35),
                (Resource::Resource("Spices".to_string()), 30),
                (Resource::Resource("Gems".to_string()), 20),
                (Resource::Resource("Sugar".to_string()), 20),
                (Resource::Resource("Pearls".to_string()), 20),
                (Resource::Resource("Copper".to_string()), 5),
                (Resource::Resource("Truffles".to_string()), 5),
                (Resource::Resource("Crab".to_string()), 5),
                (Resource::Resource("Silk".to_string()), 5),
                (Resource::Resource("Dyes".to_string()), 5),
            ],
            RegionType::Forest => vec![
                (Resource::Resource("Dyes".to_string()), 30),
                (Resource::Resource("Silk".to_string()), 30),
                (Resource::Resource("Truffles".to_string()), 30),
                (Resource::Resource("Furs".to_string()), 10),
                (Resource::Resource("Spices".to_string()), 10),
                (Resource::Resource("Citrus".to_string()), 5),
                (Resource::Resource("Salt".to_string()), 5),
                (Resource::Resource("Copper".to_string()), 5),
                (Resource::Resource("Cocoa".to_string()), 5),
                (Resource::Resource("Crab".to_string()), 10),
                (Resource::Resource("Whales".to_string()), 10),
                (Resource::Resource("Pearls".to_string()), 10),
            ],
            RegionType::Desert => vec![
                (Resource::Resource("Incense".to_string()), 35),
                (Resource::Resource("Salt".to_string()), 15),
                (Resource::Resource("Gold Ore".to_string()), 25),
                (Resource::Resource("Copper".to_string()), 10),
                (Resource::Resource("Cotton".to_string()), 15),
                (Resource::Resource("Sugar".to_string()), 15),
                (Resource::Resource("Pearls".to_string()), 5),
                (Resource::Resource("Citrus".to_string()), 5),
            ],
            RegionType::Hill => vec![
                (Resource::Resource("Gold Ore".to_string()), 30),
                (Resource::Resource("Silver".to_string()), 30),
                (Resource::Resource("Copper".to_string()), 30),
                (Resource::Resource("Gems".to_string()), 15),
                (Resource::Resource("Pearls".to_string()), 15),
                (Resource::Resource("Salt".to_string()), 10),
                (Resource::Resource("Crab".to_string()), 10),
                (Resource::Resource("Whales".to_string()), 10),
            ],
            RegionType::Plain => vec![
                (Resource::Resource("Ivory".to_string()), 35),
                (Resource::Resource("Wine".to_string()), 35),
                (Resource::Resource("Salt".to_string()), 25),
                (Resource::Resource("Incense".to_string()), 10),
                (Resource::Resource("Spices".to_string()), 5),
                (Resource::Resource("Whales".to_string()), 5),
                (Resource::Resource("Pearls".to_string()), 5),
                (Resource::Resource("Crab".to_string()), 5),
                (Resource::Resource("Truffles".to_string()), 5),
                (Resource::Resource("Gold Ore".to_string()), 5),
            ],
            RegionType::Grassland => vec![
                (Resource::Resource("Cotton".to_string()), 30),
                (Resource::Resource("Silver".to_string()), 20),
                (Resource::Resource("Sugar".to_string()), 20),
                (Resource::Resource("Copper".to_string()), 20),
                (Resource::Resource("Crab".to_string()), 20),
                (Resource::Resource("Pearls".to_string()), 10),
                (Resource::Resource("Whales".to_string()), 10),
                (Resource::Resource("Cocoa".to_string()), 10),
                (Resource::Resource("Truffles".to_string()), 5),
                (Resource::Resource("Spices".to_string()), 5),
                (Resource::Resource("Gems".to_string()), 5),
            ],
            RegionType::Hybrid => vec![
                (Resource::Resource("Ivory".to_string()), 15),
                (Resource::Resource("Cotton".to_string()), 15),
                (Resource::Resource("Wine".to_string()), 15),
                (Resource::Resource("Silver".to_string()), 10),
                (Resource::Resource("Salt".to_string()), 15),
                (Resource::Resource("Copper".to_string()), 20),
                (Resource::Resource("Whales".to_string()), 20),
                (Resource::Resource("Pearls".to_string()), 20),
                (Resource::Resource("Crab".to_string()), 20),
                (Resource::Resource("Truffles".to_string()), 10),
                (Resource::Resource("Cocoa".to_string()), 10),
                (Resource::Resource("Spices".to_string()), 5),
                (Resource::Resource("Sugar".to_string()), 5),
                (Resource::Resource("Incense".to_string()), 5),
                (Resource::Resource("Silk".to_string()), 5),
                (Resource::Resource("Gems".to_string()), 5),
                (Resource::Resource("Gold Ore".to_string()), 5),
            ],
        };

        let split_cap = if map_parameters.civilization_num > 12 {
            MAX_REGIONS_PER_LUXURY_TYPE
        } else if map_parameters.civilization_num > 8 {
            2
        } else {
            1
        };

        let num_assigned_luxury_types = self.luxury_assign_to_region_count.len();

        // Check if the luxury resource is eligible to be assigned to the region.
        // The luxury resource is eligible if:
        // 1. The luxury assignment count is less than the maximum regions per luxury type.
        //    Usually the maximum regions per luxury type is determined by the number of civilizations in the game.
        //    When we use fallback options, the maximum regions per luxury type is 3.
        // 2. The number of assigned luxury types should <= the maximum allowed luxury types for regions (8).
        //    - If num_assigned_luxury_types < 8, then we can assign more luxury types to regions.
        //    - If num_assigned_luxury_types = 8, then we can only assign luxury types to regions that are already assigned to regions.
        let is_eligible_luxury_resource =
            |luxury_resource: &str,
             luxury_assignment_count: u32,
             max_regions_per_luxury_type: u32| {
                luxury_assignment_count < max_regions_per_luxury_type
                    && (num_assigned_luxury_types < NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS
                        || self
                            .luxury_resource_role
                            .luxury_assigned_to_regions
                            .contains(luxury_resource))
            };

        let mut resource_list = Vec::new();
        let mut resource_weight_list = Vec::new();
        for (luxury_resource, weight) in luxury_candidates.iter() {
            let luxury_resource = luxury_resource.name();
            let luxury_assign_to_region_count: u32 = *self
                .luxury_assign_to_region_count
                .get(luxury_resource)
                .unwrap_or(&0);

            if is_eligible_luxury_resource(
                luxury_resource,
                luxury_assign_to_region_count,
                split_cap,
            ) {
                // This type still eligible.
                // Water-based resources need to run a series of permission checks: coastal start in region, not a disallowed regions type, enough water, etc.
                if luxury_resource == "Whales"
                    || luxury_resource == "Pearls"
                    || luxury_resource == "Crab"
                {
                    // The code below is commented is unnecessary in the current implementation
                    // because `luxury_candidates` is already filtered to only include resources that are allowed in the region type.
                    /* if luxury_resource == "Whales" && region_type == RegionType::Jungle {
                        // Whales are not allowed in Jungle regions.
                        continue;
                    } else if luxury_resource == "Pearls" && region_type == RegionType::Tundra {
                        // Pearls are not allowed in Tundra regions.
                        continue;
                    } else if luxury_resource == "Crab" && region_type == RegionType::Desert {
                        // Crabs are not allowed in Desert regions.
                        continue;
                    } else */
                    if region.start_location_condition.along_ocean
                        && region.terrain_statistic.terrain_type_sum[&TerrainType::Water] > 12
                    {
                        // Water-based luxuries are allowed if both of the following are true:
                        // 1. This region's start is along an ocean,
                        // 2. This region has enough water to support water-based luxuries.
                        resource_list.push(luxury_resource);
                        let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                        resource_weight_list.push(adjusted_weight);
                    }
                } else {
                    // Land-based resources are automatically approved if they were in the region's option table.
                    resource_list.push(luxury_resource);
                    let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                    resource_weight_list.push(adjusted_weight);
                }
            }
        }

        // If options list is empty and region type isn't undefined and split_cap isn't 3, try to pick from fallback options.
        // We don't need to run again because when region type is undefined and split_cap is 3,
        // `luxury_candidates` is equal to fallback options, and we have already run the same function code above.
        if resource_list.is_empty() && region_type != RegionType::Undefined && split_cap != 3 {
            for (luxury_resource, weight) in luxury_fallback_weights.iter() {
                let luxury_resource = luxury_resource.name();
                let luxury_assign_to_region_count: u32 = *self
                    .luxury_assign_to_region_count
                    .get(luxury_resource)
                    .unwrap_or(&0);
                if is_eligible_luxury_resource(
                    luxury_resource,
                    luxury_assign_to_region_count,
                    MAX_REGIONS_PER_LUXURY_TYPE,
                ) {
                    // This type still eligible.
                    // Water-based resources need to run a series of permission checks: coastal start in region, not a disallowed regions type, enough water, etc.
                    if luxury_resource == "Whales"
                        || luxury_resource == "Pearls"
                        || luxury_resource == "Crab"
                    {
                        if luxury_resource == "Whales" && region_type == RegionType::Jungle {
                            // Whales are not allowed in Jungle regions.
                            continue;
                        } else if luxury_resource == "Pearls" && region_type == RegionType::Tundra {
                            // Pearls are not allowed in Tundra regions.
                            continue;
                        } else if luxury_resource == "Crab" && region_type == RegionType::Desert {
                            // Crabs are not allowed in Desert regions.
                            // NOTE: In the original code, this check is not present. I think it is a bug.
                            continue;
                        } else if region.start_location_condition.along_ocean
                            && region.terrain_statistic.terrain_type_sum[&TerrainType::Water] > 12
                        {
                            resource_list.push(luxury_resource);
                            let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                            resource_weight_list.push(adjusted_weight);
                        }
                    } else {
                        resource_list.push(luxury_resource);
                        let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                        resource_weight_list.push(adjusted_weight);
                    }
                }
            }
        }

        // If we get to here and still need to assign a luxury type, it means we have to force a water-based luxury in to this region, period.
        // This should be the rarest of the rare emergency assignment cases, unless modifications to the system have tightened things too far.
        if resource_list.is_empty() {
            for (luxury_resource, weight) in luxury_candidates.iter() {
                let luxury_resource = luxury_resource.name();
                let luxury_assign_to_region_count: u32 = *self
                    .luxury_assign_to_region_count
                    .get(luxury_resource)
                    .unwrap_or(&0);
                if is_eligible_luxury_resource(
                    luxury_resource,
                    luxury_assign_to_region_count,
                    MAX_REGIONS_PER_LUXURY_TYPE,
                ) {
                    resource_list.push(luxury_resource);
                    let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                    resource_weight_list.push(adjusted_weight);
                }
            }
        }

        if resource_list.is_empty() {
            panic!("No luxury resource available to assign to the region.");
        }

        // Choose a random luxury resource from the list.
        let dist: WeightedIndex<u32> = WeightedIndex::new(&resource_weight_list).unwrap();
        let resource = resource_list[dist.sample(&mut self.random_number_generator)];

        Resource::Resource(resource.to_string())
    }

    // function AssignStartingPlots:BalanceAndAssign
    /// This function does 2 things:
    /// 1. Balance the starting plots, such as add bonus/strategic resources, change neighbouring terrain, etc.
    ///    That will make each civilization have a fair chance to win the game.
    /// 2. Assign the starting plots to civilizations.
    /// # Notice
    /// We have not implemented to create the team for the civilization.
    pub fn balance_and_assign(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
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
        civilization_list.sort();

        civilization_list.shuffle(&mut self.random_number_generator);

        let mut start_civilization_list: Vec<_> = civilization_list
            .into_iter()
            .take(map_parameters.civilization_num as usize)
            .collect();
        /***** That will implement in `map_parameters` file later *****/

        for region_index in 0..self.region_list.len() {
            let start_location_condition =
                self.normalize_start_location(map_parameters, region_index);
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
            let hills_count = terrain_statistic.terrain_type_sum[&TerrainType::Hill];
            let peaks_count = terrain_statistic.terrain_type_sum[&TerrainType::Mountain];
            let grass_count = terrain_statistic.base_terrain_sum[&BaseTerrain::Grassland];
            let plains_count = terrain_statistic.base_terrain_sum[&BaseTerrain::Plain];
            let desert_count = terrain_statistic.base_terrain_sum[&BaseTerrain::Desert];
            let tundra_count = terrain_statistic.base_terrain_sum[&BaseTerrain::Tundra];
            let snow_count = terrain_statistic.base_terrain_sum[&BaseTerrain::Snow];
            let forest_count = terrain_statistic.feature_sum[&Feature::Forest];
            let jungle_count = terrain_statistic.feature_sum[&Feature::Jungle];
            let marsh_count = terrain_statistic.feature_sum[&Feature::Marsh];
            let floodplain_count = terrain_statistic.feature_sum[&Feature::Floodplain];
            let oasis_count = terrain_statistic.feature_sum[&Feature::Oasis];

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
    /// This function normalizes the starting tile for each civilization.
    /// It does 2 things:
    /// 1. Remove any feature Ice from the first ring of the starting tile.
    /// 2. Add some resource to the region.
    /// 3. Change the terrain of the starting tile's surroundings.
    /// 4. Get information about the starting tile and its surroundings for placing the civilization.
    pub fn normalize_start_location(
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

        // Remove any feature Ice from the first ring.
        // TODO: This should be reimplemented to remove feature ice from the first ring.
        self.generate_luxury_plot_lists_at_city_site(map_parameters, starting_tile, 1, true);

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
            for tile in neighbor_tiles.iter() {
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
            for tile in tiles_at_distance_two.iter() {
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
                self.place_resource_impact(map_parameters, conversion_tile, Layer::Strategic, 0);
            }
        }

        if num_food_bonus_needed > 0 {
            let max_bonuses_possible = inner_can_have_bonus + outer_can_have_bonus;
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
                                &tile,
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
                    while let Some(tile) = second_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                &tile,
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
                    while let Some(tile) = third_ring_iter.next() {
                        let (placed_bonus, placed_oasis) = self
                            .attempt_to_place_bonus_resource_at_plot(
                                map_parameters,
                                &tile,
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
                        let placed_bonus = self.attempt_to_place_stone_at_grass_plot(&tile);
                        if placed_bonus {
                            inner_placed = true;
                            num_stone_needed -= 1;
                            break;
                        }
                    }
                } else if second_ring_iter.peek().is_some() {
                    // Add bonus to second ring.
                    while let Some(tile) = second_ring_iter.next() {
                        let placed_bonus = self.attempt_to_place_stone_at_grass_plot(&tile);
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

    // function AssignStartingPlots:AttemptToPlaceHillsAtPlot
    /// This function attempts to place a Hill at the currently chosen tile.
    /// If successful, it returns `true`, otherwise it returns `false`.
    pub fn attempt_to_place_hill_at_tile(
        &mut self,
        map_parameters: &MapParameters,
        tile: &Tile,
    ) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) != TerrainType::Water
            && tile.feature(self) != Some(Feature::Forest)
            && !tile.has_river(self, map_parameters)
        {
            self.terrain_type_query[tile.index()] = TerrainType::Hill;
            self.feature_query[tile.index()] = None;
            self.natural_wonder_query[tile.index()] = None;
            return true;
        } else {
            return false;
        }
    }

    // function AssignStartingPlots:AttemptToPlaceSmallStrategicAtPlot
    /// This function attempts to place a Small `Horses` or `Iron` Resource at the currently chosen tile.
    /// If successful, it returns `true`, otherwise it returns `false`.
    pub fn attempt_to_place_small_strategic_at_plot(&mut self, tile: &Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) == TerrainType::Flatland
            && tile.feature(self).is_none()
        {
            if matches!(
                tile.base_terrain(self),
                BaseTerrain::Grassland | BaseTerrain::Plain
            ) {
                let mut resource = Resource::Resource("Horses".to_owned());
                let diceroll = self.random_number_generator.gen_range(0..4);
                if diceroll == 2 {
                    resource = Resource::Resource("Iron".to_owned());
                }
                self.resource_query[tile.index()] = Some((resource, 2));
            } else {
                self.resource_query[tile.index()] =
                    Some((Resource::Resource("Iron".to_owned()), 2));
            }
            return true;
        } else {
            return false;
        }
    }

    // function AssignStartingPlots:AttemptToPlaceBonusResourceAtPlot
    /// This function attempts to place a Bonus Resource at the currently chosen tile.
    /// Returns two booleans. First is true if something was placed. Second true if Oasis placed.
    pub fn attempt_to_place_bonus_resource_at_plot(
        &mut self,
        map_parameters: &MapParameters,
        tile: &Tile,
        allow_oasis: bool,
    ) -> (bool, bool) {
        let terrain_type = tile.terrain_type(self);
        let base_terrain = tile.base_terrain(self);
        let feature = tile.feature(self);

        if tile.resource(self).is_none()
            && base_terrain != BaseTerrain::Snow
            && feature != Some(Feature::Oasis)
        {
            match terrain_type {
                TerrainType::Water => {
                    if base_terrain == BaseTerrain::Coast && feature == None {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Fish".to_owned()), 1));
                        return (true, false);
                    }
                }
                TerrainType::Flatland => {
                    if feature == None {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                self.resource_query[tile.index()] =
                                    Some((Resource::Resource("Cow".to_owned()), 1));
                                return (true, false);
                            }
                            BaseTerrain::Desert => {
                                if tile.is_freshwater(self, map_parameters) {
                                    self.resource_query[tile.index()] =
                                        Some((Resource::Resource("Wheat".to_owned()), 1));
                                    return (true, false);
                                } else if allow_oasis {
                                    self.feature_query[tile.index()] = Some(Feature::Oasis);
                                    return (true, true);
                                }
                            }
                            BaseTerrain::Plain => {
                                self.resource_query[tile.index()] =
                                    Some((Resource::Resource("Wheat".to_owned()), 1));
                                return (true, false);
                            }
                            BaseTerrain::Tundra => {
                                self.resource_query[tile.index()] =
                                    Some((Resource::Resource("Deer".to_owned()), 1));
                                return (true, false);
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    } else if feature == Some(Feature::Forest) {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Deer".to_owned()), 1));
                        return (true, false);
                    } else if feature == Some(Feature::Jungle) {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Bananas".to_owned()), 1));
                        return (true, false);
                    }
                }
                TerrainType::Mountain => (),
                TerrainType::Hill => {
                    if feature == None {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Sheep".to_owned()), 1));
                        return (true, false);
                    } else if feature == Some(Feature::Forest) {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Deer".to_owned()), 1));
                        return (true, false);
                    } else if feature == Some(Feature::Jungle) {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Bananas".to_owned()), 1));
                        return (true, false);
                    }
                }
            }
        }
        // Nothing placed.
        (false, false)
    }

    // function AssignStartingPlots:AttemptToPlaceStoneAtGrassPlot
    /// This function attempts to place a stone at a grass plot.
    /// Returns `true` if Stone is placed. Otherwise returns `false`.
    pub fn attempt_to_place_stone_at_grass_plot(&mut self, tile: &Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) == TerrainType::Flatland
            && tile.base_terrain(self) == BaseTerrain::Grassland
            && tile.feature(self).is_none()
        {
            self.resource_query[tile.index()] = Some((Resource::Resource("Stone".to_owned()), 1));
            return true;
        } else {
            return false;
        }
    }

    // function AssignStartingPlots:AddStrategicBalanceResources
    /// This function adds the required Strategic Resources to start plots, for games that have selected to enable Strategic Resource Balance.
    pub fn add_strategic_balance_resources(
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

        let radius = 3;

        for ripple_radius in 1..=radius {
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
        let (uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt) =
            Self::get_major_strategic_resource_quantity_values(map_parameters.resource_setting);

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

    // function AssignStartingPlots:PlaceSpecificNumberOfResources
    /// # Parameters
    /// - `quantity` is the maximum number of every type resource that can be placed on the tile.
    /// - `amount` is the number of tiles intended to receive an assignment of this resource.
    /// - `ratio` should be > 0 and <= 1 and is what determines when secondary and tertiary lists
    /// come in to play.\
    /// For instance, if we are assigning Sugar resources to Marsh, then if we are to assign 8 Sugar
    /// resources, but there are only 4 Marsh plots in the list:
    ///     - `ratio = 1` would assign a Sugar to every single marsh plot, and then the function return an unplaced value of 4;
    ///     - `ratio = 0.5` would assign only 2 Sugars to the 4 marsh plots, and the function return a
    /// value of 6.
    ///     - Any ratio less than or equal to 0.25 would assign 1 Sugar and return 7, as the ratio results will be rounded up not down, to the nearest integer.
    /// - `layer` is the layer to place the resource on. If None, the resource can be placed on any tiles of `tile_list` that are not already assigned to a resource.
    /// - `min_radius` and `max_radius` is related to resource_impact when we place resources on tiles.
    /// If `layer` is None, then `min_radius` and `max_radius` are ignored.
    /// If `layer` is not `Layer::Strategic`, `Layer::Luxury`, `Layer::Bonus`, or `Layer::Fish`, then `min_radius` and `max_radius` are ignored as well.
    /// # Panic
    /// - `max_radius` must be greater than or equal to `min_radius`. Otherwise, the function will panic.
    pub fn place_specific_number_of_resources(
        &mut self,
        map_parameters: &MapParameters,
        resource: Resource,
        quantity: u32,
        amount: u32,
        ratio: f64,
        layer: Option<Layer>,
        min_radius: u32,
        max_radius: u32,
        tile_list: &[Tile],
    ) -> u32 {
        assert!(
            max_radius >= min_radius,
            "'max_radius' must be greater than or equal to 'min_radius'!"
        );

        if tile_list.is_empty() {
            return amount;
        }

        let impact_table = match layer {
            Some(Layer::Strategic)
            | Some(Layer::Luxury)
            | Some(Layer::Bonus)
            | Some(Layer::Fish) => &self.layer_data[layer.as_ref().unwrap()],
            _ => &Vec::new(),
        };

        // Store how many resources are left to place
        let mut num_left_to_place = amount;

        // Calculate how many tiles is the candidates to place the resource on based on the ratio.
        // That means only a certain number of tiles in the `tile_list` will be assigned
        // If `ratio` is 1, then all tiles will be the candidates for assignment.
        // If `ratio` is less than 1, then the number of tiles to be the candidates is calculated
        let num_candidate_tiles = (ratio * tile_list.len() as f64).ceil() as u32;

        // `amount` is the number of tiles intended to receive an assignment of this resource.
        // `num_resources` is the maximum number of tiles that can receive an assignment of this resource.
        // `num_resources` is the minimum of `amount` and `num_candidate_tiles`.
        let num_resources = min(amount, num_candidate_tiles);

        let mut tile_and_impact_radius = Vec::with_capacity(num_resources as usize);

        for _ in 1..=num_resources {
            for &tile in tile_list.into_iter() {
                if impact_table.is_empty() || impact_table.get(tile.index()) == Some(&0) {
                    if tile.resource(self).is_none() {
                        self.resource_query[tile.index()] = Some((resource.clone(), quantity));
                        num_left_to_place -= 1;
                    }
                    // Place resource if impact table is not empty
                    if !impact_table.is_empty() {
                        let radius = self
                            .random_number_generator
                            .gen_range(min_radius..=max_radius);
                        tile_and_impact_radius.push((tile, radius));
                    }
                    break;
                }
            }
        }

        tile_and_impact_radius.into_iter().for_each(|(tile, rad)| {
            self.place_resource_impact(map_parameters, tile, layer.unwrap(), rad)
        });

        num_left_to_place
    }

    // function AssignStartingPlots:ChooseLocations
    /// Get starting tile for each civilization according to region. Every region will have a starting tile.
    ///
    /// # Notice
    /// That function may be implemented as struct [`Region`]'s member function in the future.
    fn choose_locations(&mut self, map_parameters: &MapParameters) {
        // Sort the region list by average fertility
        self.region_list
            .sort_by(|a, b| a.average_fertility().total_cmp(&b.average_fertility()));

        // When map_parameters.region_divide_method is `RegionDivideMethod::WholeMapRectangle` or `RegionDivideMethod::CustomRectangle`, all region's landmass_id is always `None`.
        let ignore_landmass_id = self.region_list[0].landmass_id.is_none();

        (0..self.region_list.len())
            .into_iter()
            .for_each(|region_index| {
                if ignore_landmass_id {
                    self.find_start_without_regard_to_area_id(map_parameters, region_index);
                } else if map_parameters.civilization_starting_tile_must_be_coastal_land {
                    self.find_coastal_land_start(map_parameters, region_index);
                } else {
                    self.find_start(map_parameters, region_index);
                }
            })
    }

    // function AssignStartingPlots:FindStartWithoutRegardToAreaID
    fn find_start_without_regard_to_area_id(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> (bool, bool) {
        let region = &self.region_list[region_index];

        let success_flag = false; // Returns true when a start is placed, false when process fails.
        let forced_placement_flag = false; // Returns true if this region had no eligible starts and one was forced to occur.

        let mut fallback_tile_and_score = Vec::new();

        let mut area_id_and_fertility = HashMap::new();

        // Store the candidate starting tile in each area (different area_id means different area)
        // At first, the candidate starting tile is flatland or hill, and then it should meet one of the following conditions:
        // 1. It is a coastal land tile
        // 2. It is not a coastal land tile, and it does not have any coastal land tiles as neighbors
        let mut area_id_and_candidate_tiles: HashMap<i32, Vec<Tile>> = HashMap::new();

        for (i, tile) in region.rectangle.iter_tiles(map_parameters).enumerate() {
            if matches!(
                tile.terrain_type(self),
                TerrainType::Flatland | TerrainType::Hill
            ) {
                let tile_fertility = region.fertility_list[i];

                let area_id = tile.area_id(self);

                *area_id_and_fertility.entry(area_id).or_insert(0) += tile_fertility;

                if tile.can_be_civilization_starting_tile(self, map_parameters) {
                    area_id_and_candidate_tiles
                        .entry(area_id)
                        .or_default()
                        .push(tile);
                }
            }
        }

        let mut area_id_and_fertility: Vec<_> = area_id_and_fertility.into_iter().collect();

        area_id_and_fertility.sort_by_key(|(_, fertility)| *fertility);

        // Iterate through the area_id_and_fertility list in descending order of fertility
        for &(area_id, _) in area_id_and_fertility.iter().rev() {
            let tile_list = &area_id_and_candidate_tiles[&area_id];
            let (eletion1_tile, election2_tile, _, election2_tile_score) =
                self.iterate_through_candidate_tile_list(map_parameters, tile_list, region);

            if let Some(election1_tile) = eletion1_tile {
                self.region_list[region_index].starting_tile = election1_tile;
                self.place_impact_and_ripples(map_parameters, election1_tile);
                return (true, false);
            }

            if let Some(election_2_tile) = election2_tile {
                fallback_tile_and_score.push((election_2_tile, election2_tile_score));
            }
        }

        let max_score_tile = fallback_tile_and_score
            .iter()
            .max_by_key(|&(_, score)| score)
            .map(|&(tile, _)| tile);

        if let Some(max_score_tile) = max_score_tile {
            self.region_list[region_index].starting_tile = max_score_tile;
            self.place_impact_and_ripples(map_parameters, max_score_tile);
            return (true, false);
        } else {
            let x = region.rectangle.west_x;
            let y = region.rectangle.south_y;

            let tile = Tile::from_offset_coordinate(map_parameters, OffsetCoordinate::new(x, y))
                .expect("Offset coordinate is outside the map!");
            self.terrain_type_query[tile.index()] = TerrainType::Flatland;
            self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
            self.feature_query[tile.index()] = None;
            self.natural_wonder_query[tile.index()] = None;
            self.region_list[region_index].starting_tile = tile;
            self.place_impact_and_ripples(map_parameters, tile);
            return (false, true);
        }
    }

    // function AssignStartingPlots:FindCoastalStart
    fn find_coastal_land_start(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> (bool, bool) {
        let mut fallback_tile_and_score = Vec::new();

        let coastal_land_sum = (&self.region_list[region_index])
            .terrain_statistic
            .coastal_land_sum;

        let mut success_flag = false; // Returns true when a start is placed, false when process fails.
        let mut forced_placement_flag = false; // Returns true if this region had no eligible starts and one was forced to occur.

        if coastal_land_sum < 3 {
            // This region cannot support an Along Ocean start.
            // Try instead to find an inland start for it.
            (success_flag, forced_placement_flag) = self.find_start(map_parameters, region_index);

            if !success_flag {
                forced_placement_flag = true;

                let x = (&self.region_list[region_index]).rectangle.west_x;
                let y = (&self.region_list[region_index]).rectangle.south_y;

                let tile =
                    Tile::from_offset_coordinate(map_parameters, OffsetCoordinate::new(x, y))
                        .expect("Offset coordinate is outside the map!");
                self.terrain_type_query[tile.index()] = TerrainType::Flatland;
                self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
                self.feature_query[tile.index()] = None;
                self.natural_wonder_query[tile.index()] = None;
                self.region_list[region_index].starting_tile = tile;
                self.place_impact_and_ripples(map_parameters, tile);
            }

            return (success_flag, forced_placement_flag);
        }

        let rectangle = self.region_list[region_index].rectangle;

        // Positioner defaults. These are the controls for the "Center Bias" placement method for civ starts in regions.
        const CENTER_BIAS: f64 = 1. / 3.; // d% of radius from region center to examine first
        const MIDDLE_BIAS: f64 = 2. / 3.; // d% of radius from region center to check second

        let center_width = CENTER_BIAS * rectangle.width as f64;
        let non_center_width = ((rectangle.width as f64 - center_width) / 2.0).floor() as i32;
        let center_width = rectangle.width - (non_center_width * 2);

        let center_west_x = (rectangle.west_x + non_center_width) % map_parameters.map_size.width;

        let center_height = CENTER_BIAS * rectangle.height as f64;
        let non_center_height = ((rectangle.height as f64 - center_height) / 2.0).floor() as i32;
        let center_height = rectangle.height - (non_center_height * 2);

        let center_south_y =
            (rectangle.south_y + non_center_height) % map_parameters.map_size.height;

        let center_rectangle = Rectangle {
            west_x: center_west_x,
            south_y: center_south_y,
            width: center_width,
            height: center_height,
        };

        let middle_width = MIDDLE_BIAS * rectangle.width as f64;
        let outer_width = ((rectangle.width as f64 - middle_width) / 2.0).floor() as i32;
        let middle_width = rectangle.width - (outer_width * 2);

        let middle_west_x = (rectangle.west_x + outer_width) % map_parameters.map_size.width;

        let middle_height = MIDDLE_BIAS * rectangle.height as f64;
        let outer_height = ((rectangle.height as f64 - middle_height) / 2.0).floor() as i32;
        let middle_height = rectangle.height - (outer_height * 2);

        let middle_south_y = (rectangle.south_y + outer_height) % map_parameters.map_size.height;

        let middle_rectangle = Rectangle {
            west_x: middle_west_x,
            south_y: middle_south_y,
            width: middle_width,
            height: middle_height,
        };

        let mut center_coastal_plots = Vec::new();
        let mut center_plots_on_river = Vec::new();
        let mut center_fresh_plots = Vec::new();
        let mut center_dry_plots = Vec::new();

        let mut middle_coastal_plots = Vec::new();
        let mut middle_plots_on_river = Vec::new();
        let mut middle_fresh_plots = Vec::new();
        let mut middle_dry_plots = Vec::new();

        let mut outer_coastal_plots = Vec::new();

        for tile in rectangle.iter_tiles(map_parameters) {
            if tile.can_be_civilization_starting_tile(self, map_parameters) {
                let area_id = tile.area_id(self);
                let landmass_id = self.region_list[region_index].landmass_id;
                if landmass_id == Some(area_id) {
                    if center_rectangle.contains(map_parameters, tile) {
                        // Center Bias
                        center_coastal_plots.push(tile);
                        if tile.has_river(self, map_parameters) {
                            center_plots_on_river.push(tile);
                        } else if tile.is_freshwater(self, map_parameters) {
                            center_fresh_plots.push(tile);
                        } else {
                            center_dry_plots.push(tile);
                        }
                    } else if middle_rectangle.contains(map_parameters, tile) {
                        // Middle Bias
                        middle_coastal_plots.push(tile);
                        if tile.has_river(self, map_parameters) {
                            middle_plots_on_river.push(tile);
                        } else if tile.is_freshwater(self, map_parameters) {
                            middle_fresh_plots.push(tile);
                        } else {
                            middle_dry_plots.push(tile);
                        }
                    } else {
                        outer_coastal_plots.push(tile);
                    }
                }
            }
        }

        let region = &self.region_list[region_index];

        if center_coastal_plots.len() + middle_coastal_plots.len() > 0 {
            let candidate_lists = [
                center_plots_on_river,
                center_fresh_plots,
                center_dry_plots,
                middle_plots_on_river,
                middle_fresh_plots,
                middle_dry_plots,
            ];

            for tile_list in candidate_lists.iter() {
                let (eletion1_tile, election2_tile, _, election2_tile_score) =
                    self.iterate_through_candidate_tile_list(map_parameters, tile_list, region);

                if let Some(election1_tile) = eletion1_tile {
                    self.region_list[region_index].starting_tile = election1_tile;
                    self.place_impact_and_ripples(map_parameters, election1_tile);
                    return (true, false);
                }
                if let Some(election_2_tile) = election2_tile {
                    fallback_tile_and_score.push((election_2_tile, election2_tile_score));
                }
            }
        }

        if outer_coastal_plots.len() > 0 {
            let mut outer_eligible_list = Vec::new();
            let mut found_eligible = false;
            let mut found_fallback = false;
            let mut best_fallback_score = 0; //-50.0;
            let mut best_fallback_index = None;

            // Process list of candidate plots.
            for tile in outer_coastal_plots.into_iter() {
                let (score, meets_minimum_requirements) =
                    self.evaluate_candidate_tile(map_parameters, tile, region);

                if meets_minimum_requirements {
                    found_eligible = true;
                    outer_eligible_list.push(tile);
                } else {
                    found_fallback = true;
                    if score > best_fallback_score {
                        best_fallback_score = score;
                        best_fallback_index = Some(tile);
                    }
                }
            }

            if found_eligible {
                // Iterate through eligible plots and choose the one closest to the center of the region.
                let mut closest_tile = None;
                let mut closest_distance = i32::max(
                    map_parameters.map_size.width,
                    map_parameters.map_size.height,
                ) as f64;

                // Because west_x >= 0, bullseye_x will always be >= 0.
                let mut bullseye_x = rectangle.west_x as f64 + (rectangle.width as f64 / 2.0);
                // Because south_y >= 0, bullseye_y will always be >= 0.
                let mut bullseye_y = rectangle.south_y as f64 + (rectangle.height as f64 / 2.0);

                match (map_parameters.hex_layout.orientation, map_parameters.offset) {
                    (HexOrientation::Pointy, Offset::Odd) => {
                        if bullseye_y / 2.0 != (bullseye_y / 2.0).floor() {
                            // Y coord is odd, add .5 to X coord for hex-shift.
                            bullseye_x += 0.5;
                        }
                    }
                    (HexOrientation::Pointy, Offset::Even) => {
                        if bullseye_y / 2.0 == (bullseye_y / 2.0).floor() {
                            // Y coord is even, add .5 to X coord for hex-shift.
                            bullseye_x += 0.5;
                        }
                    }
                    (HexOrientation::Flat, Offset::Odd) => {
                        // X coord is odd, add .5 to Y coord for hex-shift.
                        if bullseye_x / 2.0 != (bullseye_x / 2.0).floor() {
                            // X coord is odd, add .5 to Y coord for hex-shift.
                            bullseye_y += 0.5;
                        }
                    }
                    (HexOrientation::Flat, Offset::Even) => {
                        // X coord is even, add .5 to Y coord for hex-shift.
                        if bullseye_x / 2.0 == (bullseye_x / 2.0).floor() {
                            // X coord is even, add .5 to Y coord for hex-shift.
                            bullseye_y += 0.5;
                        }
                    }
                }

                for tile in outer_eligible_list.into_iter() {
                    let offset_coordinate = tile.to_offset_coordinate(map_parameters);

                    let [x, y] = offset_coordinate.to_array();

                    let mut adjusted_x = x as f64;
                    let mut adjusted_y = y as f64;

                    match (map_parameters.hex_layout.orientation, map_parameters.offset) {
                        (HexOrientation::Pointy, Offset::Odd) => {
                            if y % 2 != 0 {
                                // Y coord is odd, add .5 to X coord for hex-shift.
                                adjusted_x += 0.5;
                            }
                        }
                        (HexOrientation::Pointy, Offset::Even) => {
                            if y % 2 == 0 {
                                // Y coord is even, add .5 to X coord for hex-shift.
                                adjusted_x += 0.5;
                            }
                        }
                        (HexOrientation::Flat, Offset::Odd) => {
                            if x % 2 != 0 {
                                // X coord is odd, add .5 to Y coord for hex-shift.
                                adjusted_y += 0.5;
                            }
                        }
                        (HexOrientation::Flat, Offset::Even) => {
                            if x % 2 == 0 {
                                // X coord is even, add .5 to Y coord for hex-shift.
                                adjusted_y += 0.5;
                            }
                        }
                    }

                    if x < rectangle.west_x {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_x += map_parameters.map_size.width as f64;
                    }
                    if y < rectangle.south_y {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_y += map_parameters.map_size.height as f64;
                    }

                    let distance = ((adjusted_x - bullseye_x).powf(2.0)
                        + (adjusted_y - bullseye_y).powf(2.0))
                    .sqrt();
                    if distance < closest_distance {
                        // Found new "closer" tile.
                        closest_tile = Some(tile);
                        closest_distance = distance;
                    }
                }

                if let Some(closest_tile) = closest_tile {
                    // Re-get plot score for inclusion in start plot data.
                    let (_score, _meets_minimum_requirements) =
                        self.evaluate_candidate_tile(map_parameters, closest_tile, region);

                    // Assign this tile as the start for this region.
                    self.region_list[region_index].starting_tile = closest_tile;
                    self.place_impact_and_ripples(map_parameters, closest_tile);
                    return (true, false);
                }
            }

            // Add the fallback tile (best scored tile) from the Outer region to the fallback list.
            if found_fallback {
                if let Some(best_fallback_index) = best_fallback_index {
                    fallback_tile_and_score.push((best_fallback_index, best_fallback_score));
                }
            }
        }

        let max_score_tile = fallback_tile_and_score
            .iter()
            .max_by_key(|&(_, score)| score)
            .map(|&(tile, _)| tile);

        if let Some(max_score_tile) = max_score_tile {
            self.region_list[region_index].starting_tile = max_score_tile;
            self.place_impact_and_ripples(map_parameters, max_score_tile);
            return (true, false);
        } else {
            (success_flag, forced_placement_flag) = self.find_start(map_parameters, region_index);

            if !success_flag {
                forced_placement_flag = true;

                let x = rectangle.west_x;
                let y = rectangle.south_y;

                let tile =
                    Tile::from_offset_coordinate(map_parameters, OffsetCoordinate::new(x, y))
                        .expect("Offset coordinate is outside the map!");
                self.terrain_type_query[tile.index()] = TerrainType::Flatland;
                self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
                self.feature_query[tile.index()] = None;
                self.natural_wonder_query[tile.index()] = None;
                self.region_list[region_index].starting_tile = tile;
                self.place_impact_and_ripples(map_parameters, tile);
            }

            return (success_flag, forced_placement_flag);
        }
    }

    // function AssignStartingPlots:FindStart
    fn find_start(&mut self, map_parameters: &MapParameters, region_index: usize) -> (bool, bool) {
        let mut fallback_tile_and_score = Vec::new();

        let region = &self.region_list[region_index];

        let rectangle = region.rectangle;

        // Positioner defaults. These are the controls for the "Center Bias" placement method for civ starts in regions.
        const CENTER_BIAS: f64 = 1. / 3.; // d% of radius from region center to examine first
        const MIDDLE_BIAS: f64 = 2. / 3.; // d% of radius from region center to check second

        let center_width = CENTER_BIAS * rectangle.width as f64;
        let non_center_width = ((rectangle.width as f64 - center_width) / 2.0).floor() as i32;
        let center_width = rectangle.width - (non_center_width * 2);

        let center_west_x = (rectangle.west_x + non_center_width) % map_parameters.map_size.width;

        let center_height = CENTER_BIAS * rectangle.height as f64;
        let non_center_height = ((rectangle.height as f64 - center_height) / 2.0).floor() as i32;
        let center_height = rectangle.height - (non_center_height * 2);

        let center_south_y =
            (rectangle.south_y + non_center_height) % map_parameters.map_size.height;

        let center_rectangle = Rectangle {
            west_x: center_west_x,
            south_y: center_south_y,
            width: center_width,
            height: center_height,
        };

        let middle_width = MIDDLE_BIAS * rectangle.width as f64;
        let outer_width = ((rectangle.width as f64 - middle_width) / 2.0).floor() as i32;
        let middle_width = rectangle.width - (outer_width * 2);

        let middle_west_x = (rectangle.west_x + outer_width) % map_parameters.map_size.width;

        let middle_height = MIDDLE_BIAS * rectangle.height as f64;
        let outer_height = ((rectangle.height as f64 - middle_height) / 2.0).floor() as i32;
        let middle_height = rectangle.height - (outer_height * 2);

        let middle_south_y = (rectangle.south_y + outer_height) % map_parameters.map_size.height;

        let middle_rectangle = Rectangle {
            west_x: middle_west_x,
            south_y: middle_south_y,
            width: middle_width,
            height: middle_height,
        };

        let mut center_candidates = Vec::new();
        let mut center_river = Vec::new();
        let mut center_coastal_land_and_freshwater = Vec::new();
        let mut center_inland_dry_land = Vec::new();

        let mut middle_candidates = Vec::new();
        let mut middle_river = Vec::new();
        let mut middle_coastal_land_and_freshwater = Vec::new();
        let mut middle_inland_dry_land = Vec::new();

        let mut outer_plots = Vec::new();

        for tile in region.rectangle.iter_tiles(map_parameters) {
            if tile.can_be_civilization_starting_tile(self, map_parameters) {
                let area_id = tile.area_id(self);
                if region.landmass_id == Some(area_id) {
                    if center_rectangle.contains(map_parameters, tile) {
                        // Center Bias
                        center_candidates.push(tile);
                        if tile.has_river(self, map_parameters) {
                            center_river.push(tile);
                        } else if tile.is_freshwater(self, map_parameters)
                            || tile.is_coastal_land(self, map_parameters)
                        {
                            center_coastal_land_and_freshwater.push(tile);
                        } else {
                            center_inland_dry_land.push(tile);
                        }
                    } else if middle_rectangle.contains(map_parameters, tile) {
                        // Middle Bias
                        middle_candidates.push(tile);
                        if tile.has_river(self, map_parameters) {
                            middle_river.push(tile);
                        } else if tile.is_freshwater(self, map_parameters)
                            || tile.is_coastal_land(self, map_parameters)
                        {
                            middle_coastal_land_and_freshwater.push(tile);
                        } else {
                            middle_inland_dry_land.push(tile);
                        }
                    } else {
                        outer_plots.push(tile);
                    }
                }
            }
        }

        if center_candidates.len() + middle_candidates.len() > 0 {
            let candidate_lists = [
                center_river,
                center_coastal_land_and_freshwater,
                center_inland_dry_land,
                middle_river,
                middle_coastal_land_and_freshwater,
                middle_inland_dry_land,
            ];

            for tile_list in candidate_lists.iter() {
                let (eletion1_tile, election2_tile, _, election2_tile_score) =
                    self.iterate_through_candidate_tile_list(map_parameters, tile_list, region);

                if let Some(election1_tile) = eletion1_tile {
                    self.region_list[region_index].starting_tile = election1_tile;
                    self.place_impact_and_ripples(map_parameters, election1_tile);
                    return (true, false);
                }
                if let Some(election_2_tile) = election2_tile {
                    fallback_tile_and_score.push((election_2_tile, election2_tile_score));
                }
            }
        }

        if outer_plots.len() > 0 {
            let mut outer_eligible_list = Vec::new();
            let mut found_eligible = false;
            let mut found_fallback = false;
            let mut best_fallback_score = 0; //-50.0;
            let mut best_fallback_index = None;

            // Process list of candidate plots.
            for tile in outer_plots.into_iter() {
                let (score, meets_minimum_requirements) =
                    self.evaluate_candidate_tile(map_parameters, tile, region);

                if meets_minimum_requirements {
                    found_eligible = true;
                    outer_eligible_list.push(tile);
                } else {
                    found_fallback = true;
                    if score > best_fallback_score {
                        best_fallback_score = score;
                        best_fallback_index = Some(tile);
                    }
                }
            }

            if found_eligible {
                // Iterate through eligible plots and choose the one closest to the center of the region.
                let mut closest_plot = None;
                let mut closest_distance = i32::max(
                    map_parameters.map_size.width,
                    map_parameters.map_size.height,
                ) as f64;

                // Because west_x >= 0, bullseye_x will always be >= 0.
                let mut bullseye_x = rectangle.west_x as f64 + (rectangle.width as f64 / 2.0);
                // Because south_y >= 0, bullseye_y will always be >= 0.
                let mut bullseye_y = rectangle.south_y as f64 + (rectangle.height as f64 / 2.0);

                match (map_parameters.hex_layout.orientation, map_parameters.offset) {
                    (HexOrientation::Pointy, Offset::Odd) => {
                        if bullseye_y / 2.0 != (bullseye_y / 2.0).floor() {
                            // Y coord is odd, add .5 to X coord for hex-shift.
                            bullseye_x += 0.5;
                        }
                    }
                    (HexOrientation::Pointy, Offset::Even) => {
                        if bullseye_y / 2.0 == (bullseye_y / 2.0).floor() {
                            // Y coord is even, add .5 to X coord for hex-shift.
                            bullseye_x += 0.5;
                        }
                    }
                    (HexOrientation::Flat, Offset::Odd) => {
                        // X coord is odd, add .5 to Y coord for hex-shift.
                        if bullseye_x / 2.0 != (bullseye_x / 2.0).floor() {
                            // X coord is odd, add .5 to Y coord for hex-shift.
                            bullseye_y += 0.5;
                        }
                    }
                    (HexOrientation::Flat, Offset::Even) => {
                        // X coord is even, add .5 to Y coord for hex-shift.
                        if bullseye_x / 2.0 == (bullseye_x / 2.0).floor() {
                            // X coord is even, add .5 to Y coord for hex-shift.
                            bullseye_y += 0.5;
                        }
                    }
                }

                for tile in outer_eligible_list.into_iter() {
                    let offset_coordinate = tile.to_offset_coordinate(map_parameters);

                    let [x, y] = offset_coordinate.to_array();

                    let mut adjusted_x = x as f64;
                    let mut adjusted_y = y as f64;

                    match (map_parameters.hex_layout.orientation, map_parameters.offset) {
                        (HexOrientation::Pointy, Offset::Odd) => {
                            if y % 2 != 0 {
                                // Y coord is odd, add .5 to X coord for hex-shift.
                                adjusted_x += 0.5;
                            }
                        }
                        (HexOrientation::Pointy, Offset::Even) => {
                            if y % 2 == 0 {
                                // Y coord is even, add .5 to X coord for hex-shift.
                                adjusted_x += 0.5;
                            }
                        }
                        (HexOrientation::Flat, Offset::Odd) => {
                            if x % 2 != 0 {
                                // X coord is odd, add .5 to Y coord for hex-shift.
                                adjusted_y += 0.5;
                            }
                        }
                        (HexOrientation::Flat, Offset::Even) => {
                            if x % 2 == 0 {
                                // X coord is even, add .5 to Y coord for hex-shift.
                                adjusted_y += 0.5;
                            }
                        }
                    }

                    if x < region.rectangle.west_x {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_x += map_parameters.map_size.width as f64;
                    }
                    if y < region.rectangle.south_y {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_y += map_parameters.map_size.height as f64;
                    }

                    let distance = ((adjusted_x - bullseye_x).powf(2.0)
                        + (adjusted_y - bullseye_y).powf(2.0))
                    .sqrt();
                    if distance < closest_distance {
                        // Found new "closer" plot.
                        closest_plot = Some(tile);
                        closest_distance = distance;
                    }
                }

                if let Some(closest_plot) = closest_plot {
                    // Re-get plot score for inclusion in start plot data.
                    let (_score, _meets_minimum_requirements) =
                        self.evaluate_candidate_tile(map_parameters, closest_plot, region);

                    // Assign this plot as the start for this region.
                    self.region_list[region_index].starting_tile = closest_plot;
                    self.place_impact_and_ripples(map_parameters, closest_plot);
                    return (true, false);
                }
            }

            // Add the fallback plot (best scored plot) from the Outer region to the fallback list.
            if found_fallback {
                if let Some(best_fallback_index) = best_fallback_index {
                    fallback_tile_and_score.push((best_fallback_index, best_fallback_score));
                }
            }
        }

        let max_score_tile = fallback_tile_and_score
            .iter()
            .max_by_key(|&(_, score)| score)
            .map(|&(tile, _)| tile);

        if let Some(max_score_tile) = max_score_tile {
            self.region_list[region_index].starting_tile = max_score_tile;
            self.place_impact_and_ripples(map_parameters, max_score_tile);
            return (true, false);
        } else {
            let x = region.rectangle.west_x;
            let y = region.rectangle.south_y;

            let tile = Tile::from_offset_coordinate(map_parameters, OffsetCoordinate::new(x, y))
                .expect("Offset coordinate is outside the map!");
            self.terrain_type_query[tile.index()] = TerrainType::Flatland;
            self.base_terrain_query[tile.index()] = BaseTerrain::Grassland;
            self.feature_query[tile.index()] = None;
            self.natural_wonder_query[tile.index()] = None;
            self.region_list[region_index].starting_tile = tile;
            self.place_impact_and_ripples(map_parameters, tile);
            return (false, true);
        }
    }

    // function AssignStartingPlots:IterateThroughCandidatePlotList
    /// Iterates through a list of candidate plots and returns the best tile and fallback tile.
    ///
    /// This function assumes all candidate tiles can have a city built on them.
    /// Any tiles not allowed to have a city should be weeded out when building the candidate list.
    fn iterate_through_candidate_tile_list(
        &self,
        map_parameters: &MapParameters,
        candidate_tile_list: &[Tile],
        region: &Region,
    ) -> (Option<Tile>, Option<Tile>, i32, i32) {
        let mut best_tile_score = -5000;
        let mut best_tile = None;
        let mut best_fallback_score = -5000;
        let mut best_fallback_tile = None;

        for &tile in candidate_tile_list {
            let (score, meets_minimum_requirements) =
                self.evaluate_candidate_tile(map_parameters, tile, region);

            if meets_minimum_requirements {
                if score > best_tile_score {
                    best_tile_score = score;
                    best_tile = Some(tile);
                }
            } else {
                if score > best_fallback_score {
                    best_fallback_score = score;
                    best_fallback_tile = Some(tile);
                }
            }
        }

        (
            best_tile,
            best_fallback_tile,
            best_tile_score,
            best_fallback_score,
        )
    }

    // function AssignStartingPlots:EvaluateCandidatePlot
    /// Evaluates a candidate tile for starting city placement.
    ///
    /// If the tile meets the minimum requirements, it will return a score and true.
    /// If the tile does not meet the minimum requirements, it will return a score and false.
    fn evaluate_candidate_tile(
        &self,
        map_parameters: &MapParameters,
        tile: Tile,
        region: &Region,
    ) -> (i32, bool) {
        let mut meets_minimum_requirements = true;
        let min_food_inner = 1;
        let min_production_inner = 0;
        let min_good_inner = 3;
        let min_food_middle = 4;
        let min_production_middle = 0;
        let min_good_middle = 6;
        let min_food_outer = 4;
        let min_production_outer = 2;
        let min_good_outer = 8;
        let max_junk = 9;

        let mut food_total = 0;
        let mut production_total = 0;
        let mut good_total = 0;
        let mut junk_total = 0;
        let mut river_total = 0;
        let mut coastal_land_score = 0;

        if tile.is_coastal_land(self, map_parameters) {
            coastal_land_score = 40;
        }

        let neighbor_tiles = tile.neighbor_tiles(map_parameters);

        junk_total += 6 - neighbor_tiles.len() as i32;

        neighbor_tiles.into_iter().for_each(|neighbor_tile| {
            let tile_type = self.measure_single_tile(neighbor_tile, region);
            tile_type.into_iter().for_each(|tile_type| match tile_type {
                TileType::Food => food_total += 1,
                TileType::Production => production_total += 1,
                TileType::Good => good_total += 1,
                TileType::Junk => junk_total += 1,
            });
            if neighbor_tile.has_river(self, map_parameters) {
                river_total += 1;
            }
        });

        if food_total < min_food_inner
            || production_total < min_production_inner
            || good_total < min_good_inner
        {
            meets_minimum_requirements = false;
        };

        // `food_total`, `production_total` should <= 6 because the tile has max 6 neighbors.
        // So the length of weighted_food_inner, weighted_production_inner, should be 7.
        let weighted_food_inner = [0, 8, 14, 19, 22, 24, 25];
        let food_result_inner = weighted_food_inner[food_total as usize];
        let weighted_production_inner = [0, 10, 16, 20, 20, 12, 0];
        let production_result_inner = weighted_production_inner[production_total as usize];
        let good_result_inner = good_total * 2;
        let inner_ring_score =
            food_result_inner + production_result_inner + good_result_inner + river_total
                - (junk_total * 3);

        let tiles_at_distance_two = tile.tiles_at_distance(2, map_parameters);

        junk_total += 6 * 2 - tiles_at_distance_two.len() as i32;

        tiles_at_distance_two
            .into_iter()
            .for_each(|tile_at_distance_two| {
                let tile_type = self.measure_single_tile(tile_at_distance_two, region);
                tile_type.into_iter().for_each(|tile_type| match tile_type {
                    TileType::Food => food_total += 1,
                    TileType::Production => production_total += 1,
                    TileType::Good => good_total += 1,
                    TileType::Junk => junk_total += 1,
                });
                if tile_at_distance_two.has_river(self, map_parameters) {
                    river_total += 1;
                }
            });

        if food_total < min_food_middle
            || production_total < min_production_middle
            || good_total < min_good_middle
        {
            meets_minimum_requirements = false;
        }

        let weighted_food_middle = [0, 2, 5, 10, 20, 25, 28, 30, 32, 34, 35];
        // When food_total >= 10, the value is 35.
        let food_result_middle = if food_total >= 10 {
            35
        } else {
            weighted_food_middle[food_total as usize]
        };

        let weighted_production_middle = [0, 10, 20, 25, 30, 35];
        let effective_production_total = if food_total * 2 < production_total {
            (food_total + 1) / 2
        } else {
            production_total
        };

        // When effective_production_total >= 5, the value is 35.
        let production_result_middle = if effective_production_total >= 5 {
            35
        } else {
            weighted_production_middle[effective_production_total as usize]
        };

        let good_result_middle = good_total * 2;
        let middle_ring_score =
            food_result_middle + production_result_middle + good_result_middle + river_total
                - (junk_total * 3);

        let tiles_at_distance_three = tile.tiles_at_distance(3, map_parameters);

        junk_total += 6 * 3 - tiles_at_distance_three.len() as i32;

        tiles_at_distance_three
            .into_iter()
            .for_each(|tile_at_distance_three| {
                let tile_type = self.measure_single_tile(tile_at_distance_three, region);
                tile_type.into_iter().for_each(|tile_type| match tile_type {
                    TileType::Food => food_total += 1,
                    TileType::Production => production_total += 1,
                    TileType::Good => good_total += 1,
                    TileType::Junk => junk_total += 1,
                });
                if tile_at_distance_three.has_river(self, map_parameters) {
                    river_total += 1;
                }
            });

        if food_total < min_food_outer
            || production_total < min_production_outer
            || good_total < min_good_outer
            || junk_total > max_junk
        {
            meets_minimum_requirements = false;
        }

        let outer_ring_score =
            food_total + production_total + good_total + river_total - (junk_total * 2);
        let mut final_score =
            inner_ring_score + middle_ring_score + outer_ring_score + coastal_land_score;

        // Check Impact and Ripple data to see if candidate is near an already-placed start point.
        if self.distance_data[tile.index()] != 0 {
            // This candidate is near an already placed start. This invalidates its
            // eligibility for first-pass placement; but it may still qualify as a
            // fallback site, so we will reduce its Score according to the bias factor.
            meets_minimum_requirements = false;
            final_score = (final_score as f64 * (100 - self.distance_data[tile.index()]) as f64
                / 100.0) as i32;
        }
        (final_score, meets_minimum_requirements)
    }

    // function AssignStartingPlots:PlaceImpactAndRipples
    /// This function places the impact and ripple values for a starting tile of civilization.
    ///
    /// When you add a starting tile of civilization, you should run this function to place the impact and ripple values for the tile.
    fn place_impact_and_ripples(&mut self, map_parameters: &MapParameters, tile: Tile) {
        let impact_value = 99;
        let ripple_values = [97, 95, 92, 89, 69, 57, 24, 15];

        // Start points need to impact the resource layers.
        self.place_resource_impact(map_parameters, tile, Layer::Strategic, 0); // Strategic layer, at impact site only.
        self.place_resource_impact(map_parameters, tile, Layer::Luxury, 3); // Luxury layer
        self.place_resource_impact(map_parameters, tile, Layer::Bonus, 3); // Bonus layer
        self.place_resource_impact(map_parameters, tile, Layer::Fish, 3); // Fish layer
        self.place_resource_impact(map_parameters, tile, Layer::NaturalWonder, 4); // Natural Wonders layer

        self.distance_data[tile.index()] = impact_value;

        self.player_collision_data[tile.index()] = true;

        self.layer_data.get_mut(&Layer::CityState).unwrap()[tile.index()] = 1;

        for (index, ripple_value) in ripple_values.into_iter().enumerate() {
            let distance = index as u32 + 1;

            tile.tiles_at_distance(distance, map_parameters)
                .into_iter()
                .for_each(|tile_at_distance| {
                    if self.distance_data[tile_at_distance.index()] != 0 {
                        // First choose the greater of the two, existing value or current ripple.
                        let stronger_value =
                            max(self.distance_data[tile_at_distance.index()], ripple_value);
                        // Now increase it by 1.2x to reflect that multiple civs are in range of this plot.
                        let overlap_value = min(97, (stronger_value as f64 * 1.2) as u8);
                        self.distance_data[tile_at_distance.index()] = overlap_value;
                    } else {
                        self.distance_data[tile_at_distance.index()] = ripple_value;
                    }

                    if distance <= 6 {
                        self.layer_data.get_mut(&Layer::CityState).unwrap()
                            [tile_at_distance.index()] = 1;
                    }
                })
        }
    }

    // AssignStartingPlots:PlaceResourceImpact
    pub fn place_resource_impact(
        &mut self,
        map_parameters: &MapParameters,
        tile: Tile,
        layer: Layer,
        radius: u32,
    ) {
        let impact_value = if layer == Layer::Fish || layer == Layer::Marble {
            1
        } else {
            99
        };

        self.layer_data.get_mut(&layer).unwrap()[tile.index()] = impact_value;

        if radius == 0 {
            return;
        }

        if radius > 0 && radius < (self.map_size.height as u32 / 2) {
            for distance in 1..=radius {
                // Iterate over all tiles at this distance.
                tile.tiles_at_distance(distance, map_parameters)
                    .into_iter()
                    .for_each(|tile_at_distance| {
                        let ripple_value = radius - distance + 1;
                        match layer {
                            Layer::Strategic => {
                                if self.layer_data[&layer][tile_at_distance.index()] != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(
                                        self.layer_data[&layer][tile_at_distance.index()],
                                        ripple_value,
                                    );
                                    // Now increase it by 2 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(50, stronger_value + 2);
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = overlap_value;
                                } else {
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = ripple_value;
                                }
                            }
                            Layer::Luxury => {
                                if self.layer_data[&layer][tile_at_distance.index()] != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(
                                        self.layer_data[&layer][tile_at_distance.index()],
                                        ripple_value,
                                    );
                                    // Now increase it by 2 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(50, stronger_value + 2);
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = overlap_value;
                                } else {
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = ripple_value;
                                }
                            }
                            Layer::Bonus => {
                                if self.layer_data[&layer][tile_at_distance.index()] != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(
                                        self.layer_data[&layer][tile_at_distance.index()],
                                        ripple_value,
                                    );
                                    // Now increase it by 2 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(50, stronger_value + 2);
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = overlap_value;
                                } else {
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = ripple_value;
                                }
                            }
                            Layer::Fish => {
                                if self.layer_data[&layer][tile_at_distance.index()] != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(
                                        self.layer_data[&layer][tile_at_distance.index()],
                                        ripple_value,
                                    );
                                    // Now increase it by 1 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(10, stronger_value + 1);
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = overlap_value;
                                } else {
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = ripple_value;
                                }
                            }
                            Layer::CityState => {
                                self.layer_data.get_mut(&layer).unwrap()
                                    [tile_at_distance.index()] = 1;
                            }
                            Layer::NaturalWonder => {
                                if self.layer_data[&layer][tile_at_distance.index()] != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(
                                        self.layer_data[&layer][tile_at_distance.index()],
                                        ripple_value,
                                    );
                                    // Now increase it by 2 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(50, stronger_value + 2);
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = overlap_value;
                                } else {
                                    self.layer_data.get_mut(&layer).unwrap()
                                        [tile_at_distance.index()] = ripple_value;
                                }
                            }
                            Layer::Marble => {
                                self.layer_data.get_mut(&layer).unwrap()
                                    [tile_at_distance.index()] = 1;
                            }
                        }
                    })
            }
        }
    }

    // function AssignStartingPlots:MeasureSinglePlot
    fn measure_single_tile(&self, tile: Tile, region: &Region) -> Vec<TileType> {
        let region_type = region.region_type;
        /*  -- Note that "Food" is not strictly about tile yield.
        -- Different regions get their food in different ways.
        -- Tundra, Jungle, Forest, Desert, Plains regions will
        -- get Bonus resource support to cover food shortages.
        --
        -- Data table entries hold results; all begin as false:
        -- [1] "Food"
        -- [2] "Prod"
        -- [3] "Good"
        -- [4] "Junk" */
        let mut data = Vec::new();

        match tile.terrain_type(self) {
            TerrainType::Water => {
                if tile.feature(self) == Some(Feature::Ice) {
                    data.push(TileType::Junk);
                } else if tile.base_terrain(self) == BaseTerrain::Lake {
                    data.push(TileType::Food);
                    data.push(TileType::Good);
                } else if region.landmass_id.is_none()
                    && tile.base_terrain(self) == BaseTerrain::Coast
                {
                    data.push(TileType::Good);
                }
                return data;
            }
            TerrainType::Mountain => {
                data.push(TileType::Junk);
                return data;
            }
            TerrainType::Flatland | TerrainType::Hill => (),
        }

        // Tackle with the tile's terrain type is hill or flatland and has feature.
        if let Some(feature) = tile.feature(self) {
            match feature {
                Feature::Forest => {
                    data.push(TileType::Production);
                    data.push(TileType::Good);
                    if region_type == RegionType::Forest || region_type == RegionType::Tundra {
                        data.push(TileType::Food);
                    }
                    return data;
                }
                Feature::Jungle => {
                    if region_type != RegionType::Grassland {
                        data.push(TileType::Food);
                        data.push(TileType::Good);
                    } else if tile.terrain_type(self) == TerrainType::Hill {
                        data.push(TileType::Production);
                    }
                    return data;
                }
                Feature::Marsh => {
                    return data;
                }
                Feature::Oasis | Feature::Floodplain => {
                    data.push(TileType::Food);
                    data.push(TileType::Good);
                    return data;
                }
                _ => (),
            }
        }

        // Tackle with the tile's terrain type is hill and has no feature.
        if tile.terrain_type(self) == TerrainType::Hill {
            data.push(TileType::Production);
            data.push(TileType::Good);
            return data;
        }

        // Tackle with tile's terrain type is flatland and has no feature.
        match tile.base_terrain(self) {
            BaseTerrain::Grassland => {
                data.push(TileType::Good);
                if region_type == RegionType::Jungle
                    || region_type == RegionType::Forest
                    || region_type == RegionType::Hill
                    || region_type == RegionType::Grassland
                    || region_type == RegionType::Hybrid
                {
                    data.push(TileType::Food);
                }
                return data;
            }
            BaseTerrain::Desert => {
                if region_type != RegionType::Desert {
                    data.push(TileType::Junk);
                }
                return data;
            }
            BaseTerrain::Plain => {
                data.push(TileType::Good);
                if region_type == RegionType::Tundra
                    || region_type == RegionType::Desert
                    || region_type == RegionType::Hill
                    || region_type == RegionType::Plain
                    || region_type == RegionType::Hybrid
                {
                    data.push(TileType::Food);
                }
                return data;
            }
            BaseTerrain::Tundra => {
                if region_type == RegionType::Tundra {
                    data.push(TileType::Food);
                    data.push(TileType::Good);
                }
                return data;
            }
            BaseTerrain::Snow => {
                data.push(TileType::Junk);
                return data;
            }
            _ => (),
        }

        data
    }
}

pub struct LuxuryResourceRole {
    pub luxury_assigned_to_regions: HashSet<String>,
    pub luxury_assigned_to_city_state: Vec<String>,
    /// For each type of luxury resource in this vector, we need to implement a dedicated placement function to handle it.
    pub luxury_assigned_to_special_case: Vec<String>,
    pub luxury_assigned_to_random: Vec<String>,
    pub luxury_not_being_used: Vec<String>,
}

impl Default for LuxuryResourceRole {
    fn default() -> Self {
        Self {
            luxury_assigned_to_regions: HashSet::new(),
            luxury_assigned_to_city_state: Vec::new(),
            luxury_assigned_to_special_case: Vec::new(),
            luxury_assigned_to_random: Vec::new(),
            luxury_not_being_used: Vec::new(),
        }
    }
}

pub struct ResourceToPlace {
    /// `resource` is the name of the resource.
    pub resource: String,
    /// `quantity` is the number of the resource will be placed on one tile.
    pub quantity: u32,
    /// `weight` is used to determine the probability of placing the resource on a tile.
    pub weight: u32,
    /// `min_radius` is related to resource_impact when we place resources on tiles.
    pub min_radius: u32,
    /// `max_radius` is related to resource_impact when we place resources on tiles.
    pub max_radius: u32,
}
