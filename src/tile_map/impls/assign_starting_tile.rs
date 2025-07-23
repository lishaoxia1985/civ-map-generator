use std::cmp::min;

use std::collections::HashSet;

use rand::{distributions::WeightedIndex, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{
    component::map_component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    map_parameters::{MapParameters, ResourceSetting},
    ruleset::Ruleset,
    tile::Tile,
    tile_map::{Layer, TileMap},
};

use super::generate_regions::RegionType;

impl TileMap {
    pub fn start_plot_system(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.choose_civilization_starting_tiles(map_parameters);

        self.balance_and_assign_civilization_starting_tiles(map_parameters, ruleset);

        self.place_natural_wonders(ruleset);

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

        // We have replace this code with `TileMap::generate_bonus_resource_tile_lists_in_map`,
        // `TileMap::generate_luxury_resource_tile_lists_in_map` and `TileMap::generate_strategic_resource_tile_lists_in_map`.
        /* self:GenerateGlobalResourcePlotLists() */

        self.place_luxury_resources(map_parameters, ruleset);

        self.place_strategic_resources(map_parameters);

        self.place_bonus_resources(map_parameters);

        self.normalize_city_state_locations();

        self.fix_sugar_jungles();

        self.recalculate_areas(ruleset);
    }

    /// Fix Sugar graphics. That because in origin CIV5, `Sugar` could not be made visible enough in jungle, so turn any sugar jungle to marsh.
    ///
    /// Change all the terrains which both have [`Feature::Jungle`] and resource `Sugar` to a [`TerrainType::Flatland`]
    /// with [`BaseTerrain::Grassland`] and [`Feature::Marsh`].
    fn fix_sugar_jungles(&mut self) {
        self.all_tiles().for_each(|tile| {
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

    // function AssignStartingPlots:AssignLuxuryRoles
    /// Assigns luxury resources roles.
    ///
    /// Every luxury type has a role, the role should be one of the following:
    /// - Special case. For example, Marble. We need to implement a dedicated placement function to handle it.
    /// - Exclusively Assigned to a region. Each region gets an individual Luxury type assigned to it. These types are limited to 8 in original CIV5.
    /// - Exclusively Assigned to a city state. These luxury types are exclusive to city states. These types is limited to 3 in original CIV5.
    /// - Not exclusively assigned to any region or city state, and not special case too. we will place it randomly. That means it can be placed in any region or city state.
    /// - Disabled. We will not place it on the map.
    ///
    /// Assigns a Luxury resource according the rules below:
    /// - first, assign to regions
    /// - then, assign to city states
    /// - then, radomly assign
    /// - then, disable
    ///
    /// # Notice
    ///
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

        let mut luxury_assigned_to_regions = HashSet::new();
        for region_index in 0..self.region_list.len() {
            let resource = self.assign_luxury_to_region(map_parameters, region_index);
            // TODO: Should be edited in the future
            self.region_list[region_index].exclusive_luxury = resource.name().to_string();
            luxury_assigned_to_regions.insert(resource.name().to_string());
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
                    !luxury_assigned_to_regions.contains(luxury_resource.name())
                })
                .map(|(luxury_resource, weight)| (luxury_resource.name(), weight))
                .unzip();

        let mut luxury_assigned_to_city_state = Vec::new();
        for _ in 0..3 {
            // Choose a random resource from the list.
            let dist: WeightedIndex<usize> =
                WeightedIndex::new(resource_weight_list.clone()).unwrap();
            let index = dist.sample(&mut self.random_number_generator);
            let resource = resource_list[index];
            // Remove it from the list and assign it to the city state.
            luxury_assigned_to_city_state.push(resource.to_string());
            resource_weight_list.remove(index);
            resource_list.remove(index);
        }

        // Assign Marble to special casing.
        let luxury_assigned_to_special_case = vec!["Marble".to_string()];

        // Assign appropriate amount to be Disabled, then assign the rest to be Random.

        // The amount of disabled resources should be determined by the map size.
        // Please view `AssignStartingPlots:GetDisabledLuxuriesTargetNumber` for more information.
        // TODO: Implement this as one field of the map_parameters in the map_parameters.rs file in the future.
        let num_disabled_luxury_resource = 0;

        // Get the list of resources that are not assigned to regions or city states.
        let mut remaining_resource_list = luxury_city_state_weights
            .iter()
            .filter(|(luxury_resource, _)| {
                !luxury_assigned_to_regions.contains(luxury_resource.name())
                    && !luxury_assigned_to_city_state.contains(&luxury_resource.name().to_string())
            })
            .map(|(luxury_resource, _)| luxury_resource.name().to_string())
            .collect::<Vec<_>>();

        remaining_resource_list.shuffle(&mut self.random_number_generator);

        let mut luxury_not_being_used = Vec::new();
        let mut luxury_assigned_to_random = Vec::new();

        for resource in remaining_resource_list {
            if luxury_not_being_used.len() < num_disabled_luxury_resource {
                luxury_not_being_used.push(resource);
            } else {
                luxury_assigned_to_random.push(resource);
            }
        }

        self.luxury_resource_role = LuxuryResourceRole {
            luxury_assigned_to_regions,
            luxury_assigned_to_city_state,
            luxury_assigned_to_special_case,
            luxury_assigned_to_random,
            _luxury_not_being_used: luxury_not_being_used,
        };
    }

    // function AssignStartingPlots:AssignLuxuryToRegion
    /// Assigns a luxury type to a region, ensuring no resource is assigned to more than 3 regions and no more than 8 resources are assigned to regions.
    ///
    /// # Why we need to ensure no resource is assigned to more than 3 regions and no more than 8 resources are assigned to regions?
    ///
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
                        && region.terrain_statistic.terrain_type_num[TerrainType::Water] > 12
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
                            && region.terrain_statistic.terrain_type_num[TerrainType::Water] > 12
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

    // function AssignStartingPlots:AttemptToPlaceHillsAtPlot
    /// Attempts to place a Hill at the currently chosen tile.
    /// If successful, it returns `true`, otherwise it returns `false`.
    pub fn attempt_to_place_hill_at_tile(&mut self, tile: Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) != TerrainType::Water
            && tile.feature(self) != Some(Feature::Forest)
            && !tile.has_river(self)
        {
            self.terrain_type_query[tile.index()] = TerrainType::Hill;
            self.feature_query[tile.index()] = None;
            self.natural_wonder_query[tile.index()] = None;
            true
        } else {
            false
        }
    }

    // function AssignStartingPlots:AttemptToPlaceSmallStrategicAtPlot
    /// Attempts to place a Small `Horses` or `Iron` Resource at the currently chosen tile.
    /// If successful, it returns `true`, otherwise it returns `false`.
    pub fn attempt_to_place_small_strategic_at_tile(&mut self, tile: Tile) -> bool {
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
            true
        } else {
            false
        }
    }

    // function AssignStartingPlots:AttemptToPlaceBonusResourceAtPlot
    /// Attempts to place a Bonus Resource at the currently chosen tile.
    ///
    /// # Returns
    ///
    /// Returns a tuple of two booleans:
    ///
    /// - The first boolean is `true` if something was placed.
    /// - The second boolean is `true` as well if [`Feature::Oasis`] was placed.
    pub fn attempt_to_place_bonus_resource_at_tile(
        &mut self,
        tile: Tile,
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
                    if base_terrain == BaseTerrain::Coast && feature.is_none() {
                        self.resource_query[tile.index()] =
                            Some((Resource::Resource("Fish".to_owned()), 1));
                        return (true, false);
                    }
                }
                TerrainType::Flatland => {
                    if feature.is_none() {
                        match base_terrain {
                            BaseTerrain::Grassland => {
                                self.resource_query[tile.index()] =
                                    Some((Resource::Resource("Cow".to_owned()), 1));
                                return (true, false);
                            }
                            BaseTerrain::Desert => {
                                if tile.is_freshwater(self) {
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
                    if feature.is_none() {
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
    /// Attempts to place a stone at a grass plot.
    /// Returns `true` if Stone is placed. Otherwise returns `false`.
    pub fn attempt_to_place_stone_at_grass_tile(&mut self, tile: Tile) -> bool {
        if tile.resource(self).is_none()
            && tile.terrain_type(self) == TerrainType::Flatland
            && tile.base_terrain(self) == BaseTerrain::Grassland
            && tile.feature(self).is_none()
        {
            self.resource_query[tile.index()] = Some((Resource::Resource("Stone".to_owned()), 1));
            true
        } else {
            false
        }
    }

    // function AssignStartingPlots:ProcessResourceList
    /// Placing bonus or strategic resources on the map based on the given parameters.
    ///
    /// It iterates through the list of plots and places resources on eligible plots based on the
    /// resource type, quantity, and radius.\
    /// Before using this function, make sure `tile_list` has been shuffled.
    ///
    /// # Arguments
    ///
    /// - `frequency`: The frequency of resource placement.\
    ///   The num of tiles we will assign this resource is `(plot_list.len() as f64 / frequency).ceil() as u32`.
    /// - `layer`: The layer on which the resource will be placed.
    /// - `tile_list`: A vector of tiles representing the plots where resources can be placed. Before using this argument, make sure the vector has been shuffled.
    /// - `resource_list_to_place`: A vector of resource to place, which contains the resource type,
    ///   quantity, minimum radius, and maximum radius for each resource.
    ///
    /// # Panics
    ///
    /// This function will panic if the layer is not [`Layer::Bonus`] or [`Layer::Strategic`]. That means if you use this function to place luxury resources, it will panic.
    ///
    /// # Notice
    ///
    /// Although in the original CIV5, this function has some code about placing luxury resources, but in fact, it is never used to place luxury resources. So, we forbid placing luxury resources in this function.
    /// If you want to place luxury resources, please use [`TileMap::place_specific_number_of_resources`].
    pub fn process_resource_list(
        &mut self,
        frequency: f64,
        layer: Layer,
        tile_list: &[Tile],
        resource_list_to_place: &[ResourceToPlace],
    ) {
        debug_assert!(layer == Layer::Bonus || layer == Layer::Strategic,
            "`process_resource_list` can only be used to place bonus or strategic resources, not luxury resources.
            If you want to place luxury resources, please use `place_specific_number_of_resources` instead."
        );

        if tile_list.is_empty() {
            return;
        }

        let resource_weight = resource_list_to_place
            .iter()
            .map(|resource| resource.weight)
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(resource_weight).unwrap();

        let num_resources_to_place = (tile_list.len() as f64 / frequency).ceil() as u32;

        let mut plot_list_iter = tile_list.iter().peekable();

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
            for &tile in plot_list_iter.by_ref() {
                if self.layer_data[layer][tile.index()] == 0 && tile.resource(self).is_none() {
                    self.resource_query[tile.index()] =
                        Some((Resource::Resource(resource.to_string()), quantity));
                    self.place_impact_and_ripples(tile, layer, radius);
                    break;
                }
            }

            // Completed first pass of plot_list, now change to seeking lowest value instead of zero value.
            // If no eligible 0 value is found, second pass: Seek the lowest value (value < 98) on the impact matrix
            if plot_list_iter.peek().is_none() {
                let best_plot = tile_list
                    .iter()
                    .filter(|&&tile| {
                        self.layer_data[layer][tile.index()] < 98 && tile.resource(self).is_none()
                    })
                    .min_by_key(|tile| self.layer_data[layer][tile.index()]);
                if let Some(&tile) = best_plot {
                    self.resource_query[tile.index()] =
                        Some((Resource::Resource(resource.to_string()), quantity));
                    self.place_impact_and_ripples(tile, layer, radius);
                }
            }
        }
    }

    // function AssignStartingPlots:PlaceSpecificNumberOfResources
    /// Places a specific number of resources on the map.
    ///
    /// Before calling this function, make sure `tile_list` has been shuffled.
    ///
    /// # Arguments
    ///
    /// - `quantity`: The number of every type resource that can be placed on the tile.\
    ///   For example, when placing `Horses`, `quantity` is 2, which means that the tile has 2 `Horses`.\
    ///   In CIV5, when resource is bonus or luxury, `quantity` is always 1;
    ///   When resource is strategic, `quantity` is usually determined by [`ResourceSetting`].
    /// - `amount`: The number of tiles intended to receive an assignment of this resource.
    /// - `ratio`: Determines when secondary and tertiary lists come in to play, should be in (0, 1].\
    ///   The num of tiles we will assign this resource is the minimum of `amount` and `(ratio * tile_list.len() as f64).ceil() as u32`.\
    ///   For example, if we are assigning Sugar resources to Marsh, then if we are to assign 8 Sugar
    ///   resources (`amount = 8`), but there are only 4 Marsh plots in the list (`tile_list.len() = 4`):
    ///     - `ratio = 1`, the num of tiles we will assign is 4, we would assign a Sugar to every single marsh plot, and then the function return an unplaced value of 4.
    ///     - `ratio = 0.5`, the num of tiles we will assign is 2, we would assign only 2 Sugars to the 4 marsh plots, and the function return a value of 6.
    ///     - `ratio <= 0.25`, the num of tiles we will assign is 1, we would assign 1 Sugar and return 7, as the ratio results will be rounded up not down, to the nearest integer.
    /// - `layer`: The layer we should tackle resource impact or ripple. If None, the resource can be placed on any tiles of `tile_list` that are not already assigned to a resource.
    /// - `min_radius` and `max_radius`: Related to `resource_impact` when we place resources on tiles.
    ///     - If `layer` is None, then `min_radius` and `max_radius` are ignored.
    ///     - If `layer` is not [`Layer::Strategic`], [`Layer::Luxury`], [`Layer::Bonus`], or [`Layer::Fish`], then `min_radius` and `max_radius` are ignored as well.
    /// - `tile_list`: The list of tiles that are candidates to place the resource on.
    ///
    /// # Returns
    ///
    /// - The number of resources that were not placed.
    ///   It is equal to `amount` minus the number of tiles that were assigned a resource.
    ///
    /// # Panics
    ///
    /// - `max_radius` must be greater than or equal to `min_radius`. Otherwise, the function will panic.
    #[allow(clippy::too_many_arguments)]
    pub fn place_specific_number_of_resources(
        &mut self,
        resource: Resource,
        quantity: u32,
        amount: u32,
        ratio: f64,
        layer: Option<Layer>,
        min_radius: u32,
        max_radius: u32,
        tile_list: &[Tile],
    ) -> u32 {
        debug_assert!(
            max_radius >= min_radius,
            "'max_radius' must be greater than or equal to 'min_radius'!"
        );

        if tile_list.is_empty() {
            return amount;
        }

        let has_impact = matches!(
            layer,
            Some(Layer::Strategic | Layer::Luxury | Layer::Bonus | Layer::Fish)
        );

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

        for _ in 1..=num_resources {
            for &tile in tile_list.iter() {
                if !has_impact || self.layer_data[layer.unwrap()][tile.index()] == 0 {
                    // Place resource on tile if it doesn't have a resource already
                    if tile.resource(self).is_none() {
                        self.resource_query[tile.index()] = Some((resource.clone(), quantity));
                        num_left_to_place -= 1;
                    }
                    // Place impact and ripples if `has_impact` is true
                    if has_impact {
                        let radius = self
                            .random_number_generator
                            .gen_range(min_radius..=max_radius);
                        self.place_impact_and_ripples(tile, layer.unwrap(), radius)
                    }
                    break;
                }
            }
        }

        num_left_to_place
    }
}

/// The role of luxury resources. View [`TileMap::assign_luxury_roles`] for more information.
#[derive(Default)]
pub struct LuxuryResourceRole {
    /// Exclusively Assigned to regions. The length of this set is limited to 8 in original CIV5.
    ///
    /// In original CIV5, the same luxury resource can only be found in at most 3 regions on the map.
    /// Because there are a maximum of 22 civilizations (each representing a region) in the game, so these luxury types are limited to 8 in original CIV5.
    pub luxury_assigned_to_regions: HashSet<String>,
    /// Exclusively Assigned to a city state. These luxury types are exclusive to city states. These types is limited to 3 in original CIV5.
    pub luxury_assigned_to_city_state: Vec<String>,
    /// Special case. For example, `Marble`. For each type of luxury resource in this vector, we need to implement a dedicated placement function to handle it.
    pub luxury_assigned_to_special_case: Vec<String>,
    /// Not exclusively assigned to any region or city state, and not special case too. we will place it randomly. That means it can be placed in any region or city state.
    pub luxury_assigned_to_random: Vec<String>,
    /// Disabled. We will not place it on the map.
    pub _luxury_not_being_used: Vec<String>,
}

pub struct ResourceToPlace {
    /// The name of the resource.
    pub resource: String,
    /// The number of the resource will be placed on one tile.
    pub quantity: u32,
    /// Determine the probability of placing the resource on a tile.
    pub weight: u32,
    /// Related to `resource_impact` when we place resources on tiles.
    pub min_radius: u32,
    /// Related to `resource_impact` when we place resources on tiles.
    pub max_radius: u32,
}

// function AssignStartingPlots:GetMajorStrategicResourceQuantityValues
// TODO: This function should be implemented in future.
/// Determines the quantity per tile for each strategic resource's major deposit size.
///
/// # Notice
///
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
