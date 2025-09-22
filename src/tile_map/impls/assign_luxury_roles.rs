use arrayvec::ArrayVec;
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    seq::SliceRandom,
};

use crate::{
    grid::WorldSizeType,
    map_parameters::MapParameters,
    tile_component::{Resource, TerrainType},
    tile_map::{TileMap, impls::generate_regions::RegionType},
};

impl TileMap {
    // function AssignStartingPlots:AssignLuxuryRoles
    /// Assigns luxury resources roles.
    ///
    /// Every luxury type has a role, the role should be one of the following (tackle with special cases separately):
    /// - Special case. For example, Marble. We need to implement a dedicated placement function to handle it.
    /// - Exclusively Assigned to regions. These luxury types only appear in no more than [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`] different regions.
    /// - Exclusively Assigned to city states. These luxury types are exclusive to city states. These types is limited to [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES`] in original CIV5.
    /// - Not exclusively assigned to any region or city state, and not special case too. we will place it randomly. That means it can be placed in any region or city state.
    /// - Disabled. We will not place it on the map.
    ///
    /// Assigns a Luxury resource according the rules below (tackle with special cases separately):
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

        let mut luxury_assigned_to_regions = ArrayVec::new();
        for region_index in 0..self.region_list.len() {
            let resource = self.assign_luxury_to_region(region_index, map_parameters);
            self.region_list[region_index].exclusive_luxury = Some(resource);
            luxury_assigned_to_regions.push(resource);
            *self
                .luxury_assign_to_region_count
                .entry(resource)
                .or_insert(0) += 1;
        }

        let luxury_city_state_weights: Vec<(Resource, usize)> = vec![
            (Resource::Whales, 15),
            (Resource::Pearls, 15),
            (Resource::GoldOre, 10),
            (Resource::Silver, 10),
            (Resource::Gems, 10),
            (Resource::Ivory, 10),
            (Resource::Furs, 15),
            (Resource::Dyes, 10),
            (Resource::Spices, 15),
            (Resource::Silk, 15),
            (Resource::Sugar, 10),
            (Resource::Cotton, 10),
            (Resource::Wine, 10),
            (Resource::Incense, 15),
            (Resource::Copper, 10),
            (Resource::Salt, 10),
            (Resource::Citrus, 15),
            (Resource::Truffles, 15),
            (Resource::Crab, 15),
            (Resource::Cocoa, 10),
        ];

        // Assign `MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES` of the remaining resources to be exclusive to City States.
        // Get the list of candidate resources and their weight that are not assigned to regions.
        let mut luxury_candidates_and_weights: Vec<_> = luxury_city_state_weights
            .iter()
            .filter(|(luxury_resource, _)| !luxury_assigned_to_regions.contains(luxury_resource))
            .collect();

        let mut luxury_assigned_to_city_state = ArrayVec::new();

        for _ in 0..MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES {
            if luxury_candidates_and_weights.is_empty() {
                break;
            }

            let dist = WeightedIndex::new(
                luxury_candidates_and_weights
                    .iter()
                    .map(|(_, weight)| *weight),
            )
            .unwrap();
            let index = dist.sample(&mut self.random_number_generator);

            let &(resource, _) = luxury_candidates_and_weights.swap_remove(index);
            luxury_assigned_to_city_state.push(resource);
        }

        // Assign Marble to special casing.
        let luxury_assigned_to_special_case = vec![Resource::Marble];

        // Assign appropriate amount to be Disabled, then assign the rest to be Random.
        let num_disabled_luxury_resource_type =
            get_disabled_luxuries_target_number(map_parameters.world_grid.world_size_type);

        // Get the list of resources that are not assigned to regions or city states.
        let mut remaining_resource_list = luxury_city_state_weights
            .iter()
            .filter(|(luxury_resource, _)| {
                !luxury_assigned_to_regions.contains(luxury_resource)
                    && !luxury_assigned_to_city_state.contains(luxury_resource)
            })
            .map(|&(luxury_resource, _)| luxury_resource)
            .collect::<Vec<_>>();

        remaining_resource_list.shuffle(&mut self.random_number_generator);

        let luxury_assigned_to_random = remaining_resource_list.split_off(
            num_disabled_luxury_resource_type.min(remaining_resource_list.len() as u32) as usize,
        );
        // skip shrink_to_fit if memory usage isn't critical
        /* remaining_resource_list.shrink_to_fit(); */
        let luxury_not_being_used = remaining_resource_list;

        self.luxury_resource_role = LuxuryResourceRole {
            luxury_assigned_to_regions,
            luxury_assigned_to_city_state,
            luxury_assigned_to_special_case,
            luxury_assigned_to_random,
            _luxury_not_being_used: luxury_not_being_used,
        };
    }

    // function AssignStartingPlots:AssignLuxuryToRegion
    /// Assigns a luxury type to a region, ensuring no resource is assigned to more than [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`] regions and no more than [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`] resources are assigned to regions.
    ///
    /// View [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`] and [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`] for more information.
    fn assign_luxury_to_region(
        &mut self,
        region_index: usize,
        map_parameters: &MapParameters,
    ) -> Resource {
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
            (Resource::Whales, 10),
            (Resource::Pearls, 10),
            (Resource::GoldOre, 10),
            (Resource::Silver, 5),
            (Resource::Gems, 10),
            (Resource::Ivory, 5),
            (Resource::Furs, 10),
            (Resource::Dyes, 5),
            (Resource::Spices, 5),
            (Resource::Silk, 5),
            (Resource::Sugar, 5),
            (Resource::Cotton, 5),
            (Resource::Wine, 5),
            (Resource::Incense, 5),
            (Resource::Copper, 5),
            (Resource::Salt, 5),
            (Resource::Citrus, 5),
            (Resource::Truffles, 5),
            (Resource::Crab, 10),
            (Resource::Cocoa, 5),
        ];

        let luxury_candidates = match region_type {
            RegionType::Undefined => luxury_fallback_weights.clone(),
            RegionType::Tundra => vec![
                (Resource::Furs, 40),
                (Resource::Whales, 35),
                (Resource::Crab, 30),
                (Resource::Silver, 25),
                (Resource::Copper, 15),
                (Resource::Salt, 15),
                (Resource::Gems, 5),
                (Resource::Dyes, 5),
            ],
            RegionType::Jungle => vec![
                (Resource::Cocoa, 35),
                (Resource::Citrus, 35),
                (Resource::Spices, 30),
                (Resource::Gems, 20),
                (Resource::Sugar, 20),
                (Resource::Pearls, 20),
                (Resource::Copper, 5),
                (Resource::Truffles, 5),
                (Resource::Crab, 5),
                (Resource::Silk, 5),
                (Resource::Dyes, 5),
            ],
            RegionType::Forest => vec![
                (Resource::Dyes, 30),
                (Resource::Silk, 30),
                (Resource::Truffles, 30),
                (Resource::Furs, 10),
                (Resource::Spices, 10),
                (Resource::Citrus, 5),
                (Resource::Salt, 5),
                (Resource::Copper, 5),
                (Resource::Cocoa, 5),
                (Resource::Crab, 10),
                (Resource::Whales, 10),
                (Resource::Pearls, 10),
            ],
            RegionType::Desert => vec![
                (Resource::Incense, 35),
                (Resource::Salt, 15),
                (Resource::GoldOre, 25),
                (Resource::Copper, 10),
                (Resource::Cotton, 15),
                (Resource::Sugar, 15),
                (Resource::Pearls, 5),
                (Resource::Citrus, 5),
            ],
            RegionType::Hill => vec![
                (Resource::GoldOre, 30),
                (Resource::Silver, 30),
                (Resource::Copper, 30),
                (Resource::Gems, 15),
                (Resource::Pearls, 15),
                (Resource::Salt, 10),
                (Resource::Crab, 10),
                (Resource::Whales, 10),
            ],
            RegionType::Plain => vec![
                (Resource::Ivory, 35),
                (Resource::Wine, 35),
                (Resource::Salt, 25),
                (Resource::Incense, 10),
                (Resource::Spices, 5),
                (Resource::Whales, 5),
                (Resource::Pearls, 5),
                (Resource::Crab, 5),
                (Resource::Truffles, 5),
                (Resource::GoldOre, 5),
            ],
            RegionType::Grassland => vec![
                (Resource::Cotton, 30),
                (Resource::Silver, 20),
                (Resource::Sugar, 20),
                (Resource::Copper, 20),
                (Resource::Crab, 20),
                (Resource::Pearls, 10),
                (Resource::Whales, 10),
                (Resource::Cocoa, 10),
                (Resource::Truffles, 5),
                (Resource::Spices, 5),
                (Resource::Gems, 5),
            ],
            RegionType::Hybrid => vec![
                (Resource::Ivory, 15),
                (Resource::Cotton, 15),
                (Resource::Wine, 15),
                (Resource::Silver, 10),
                (Resource::Salt, 15),
                (Resource::Copper, 20),
                (Resource::Whales, 20),
                (Resource::Pearls, 20),
                (Resource::Crab, 20),
                (Resource::Truffles, 10),
                (Resource::Cocoa, 10),
                (Resource::Spices, 5),
                (Resource::Sugar, 5),
                (Resource::Incense, 5),
                (Resource::Silk, 5),
                (Resource::Gems, 5),
                (Resource::GoldOre, 5),
            ],
        };

        let max_regions_per_exclusive_luxury = match map_parameters.num_civilization as usize {
            n if n >= MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS * 3 / 2 => {
                MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE
            }
            n if n > MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS => 2,
            _ => 1,
        };

        let num_assigned_luxury_types = self.luxury_assign_to_region_count.len();

        // Check if the luxury resource is eligible to be assigned to the region.
        // The luxury resource is eligible if:
        // 1. The luxury assignment count is less than the maximum regions per luxury type.
        //    Usually the maximum regions per luxury type is determined by the number of civilizations in the game.
        //    When we use fallback options, the maximum regions per luxury type is `MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE` (3 in original CIV5).
        // 2. The number of assigned luxury types should <= `MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS` (8 in original CIV5).
        //    - If num_assigned_luxury_types < `MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`, then we can assign more luxury types to regions.
        //    - If num_assigned_luxury_types = `MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`, then we can only assign luxury types to regions that are already assigned to regions.
        let is_eligible_luxury_resource =
            |luxury_resource: Resource,
             luxury_assignment_count: u32,
             max_regions_per_luxury_type: u32| {
                luxury_assignment_count < max_regions_per_luxury_type
                    && (num_assigned_luxury_types
                        < MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS
                        || self
                            .luxury_resource_role
                            .luxury_assigned_to_regions
                            .contains(&luxury_resource))
            };

        let mut resource_list = Vec::new();
        let mut resource_weight_list = Vec::new();
        for &(luxury_resource, weight) in luxury_candidates.iter() {
            let luxury_assign_to_region_count: u32 = *self
                .luxury_assign_to_region_count
                .get(&luxury_resource)
                .unwrap_or(&0);

            if is_eligible_luxury_resource(
                luxury_resource,
                luxury_assign_to_region_count,
                max_regions_per_exclusive_luxury,
            ) {
                // This type still eligible.
                // Water-based resources need to run a series of permission checks: coastal start in region, not a disallowed regions type, enough water, etc.
                if luxury_resource == Resource::Whales
                    || luxury_resource == Resource::Pearls
                    || luxury_resource == Resource::Crab
                {
                    // The code below is commented is unnecessary in the current implementation,
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
                        && region.terrain_statistic.terrain_type_num[TerrainType::Water] >= 12
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

        // If options list is empty and region type isn't undefined and `max_regions_per_exclusive_luxury` isn't `MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`, try to pick from fallback options.
        // We don't need to run the code below when region type is undefined and `max_regions_per_exclusive_luxury` is `MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`,
        // because in this situation `luxury_candidates` is equal to fallback options, and we have already run the same function code above.
        if resource_list.is_empty()
            && region_type != RegionType::Undefined
            && max_regions_per_exclusive_luxury
                != MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE
        {
            for &(luxury_resource, weight) in luxury_fallback_weights.iter() {
                let luxury_assign_to_region_count: u32 = *self
                    .luxury_assign_to_region_count
                    .get(&luxury_resource)
                    .unwrap_or(&0);
                if is_eligible_luxury_resource(
                    luxury_resource,
                    luxury_assign_to_region_count,
                    MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE,
                ) {
                    // This type still eligible.
                    // Water-based resources need to run a series of permission checks: coastal start in region, not a disallowed regions type, enough water, etc.
                    if luxury_resource == Resource::Whales
                        || luxury_resource == Resource::Pearls
                        || luxury_resource == Resource::Crab
                    {
                        // Diffent with the code commented above, this code is necessary here,
                        // because `luxury_fallback_weights` is not filtered according to the region type.
                        if luxury_resource == Resource::Whales && region_type == RegionType::Jungle
                        {
                            // Whales are not allowed in Jungle regions.
                            continue;
                        } else if luxury_resource == Resource::Pearls
                            && region_type == RegionType::Tundra
                        {
                            // Pearls are not allowed in Tundra regions.
                            continue;
                        } else if luxury_resource == Resource::Crab
                            && region_type == RegionType::Desert
                        {
                            // Crabs are not allowed in Desert regions.
                            // NOTE: In the original code, this check is not present. I think it is a bug.
                            continue;
                        } else if region.start_location_condition.along_ocean
                            && region.terrain_statistic.terrain_type_num[TerrainType::Water] >= 12
                        {
                            // Water-based luxuries are allowed if both of the following are true:
                            // 1. This region's start is along an ocean,
                            // 2. This region has enough water to support water-based luxuries.
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
            for &(luxury_resource, weight) in luxury_candidates.iter() {
                let luxury_assign_to_region_count: u32 = *self
                    .luxury_assign_to_region_count
                    .get(&luxury_resource)
                    .unwrap_or(&0);
                if is_eligible_luxury_resource(
                    luxury_resource,
                    luxury_assign_to_region_count,
                    MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE,
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

        resource_list[dist.sample(&mut self.random_number_generator)]
    }
}

/// The role of luxury resources. View [`TileMap::assign_luxury_roles`] for more information.
#[derive(PartialEq, Eq, Default, Debug)]
pub struct LuxuryResourceRole {
    /// Exclusively Assigned to regions. The length of this set is limited to [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`].
    ///
    /// In original CIV5, the same luxury resource can only be found in at most 3 regions on the map.
    /// Because there are a maximum of 22 civilizations (each representing a region) in the game, so these luxury types are limited to 8 in original CIV5.
    pub luxury_assigned_to_regions:
        ArrayVec<Resource, { MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS }>,

    /// Exclusively Assigned to a city state. The length of this set is limited to [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES`].
    ///
    /// These luxury types are exclusive to city states. These types is limited to 3 in original CIV5.
    pub luxury_assigned_to_city_state:
        ArrayVec<Resource, { MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES }>,

    /// Special case. For example, `Marble`. For each type of luxury resource in this vector, we need to implement a dedicated placement function to handle it.
    pub luxury_assigned_to_special_case: Vec<Resource>,

    /// Not exclusively assigned to any region or city state, and not special case too. we will place it randomly. That means it can be placed in any region or city state.
    pub luxury_assigned_to_random: Vec<Resource>,

    /// Disabled. We will not place it on the map.
    pub _luxury_not_being_used: Vec<Resource>,
}

/// Determines the target number of disabled luxury resources which can not be placed on the map.
fn get_disabled_luxuries_target_number(world_size_type: WorldSizeType) -> u32 {
    match world_size_type {
        WorldSizeType::Duel => 11,
        WorldSizeType::Tiny => 8,
        WorldSizeType::Small => 6,
        WorldSizeType::Standard => 4,
        WorldSizeType::Large => 2,
        WorldSizeType::Huge => 1,
    }
}
