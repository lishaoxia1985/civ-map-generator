use std::cmp::min;

use std::collections::{HashMap, HashSet};

use rand::{
    seq::{index::sample, SliceRandom},
    Rng,
};

use crate::{
    grid::offset_coordinate::OffsetCoordinate,
    map_parameters::{MapParameters, Rectangle, RegionDivideMethod},
    ruleset::Ruleset,
    tile::Tile,
    tile_component::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType},
    tile_map::{Layer, TileMap},
};

impl TileMap {
    // function AssignStartingPlots:PlaceCityStates
    /// Place city states on the map.
    ///
    /// This function depends on [`TileMap::assign_luxury_roles`] being executed first.
    /// This is because some city state placements are made as compensation for situations where
    /// multiple regions are assigned the same luxury resource type.
    pub fn place_city_states(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        let city_states_assignment =
            self.assign_city_states_to_regions_or_uninhabited_landmasses(map_parameters);

        let mut city_state_list = ruleset
            .nations
            .iter()
            .filter(|(_, nation)| !nation.city_state_type.is_empty())
            .map(|(city_state, _)| city_state)
            .collect::<Vec<_>>();

        // We get the civilization in the order.
        // That make sure we get the same civilization list every time we run the game.
        // We use `sort_unstable` instead of `sort` because there are no duplicate elements in the list.
        city_state_list.sort_unstable();

        let mut start_city_state_list: Vec<_> = sample(
            &mut self.random_number_generator,
            city_state_list.len(),
            map_parameters.city_state_num as usize,
        )
        .into_iter()
        .map(|i| city_state_list[i])
        .collect();

        let mut num_uninhabited_candidate_tiles = city_states_assignment
            .uninhabited_areas_coastal_land_tiles
            .len()
            + city_states_assignment.uninhabited_areas_inland_tiles.len();

        let uninhabited_areas_coastal_tile_list =
            city_states_assignment.uninhabited_areas_coastal_land_tiles;
        let uninhabited_areas_inland_tile_list =
            city_states_assignment.uninhabited_areas_inland_tiles;

        let candidate_tile_list = [
            uninhabited_areas_coastal_tile_list,
            uninhabited_areas_inland_tile_list,
        ];

        let mut num_city_states_discarded = 0;

        for region_index in city_states_assignment.region_index_assignment {
            if region_index.is_none() && num_uninhabited_candidate_tiles > 0 {
                num_uninhabited_candidate_tiles -= 1;
                let tile = self.get_city_state_start_tile(&candidate_tile_list, true, true);
                // Place city state on uninhabited land
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, None));
                } else {
                    num_city_states_discarded += 1;
                }
            } else if region_index.is_none() && num_uninhabited_candidate_tiles == 0 {
                // Place city state on a random region
                let region_index = self
                    .random_number_generator
                    .gen_range(0..self.region_list.len());
                let tile = self.get_city_state_start_tile_in_region(region_index);
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, Some(region_index)));
                } else {
                    num_city_states_discarded += 1;
                }
            } else {
                // Assigned to a Region.
                let region_index = region_index.unwrap();
                let tile = self.get_city_state_start_tile_in_region(region_index);
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, Some(region_index)));
                } else {
                    num_city_states_discarded += 1;
                }
            }
        }

        // Last chance method to place city states that didn't fit where they were supposed to go.
        // Notice: These codes below are different from the original code.
        //  - The original code chooses a random tile from the list of candidate tiles directly.
        //  - In our version we divide the candidate tiles into two lists, one for coastal and one for inland.
        //      We choose the tile from the list of coastal tiles first.
        //      If there are no coastal tiles, we choose from the list of inland tiles.
        if num_city_states_discarded > 0 {
            let mut coastal_tile_list = Vec::new();
            let mut inland_tile_list = Vec::new();

            self.all_tiles().for_each(|tile| {
                if tile.can_be_city_state_starting_tile(self, None) {
                    if tile.is_coastal_land(self) {
                        coastal_tile_list.push(tile);
                    } else {
                        inland_tile_list.push(tile);
                    }
                }
            });

            let candidate_tile_list = [coastal_tile_list, inland_tile_list];

            for city_state in start_city_state_list.iter() {
                let tile = self.get_city_state_start_tile(&candidate_tile_list, true, true);
                if let Some(tile) = tile {
                    self.place_city_state(city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, None));
                    num_city_states_discarded -= 1;
                } else {
                    break;
                }
            }
        }

        if num_city_states_discarded > 0 {
            panic!(
                "Could not place {} city states on map. Too many city states for map size.",
                num_city_states_discarded
            );
        }
    }

    /// Places a city state on the map.
    ///
    /// This function will do as follows:
    /// 1. Add the city state tile to the `city_state_and_starting_tile` map.
    /// 2. Clear the ice feature from the coast tiles adjacent to the city state.
    /// 3. Place resource impacts and ripple on the city state tile and its around tiles.
    fn place_city_state(&mut self, city_state: &str, tile: Tile) {
        self.starting_tile_and_city_state
            .insert(tile, city_state.to_string());
        // Removes Feature Ice from coasts adjacent to the city state's new location
        self.clear_ice_near_city_site(tile, 1);

        self.place_impact_and_ripples(tile, Layer::CityState, u32::MAX);

        self.player_collision_data[tile.index()] = true;
    }

    // function AssignStartingPlots:PlaceCityStateInRegion(city_state_number, region_number)
    /// Get the starting tile for a city state in a region.
    fn get_city_state_start_tile_in_region(&mut self, region_index: usize) -> Option<Tile> {
        let candidate_tile_list = self.get_candidate_city_state_tiles_in_region(region_index);

        self.get_city_state_start_tile(&candidate_tile_list, false, false)
    }

    // function AssignStartingPlots:ObtainNextSectionInRegion
    /// Get all the candidate tiles can be used for placing a city state in a region.
    ///
    /// We get all the candidate tiles in the region follow the following steps:
    /// 1. Divide the region into 3 parts: one center part and two edge parts.
    /// 2. Check if the center part is enough small:
    ///     - If it is, we will process all the tiles in the region to get the candidate tiles.
    ///     - If it is not, we will process the edge parts to get the candidate tiles. That is because we often use the center rectangle to place civilizations.
    ///
    /// # Returns
    ///
    /// Returns an array of two vectors of tiles.
    /// The first vector is the coastal tiles, and the second vector is the inland tiles.
    pub fn get_candidate_city_state_tiles_in_region(&self, region_index: usize) -> [Vec<Tile>; 2] {
        let grid = self.world_grid.grid;

        let region = &self.region_list[region_index];
        let rectangle = &region.rectangle;

        // Check if the rectangle is small enough to process all the tiles. If it is, we will process all the tiles.
        let should_process_all_tiles = rectangle.width() < 4 || rectangle.height() < 4;
        let taller = rectangle.height() > rectangle.width();

        // Divide the rectangle into 3 parts according to whether it is taller or not.
        // If it is taller, we will divide it vertically, and if it is not, we will divide it horizontally.
        // The center will be 2/3 of the rectangle, and the other two parts will be 1/6 each.
        const CENTER_BIAS: f64 = 2.0 / 3.0;

        let (center_west_x, center_south_y, center_width, center_height);

        if taller {
            let non_center_height =
                ((1. - CENTER_BIAS) / 2.0 * rectangle.height() as f64).floor() as u32;

            center_west_x = rectangle.west_x();
            center_south_y = rectangle.south_y() + non_center_height as i32;
            center_width = rectangle.width();
            center_height = rectangle.height() - (non_center_height * 2);
        } else {
            let non_center_width =
                ((1. - CENTER_BIAS) / 2.0 * rectangle.width() as f64).floor() as u32;

            center_west_x = rectangle.west_x() + non_center_width as i32;
            center_south_y = rectangle.south_y();
            center_width = rectangle.width() - (non_center_width * 2);
            center_height = rectangle.height();
        }

        let center_rectangle = Rectangle::new(
            OffsetCoordinate::new(center_west_x, center_south_y),
            center_width,
            center_height,
            grid,
        );

        let mut coastal_tile_list = Vec::new();
        let mut inland_tile_list = Vec::new();

        for tile in rectangle.all_tiles(grid) {
            if should_process_all_tiles {
                // When the rectangle is small enough, we will process all the tiles.
                if tile.can_be_city_state_starting_tile(self, Some(region)) {
                    if tile.is_coastal_land(self) {
                        coastal_tile_list.push(tile);
                    } else {
                        inland_tile_list.push(tile);
                    }
                }
            } else {
                // Process only tiles near enough to the region edge.
                // That means tiles that are not in the center rectangle.
                // That is because we often use the center rectangle to place civilizations.
                if !center_rectangle.contains(tile, grid)
                    && tile.can_be_city_state_starting_tile(self, Some(region))
                {
                    if tile.is_coastal_land(self) {
                        coastal_tile_list.push(tile);
                    } else {
                        inland_tile_list.push(tile);
                    }
                }
            }
        }

        [coastal_tile_list, inland_tile_list]
    }

    // function AssignStartingPlots:PlaceCityState
    /// Randomly selects a tile to place a city-state from a list of candidate tiles.
    ///
    /// Coastal tiles are prioritized; if no coastal tiles are available, inland tiles are considered.
    ///
    /// # Arguments
    ///
    /// - `candidate_tile_list`: A list of candidate tiles.  
    ///   Typically, this is an array of two `Vec`s. The first `Vec` contains coastal tiles, and the second contains inland tiles.  
    ///   The selection is made first from the coastal tiles (`Vec`), and if unsuccessful, the selection proceeds with the inland tiles (`Vec`).
    /// - `check_proximity`: A flag indicating whether to check the proximity to other city-states.  
    ///   If `check_proximity` is `true`, the tile is chosen from those that are not too close to other city-states.
    /// - `check_collision`: A flag indicating whether to check for collision with other city-states.  
    ///   If `check_collision` is `true`, the tile is chosen from those that are not occupied by other city-states.
    ///
    /// # Returns
    ///
    /// If a suitable tile is found, the function returns the tile. Otherwise, it returns `None`.
    fn get_city_state_start_tile(
        &mut self,
        candidate_tile_list: &[Vec<Tile>],
        check_proximity: bool,
        check_collision: bool,
    ) -> Option<Tile> {
        let mut chosen_tile = None;
        // We choose tile according in the order of the candidate tile list.
        for candidate_list in candidate_tile_list {
            if !candidate_list.is_empty() {
                let mut candidate_list = candidate_list.to_vec();
                if check_collision {
                    // Place city state, avoiding collision
                    candidate_list.shuffle(&mut self.random_number_generator);
                    for tile in candidate_list {
                        if !self.player_collision_data[tile.index()]
                            && (!check_proximity
                                || self.layer_data[Layer::CityState][tile.index()] == 0)
                        {
                            chosen_tile = Some(tile);
                            break;
                        }
                    }
                } else {
                    chosen_tile = candidate_list
                        .choose(&mut self.random_number_generator)
                        .cloned();
                }
            }
        }
        chosen_tile
    }

    // function AssignStartingPlots:AssignCityStatesToRegionsOrToUninhabited
    /// Assigns city states to regions or uninhabited landmass.
    ///
    /// This function will do as follows:
    /// 1. Assign n city states to Per Region;
    /// 2. Assign city states to uninhabited landmasses;
    /// 3. Assign city states to regions with shared luxury resources.
    ///    These city states are compensated for multiple regions assigned the same luxury resource type.\
    ///    It only compensates when one luxury resource type is assigned to 3 different regions.
    ///    3 is the maximum number of regions that can share the same luxury resource type,
    ///    This parameter is defined by the const `MAX_REGIONS_PER_LUXURY_TYPE` variable in [`TileMap::assign_luxury_to_region`].
    ///    View [`TileMap::assign_luxury_to_region`] for more information.
    /// 4. Assign city states to low fertility regions.
    ///
    /// # Returns
    ///
    /// Returns a [`CityStatesAssignment`], view its documentation for more information.
    fn assign_city_states_to_regions_or_uninhabited_landmasses(
        &mut self,
        map_parameters: &MapParameters,
    ) -> CityStatesAssignment {
        let mut num_city_states_unassigned = map_parameters.city_state_num;

        // Store region index which city state is assigned to
        let mut region_index_assignment =
            Vec::with_capacity(map_parameters.city_state_num as usize);

        let mut uninhabited_areas_coastal_land_tiles = Vec::new();
        let mut uninhabited_areas_inland_tiles = Vec::new();

        /***** Assign the "Per Region" City States to their regions ******/
        let ratio = map_parameters.city_state_num as f64 / map_parameters.civilization_num as f64;
        let num_city_states_per_region = match ratio {
            r if r > 14.0 => 10,
            r if r > 11.0 => 8,
            r if r > 8.0 => 6,
            r if r > 5.7 => 4,
            r if r > 4.35 => 3,
            r if r > 2.7 => 2,
            r if r > 1.35 => 1,
            _ => 0,
        };

        // if num_city_states_per_region is 0, the code below will not be executed.
        for _ in 0..num_city_states_per_region {
            for region_index in 0..self.region_list.len() {
                region_index_assignment.push(Some(region_index));
            }
        }

        num_city_states_unassigned -= region_index_assignment.len() as u32;
        /***** Assign the "Per Region" City States to their regions ******/

        /***** Assign city states to uninhabited landmasses ******/
        // Number of City States to be placed on landmasses uninhabited by civs
        let _num_city_states_uninhabited;

        let mut land_area_id_and_tiles: HashMap<usize, Vec<_>> = HashMap::new();

        let mut num_civ_landmass_tiles = 0;
        let mut num_uninhabited_landmass_tiles = 0;

        if let RegionDivideMethod::WholeMapRectangle = map_parameters.region_divide_method {
            // Rectangular regional division spanning the entire globe, ALL plots belong to inhabited regions,
            // so all city states must belong to a region!
            _num_city_states_uninhabited = 0;
        } else {
            // Possibility of plots that do not belong to any civ's Region. Evaluate these plots and assign an appropriate number of City States to them.
            self.all_tiles().for_each(|tile| {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                if matches!(terrain_type, TerrainType::Flatland | TerrainType::Hill)
                    && base_terrain != BaseTerrain::Snow
                {
                    if let RegionDivideMethod::CustomRectangle(rectangle) =
                        map_parameters.region_divide_method
                    {
                        if rectangle.contains(tile, self.world_grid.grid) {
                            num_civ_landmass_tiles += 1;
                        } else {
                            num_uninhabited_landmass_tiles += 1;
                            if tile.is_coastal_land(self) {
                                uninhabited_areas_coastal_land_tiles.push(tile)
                            } else {
                                uninhabited_areas_inland_tiles.push(tile)
                            }
                        }
                    } else {
                        let area_id = tile.area_id(self);
                        land_area_id_and_tiles
                            .entry(area_id)
                            .or_default()
                            .push(tile);
                    }
                }
            });

            // Complete the AreaID-based method.
            if matches!(
                map_parameters.region_divide_method,
                RegionDivideMethod::Pangaea | RegionDivideMethod::Continent
            ) {
                // Generate list of inhabited area ID.
                let areas_inhabited_by_civs: HashSet<_> = self
                    .region_list
                    .iter()
                    .filter_map(|region| region.area_id)
                    .collect();

                for (land_area_id, tiles) in land_area_id_and_tiles.iter() {
                    if areas_inhabited_by_civs.contains(land_area_id) {
                        num_civ_landmass_tiles += tiles.len();
                    } else {
                        num_uninhabited_landmass_tiles += tiles.len();
                        // We should make sure that the uninhabited landmass is enough large to place a city state.
                        if tiles.len() >= 4 {
                            tiles.iter().for_each(|&tile| {
                                // It have checked in the code above. So we don't need to check it again.
                                /* debug_assert!(
                                    matches!(
                                        tile.terrain_type(self),
                                        TerrainType::Flatland | TerrainType::Hill
                                    ) && tile.base_terrain(self) != BaseTerrain::Snow
                                ); */
                                if tile.is_coastal_land(self) {
                                    uninhabited_areas_coastal_land_tiles.push(tile);
                                } else {
                                    uninhabited_areas_inland_tiles.push(tile);
                                }
                            });
                        }
                    }
                }
            }

            let uninhabited_ratio = num_uninhabited_landmass_tiles as f64
                / (num_civ_landmass_tiles + num_uninhabited_landmass_tiles) as f64;
            let max_by_ratio =
                (3. * uninhabited_ratio * map_parameters.city_state_num as f64) as u32;
            let max_by_method =
                if let RegionDivideMethod::Pangaea = map_parameters.region_divide_method {
                    (map_parameters.city_state_num as f64 / 4.).ceil()
                } else {
                    (map_parameters.city_state_num as f64 / 2.).ceil()
                } as u32;

            _num_city_states_uninhabited =
                min(num_city_states_unassigned, min(max_by_ratio, max_by_method));

            region_index_assignment.extend(vec![None; _num_city_states_uninhabited as usize]);
            num_city_states_unassigned -= _num_city_states_uninhabited;
        }
        /***** Assign city states to uninhabited landmasses ******/

        /***** Assign city states to regions with shared luxury resources ******/
        let mut num_city_states_shared_luxury = 0;
        let num_city_states_low_fertility;

        if num_city_states_unassigned > 0 {
            let mut num_regions_shared_luxury = 0;
            let mut shared_luxury = Vec::new();
            // Determine how many to place in support of regions that share their luxury type with two other regions.
            for (luxury_resource, &luxury_assign_to_region_count) in
                self.luxury_assign_to_region_count.iter()
            {
                if luxury_assign_to_region_count == 3 {
                    num_regions_shared_luxury += 3;
                    shared_luxury.push(luxury_resource);
                }
            }

            if num_regions_shared_luxury > 0
                && num_regions_shared_luxury <= num_city_states_unassigned
            {
                num_city_states_shared_luxury = num_regions_shared_luxury;
                num_city_states_low_fertility =
                    num_city_states_unassigned - num_city_states_shared_luxury;
            } else {
                num_city_states_low_fertility = num_city_states_unassigned;
            }

            if num_city_states_shared_luxury > 0 {
                for luxury_resource in shared_luxury.iter() {
                    for (region_index, region) in self.region_list.iter().enumerate() {
                        if &&region.exclusive_luxury == luxury_resource {
                            region_index_assignment.push(Some(region_index));
                            num_city_states_unassigned -= 1;
                        }
                    }
                }
            }
            /***** Assign city states to regions with shared luxury resources ******/

            /***** Assign city states to regions with low fertility ******/
            if num_city_states_low_fertility > 0 {
                // If more to assign than number of regions, assign per region.
                let num_regions = self.region_list.len() as u32;
                let num_assignments_per_region = num_city_states_unassigned / num_regions;
                num_city_states_unassigned %= num_regions;

                for _ in 0..num_assignments_per_region {
                    for region_index in 0..self.region_list.len() {
                        region_index_assignment.push(Some(region_index));
                    }
                }
            }

            if num_city_states_unassigned > 0 {
                let mut region_index_and_fertility_per_land_tile = Vec::new();
                for (region_index, region) in self.region_list.iter().enumerate() {
                    let land_tile_count = region.terrain_statistic.terrain_type_num
                        [TerrainType::Flatland]
                        + region.terrain_statistic.terrain_type_num[TerrainType::Hill];
                    let region_fertility = region.fertility_sum;
                    let fertility_per_land_tile = region_fertility / land_tile_count as i32;
                    region_index_and_fertility_per_land_tile
                        .push((region_index, fertility_per_land_tile));
                }
                region_index_and_fertility_per_land_tile
                    .sort_by_key(|(_, fertility_per_land_tile)| *fertility_per_land_tile);

                for (region_index, _) in region_index_and_fertility_per_land_tile
                    .iter()
                    .take(num_city_states_unassigned as usize)
                {
                    region_index_assignment.push(Some(*region_index));
                }
            }
        }
        /***** Assign city states to regions with low fertility ******/

        CityStatesAssignment {
            region_index_assignment,
            uninhabited_areas_coastal_land_tiles,
            uninhabited_areas_inland_tiles,
        }
    }

    /// Normalizes each city state locations.
    pub fn normalize_city_state_locations(&mut self) {
        let starting_tiles: Vec<_> = self.starting_tile_and_city_state.keys().cloned().collect();
        for starting_tile in starting_tiles {
            self.normalize_city_state(starting_tile);
        }
    }

    // function AssignStartingPlots:NormalizeCityState
    /// Normalizes city state location.
    ///
    /// This function will do as follows:
    /// 1. Add hills to city state location's 1 radius if it has not enough hammer.
    /// 2. Add bonus resource for compensation to city state location's 1-2 radius if it has not enough food.
    ///
    /// # Notice
    ///
    /// We don't place impact and ripples when we add bonus resources in this function.
    fn normalize_city_state(&mut self, tile: Tile) {
        let grid = self.world_grid.grid;

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
        let mut neighbor_tile_list: Vec<Tile> = tile.neighbor_tiles(grid).collect();

        neighbor_tile_list.iter().for_each(|neighbor_tile| {
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
                    } else if tile.is_freshwater(self) {
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
        let mut tile_at_distance_two_list: Vec<Tile> = tile.tiles_at_distance(2, grid).collect();

        tile_at_distance_two_list
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
                        } else if tile_at_distance_two.is_freshwater(self) {
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
        let mut _hammer_score = (4 * inner_hills) + (2 * inner_forest) + inner_one_hammer;
        if _hammer_score < 4 {
            neighbor_tile_list.shuffle(&mut self.random_number_generator);
            for &tile in neighbor_tile_list.iter() {
                // Attempt to place a Hill at the currently chosen tile.
                let placed_hill = self.attempt_to_place_hill_at_tile(tile);
                if placed_hill {
                    _hammer_score += 4;
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
            let _max_bonuses_possible = inner_can_have_bonus + outer_can_have_bonus;
            // The num of food bonus we have placed in the first ring.
            let mut inner_placed = 0;
            // The num of food bonus we have placed in the second ring.
            let mut outer_placed = 0;
            // Permanent flag. (We don't want to place more than one Oasis per location).
            // This is set to false after the first Oasis is placed.
            let mut allow_oasis = true;

            // We shuffle the `neighbor_tiles` that was used earlier, instead of recreating a new one.
            neighbor_tile_list.shuffle(&mut self.random_number_generator);

            // We shuffle the `tiles_at_distance_two` that was used earlier, instead of recreating a new one.
            tile_at_distance_two_list.shuffle(&mut self.random_number_generator);

            /* let mut first_ring_iter = neighbor_tile_list.iter().peekable();
            let mut second_ring_iter = tile_at_distance_two_list.iter().peekable();

            while num_food_bonus_needed > 0 {
                if inner_placed < 2 && inner_can_have_bonus > 0 && first_ring_iter.peek().is_some()
                {
                    // Add bonus to inner ring.
                    while let Some(&tile) = first_ring_iter.next() {
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
                } else if (inner_placed + outer_placed < 4 && outer_can_have_bonus > 0)
                    && second_ring_iter.peek().is_some()
                {
                    // Add bonus to second ring.
                    while let Some(&tile) = second_ring_iter.next() {
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
                } else {
                    break;
                }
            } */

            // The following code is equivalent to the commented code above, but it is faster.
            // Process inner ring
            if num_food_bonus_needed > 0 && inner_placed < 2 && inner_can_have_bonus > 0 {
                for tile in neighbor_tile_list.into_iter() {
                    let (placed_bonus, placed_oasis) =
                        self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);

                    if placed_bonus {
                        if allow_oasis && placed_oasis {
                            allow_oasis = false;
                        }
                        inner_placed += 1;
                        inner_can_have_bonus -= 1;
                        num_food_bonus_needed -= 1;

                        if num_food_bonus_needed == 0
                            || inner_placed >= 2
                            || inner_can_have_bonus == 0
                        {
                            break;
                        }
                    }
                }
            }

            // Process outer ring if still needed
            if num_food_bonus_needed > 0
                && (inner_placed + outer_placed) < 4
                && outer_can_have_bonus > 0
            {
                for tile in tile_at_distance_two_list.into_iter() {
                    let (placed_bonus, placed_oasis) =
                        self.attempt_to_place_bonus_resource_at_tile(tile, allow_oasis);

                    if placed_bonus {
                        if allow_oasis && placed_oasis {
                            allow_oasis = false;
                        }
                        outer_placed += 1;
                        outer_can_have_bonus -= 1;
                        num_food_bonus_needed -= 1;

                        if num_food_bonus_needed == 0
                            || (inner_placed + outer_placed) >= 4
                            || outer_can_have_bonus == 0
                        {
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Represents the assignment of city states to regions and uninhabited landmasses.
///
/// This structure tracks where city states should be placed, either within
/// regions or on uninhabited landmasses (both coastal and inland).
#[derive(Debug)]
struct CityStatesAssignment {
    /// Region indices assigned to each city state will be placed in.
    ///
    /// - Length equals the number of city states to place
    /// - `Some(index)` indicates assignment to a region
    /// - `None` indicates assignment to an uninhabited landmass
    region_index_assignment: Vec<Option<usize>>,
    /// Available coastal tiles not belonging to any region.
    ///
    /// These tiles are candidates for placing city states in uninhabited
    /// coastal areas. The tiles should be valid for city state placement.
    uninhabited_areas_coastal_land_tiles: Vec<Tile>,
    /// Available inland tiles not belonging to any region.
    ///
    /// These tiles are candidates for placing city states in uninhabited
    /// inland areas. The tiles should be valid for city state placement.
    uninhabited_areas_inland_tiles: Vec<Tile>,
}
