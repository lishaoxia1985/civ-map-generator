use std::collections::HashSet;

use arrayvec::ArrayVec;
use rand::{
    distr::{Distribution, weighted::WeightedIndex},
    seq::SliceRandom,
};

use crate::{
    grid::WorldSizeType,
    map_parameters::MapParameters,
    ruleset::terrain_type,
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
    /// # Notes
    ///
    /// Luxury roles must be assigned before placing City States.
    /// This is because civs who are forced to share their luxury type with other
    /// civs may get extra city states placed in their region to compensate. View [`TileMap::assign_city_states_to_regions_or_uninhabited_landmasses`] for more information.
    pub fn assign_luxury_roles(&mut self, map_parameters: &MapParameters) {
        // Sort the regions by their type, with `RegionType::Undefined` being sorted last.
        // Notice: In original code, the region which has the same type should be shuffled. But here we don't do that. We will implement it in the future.
        self.region_list.sort_by_key(|region| {
            let region_type = *region.region_type.get().unwrap();
            if region_type == RegionType::Undefined {
                9 // Place undefined regions at the end
            } else {
                region_type as i32 // Otherwise, use the region type value for sorting
            }
        });

        for region_index in 0..self.region_list.len() {
            let resource = self.assign_luxury_to_region(region_index, map_parameters);
            self.region_exclusive_luxury_list.push(resource);
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
            .filter(|(luxury, _)| !self.region_exclusive_luxury_list.contains(luxury))
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
        let num_disabled_luxury_type =
            get_disabled_luxuries_target_number(map_parameters.world_grid.world_size_type);

        // Get the list of resources that are not assigned to regions or city states.
        let mut remaining_resource_list = luxury_city_state_weights
            .iter()
            .filter(|(luxury, _)| {
                !self.region_exclusive_luxury_list.contains(luxury)
                    && !luxury_assigned_to_city_state.contains(luxury)
            })
            .map(|&(luxury, _)| luxury)
            .collect::<Vec<_>>();

        remaining_resource_list.shuffle(&mut self.random_number_generator);

        let luxury_assigned_to_random = remaining_resource_list
            .split_off(num_disabled_luxury_type.min(remaining_resource_list.len() as u32) as usize);
        // skip shrink_to_fit if memory usage isn't critical
        /* remaining_resource_list.shrink_to_fit(); */
        let luxury_not_being_used = remaining_resource_list;

        // Duplicate the list to get luxury types assigned to regions
        let mut seen = HashSet::new();
        let mut regions_exclusive: ArrayVec<
            Resource,
            { MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS },
        > = ArrayVec::new();

        for resource in self.region_exclusive_luxury_list.iter() {
            if seen.insert(*resource) {
                regions_exclusive.push(*resource);
            }
        }

        self.luxury_resource_role = LuxuryResourceRole {
            regions_exclusive,
            city_states_exclusive: luxury_assigned_to_city_state,
            special_cases: luxury_assigned_to_special_case,
            random_placement: luxury_assigned_to_random,
            disabled: luxury_not_being_used,
        };
    }

    // function AssignStartingPlots:AssignLuxuryToRegion
    /// Assigns a luxury type exclusive to a region.
    ///
    /// In the assign process, the rules are as follows:
    /// 1. No more than [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`] regions have the same luxury type.
    /// 2. No more than [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`] luxury types are assigned to regions.
    ///
    /// View [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`] and [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`] for more information.
    fn assign_luxury_to_region(
        &mut self,
        region_index: usize,
        map_parameters: &MapParameters,
    ) -> Resource {
        let region = &self.region_list[region_index];
        let region_type = *region.region_type.get().unwrap();
        let terrain_statistic = region.terrain_statistic.get().unwrap();
        let start_location_condition = region.start_location_condition.get().unwrap();

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

        let max_regions_per_exclusive_luxury =
            match map_parameters.world_size_type_profile.num_civilizations as usize {
                n if n >= MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS * 3 / 2 => {
                    MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE
                }
                n if n > MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS => 2,
                _ => 1,
            };

        let num_assigned_luxury_types = self
            .region_exclusive_luxury_list
            .iter()
            .collect::<HashSet<_>>()
            .len();

        // Closure to determine if a luxury resource is eligible for assignment to the current region
        let is_eligible_luxury = |luxury: Resource, max_regions_per_luxury_type: u32| {
            let luxury_assign_to_region_count: u32 =
                self.assigned_region_exclusive_luxury_count(luxury);
            // Condition 1: The number of assignments for this specific luxury type has not reached its limit
            let count_within_limit = luxury_assign_to_region_count < max_regions_per_luxury_type;

            // When there are already assignments for this luxury type, it is considered region-exclusive,
            // and it can continue to be assigned to more regions until it reaches the `max_regions_per_luxury_type` limit.
            let is_region_exclusive = luxury_assign_to_region_count > 0;

            // Condition 2: The total number of unique luxury types assigned to regions is below the global cap,
            // OR the cap is reached but this specific resource is already marked as region-exclusive
            let type_limit_ok = num_assigned_luxury_types
                < MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS
                || is_region_exclusive;

            // The resource is eligible only if both the count limit and the type limit are satisfied
            count_within_limit && type_limit_ok
        };

        let mut resource_list = Vec::new();
        let mut resource_weight_list = Vec::new();
        for &(luxury, weight) in luxury_candidates.iter() {
            let luxury_assign_to_region_count: u32 =
                self.assigned_region_exclusive_luxury_count(luxury);

            if is_eligible_luxury(luxury, max_regions_per_exclusive_luxury) {
                match (luxury, region_type) {
                    // This should never happen, because `luxury_candidates` has been filtered according to the region type.
                    // So when region type is Jungle, there shouldn't be Pearls in `luxury_candidates`,
                    // when region type is Tundra, there shouldn't be Furs in `luxury_candidates`,
                    // when region type is Desert, there shouldn't be Crab in `luxury_candidates`, etc.
                    // Please view the code relative to `luxury_candidates`.
                    (Resource::Whales, RegionType::Jungle)
                    | (Resource::Pearls, RegionType::Tundra)
                    | (Resource::Crab, RegionType::Desert) => unreachable!(),
                    (Resource::Whales | Resource::Pearls | Resource::Crab, _) => {
                        if start_location_condition.along_ocean
                            && terrain_statistic.terrain_type_count[TerrainType::Water] >= 12
                        {
                            // Water-based luxuries are allowed if both of the following are true:
                            // 1. This region's start is along an ocean,
                            // 2. This region has enough water to support water-based luxuries.
                            resource_list.push(luxury);
                            let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                            resource_weight_list.push(adjusted_weight);
                        }
                    }
                    _ => {
                        // Land-based luxuries only need to satisfy the eligibility requirement, no extra placement condition.
                        resource_list.push(luxury);
                        let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                        resource_weight_list.push(adjusted_weight);
                    }
                }
            }
        }

        // If options list is empty, use `luxury_fallback_weights` as fallback options.
        // Skip the situation when `region_type` is `Undefined` and `max_regions_per_exclusive_luxury` is equal to `MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`,
        // because in this situation `luxury_candidates` is equal to fallback options, and the code is the same as the for-loop code above.
        if resource_list.is_empty()
            && region_type != RegionType::Undefined
            && max_regions_per_exclusive_luxury
                != MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE
        {
            for &(luxury, weight) in luxury_fallback_weights.iter() {
                let luxury_assign_to_region_count: u32 =
                    self.assigned_region_exclusive_luxury_count(luxury);

                if is_eligible_luxury(luxury, MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE)
                {
                    // This type still eligible.
                    // Water-based resources need to run a series of permission checks: coastal start in region, not a disallowed regions type, enough water, etc.
                    if luxury == Resource::Whales
                        || luxury == Resource::Pearls
                        || luxury == Resource::Crab
                    {
                        match (luxury, region_type) {
                            // Different with the code above,
                            // `luxury_fallback_weights` stores all the luxury types in game and not filtered according to `region_type`.
                            // Please view the code relative to `luxury_fallback_weights`.
                            // so we need to check and skip that situation here to avoid assigning water-based luxury resources to incompatible region types.
                            (Resource::Whales, RegionType::Jungle)
                            | (Resource::Pearls, RegionType::Tundra)
                            | (Resource::Crab, RegionType::Desert) => continue,
                            (Resource::Whales | Resource::Pearls | Resource::Crab, _) => {
                                if start_location_condition.along_ocean
                                    && terrain_statistic.terrain_type_count[TerrainType::Water]
                                        >= 12
                                {
                                    // Water-based luxuries are allowed if both of the following are true:
                                    // 1. This region's start is along an ocean,
                                    // 2. This region has enough water to support water-based luxuries.
                                    resource_list.push(luxury);
                                    let adjusted_weight =
                                        weight / (1 + luxury_assign_to_region_count);
                                    resource_weight_list.push(adjusted_weight);
                                }
                            }
                            _ => {
                                // Land-based luxuries only need to satisfy the eligibility requirement, no extra placement condition.
                                resource_list.push(luxury);
                                let adjusted_weight = weight / (1 + luxury_assign_to_region_count);
                                resource_weight_list.push(adjusted_weight);
                            }
                        }
                    }
                }
            }
        }

        // If we get to here and still need to assign a luxury type, it means we have to force a water-based luxury in to this region, period.
        // This should be the rarest of the rare emergency assignment cases, unless modifications to the system have tightened things too far.
        if resource_list.is_empty() {
            for &(luxury, weight) in luxury_fallback_weights.iter() {
                if is_eligible_luxury(luxury, MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE)
                {
                    let luxury_assign_to_region_count: u32 =
                        self.assigned_region_exclusive_luxury_count(luxury);
                    resource_list.push(luxury);
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

    /// Returns the number of regions that have been assigned the specified region exclusive luxury resource type.
    ///
    /// When return value is 0, the specified luxury resource type is not the region exclusive luxury resource type.
    /// The return value should not exceed [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`].
    ///
    /// # Notes
    ///
    /// Before calling this function, please make sure the resource type is `luxury`, not `bonus` or `strategic`, etc. Otherwise, the return value may be meaningless.
    pub fn assigned_region_exclusive_luxury_count(&self, luxury: Resource) -> u32 {
        self.region_exclusive_luxury_list
            .iter()
            .filter(|&&r| r == luxury)
            .count() as u32
    }
}

/// The role of luxury resources. View [`TileMap::assign_luxury_roles`] for more information.
#[derive(PartialEq, Eq, Default, Debug)]
pub struct LuxuryResourceRole {
    /// Resources exclusively assigned to player regions.
    /// The length is limited by [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`].
    ///
    /// In original CIV5, the same luxury resource appears in at most 3 regions.
    /// Because there are a maximum of 22 civilizations (each representing a region) in the game, so these luxury types are limited to 8 in original CIV5.
    pub regions_exclusive:
        ArrayVec<Resource, { MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS }>,

    /// Resources exclusively assigned to City-States.
    /// The length is limited by [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES`].
    ///
    /// These luxury types are exclusive to city states. These types is limited to 3 in original CIV5.
    pub city_states_exclusive:
        ArrayVec<Resource, { MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES }>,

    /// Resources requiring special placement logic (e.g., `Marble`).
    ///
    /// Each resource in this collection requires a dedicated placement function
    /// to handle specific map generation rules.
    pub special_cases: Vec<Resource>,

    /// Not exclusively assigned to any region or city state, and not special case too.
    ///
    /// we will place it randomly. That means it can be placed in any region or city state.
    pub random_placement: Vec<Resource>,

    /// Disabled resources that will not be placed on the map.
    pub disabled: Vec<Resource>,
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
