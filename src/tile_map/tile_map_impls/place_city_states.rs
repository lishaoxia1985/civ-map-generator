use std::cmp::min;

#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use rand::{seq::SliceRandom, Rng};

use crate::{
    component::{base_terrain::BaseTerrain, terrain_type::TerrainType},
    ruleset::Ruleset,
    tile_map::{
        tile::Tile, tile_map_impls::generate_regions::Rectangle, Layer, MapParameters,
        RegionDivideMethod, TileMap,
    },
};

impl TileMap {
    // function AssignStartingPlots:PlaceCityStates
    /// Place city states on the map.
    ///
    /// This function depends on [`TileMap::assign_luxury_roles`] being executed first.
    /// This is because some city state placements are made as compensation for situations where
    /// multiple regions are assigned the same luxury resource type.
    pub fn place_city_states(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        self.assign_city_states_to_regions_or_uninhabited_landmasses(map_parameters);

        let mut city_state_list = ruleset
            .nations
            .iter()
            .filter(|(_, nation)| nation.city_state_type != "")
            .map(|(city_state, _)| city_state)
            .collect::<Vec<_>>();

        // We get the civilization in the order.
        // That make sure we get the same civilization list every time we run the game.
        city_state_list.sort();

        city_state_list.shuffle(&mut self.random_number_generator);

        let mut start_city_state_list: Vec<_> = city_state_list
            .into_iter()
            .take(map_parameters.city_state_num as usize)
            .collect();

        let mut num_uninhabited_candidate_tiles =
            self.uninhabited_areas_coastal_tiles.len() + self.uninhabited_areas_inland_tiles.len();

        let uninhabited_areas_coastal_tile_list = self.uninhabited_areas_coastal_tiles.clone();
        let uninhabited_areas_inland_tile_list = self.uninhabited_areas_inland_tiles.clone();

        let mut num_city_states_discarded = 0;

        for index in 0..self.city_state_region_assignments.len() {
            let region_index = self.city_state_region_assignments[index];
            if region_index.is_none() && num_uninhabited_candidate_tiles > 0 {
                num_uninhabited_candidate_tiles -= 1;
                let tile = self.get_city_state_start_tile(
                    &uninhabited_areas_coastal_tile_list,
                    &uninhabited_areas_inland_tile_list,
                    true,
                    true,
                );
                // Place city state on uninhabited land
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(map_parameters, city_state, tile);
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
                let tile = self.get_city_state_start_tile_in_region(map_parameters, region_index);
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(map_parameters, city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, Some(region_index)));
                } else {
                    num_city_states_discarded += 1;
                }
            } else {
                // Assigned to a Region.
                let region_index = region_index.unwrap();
                let tile = self.get_city_state_start_tile_in_region(map_parameters, region_index);
                if let Some(tile) = tile {
                    let city_state = start_city_state_list.pop().unwrap();
                    self.place_city_state(map_parameters, city_state, tile);
                    self.city_state_starting_tile_and_region_index
                        .push((tile, Some(region_index)));
                } else {
                    num_city_states_discarded += 1;
                }
            }
        }

        // Last chance method to place city states that didn't fit where they were supposed to go.
        if num_city_states_discarded > 0 {
            let mut cs_last_chance_plot_list = self
                .iter_tiles()
                .filter(|tile| tile.can_be_city_state_starting_tile(self, None, false, false))
                .collect::<Vec<Tile>>();

            if cs_last_chance_plot_list.len() > 0 {
                cs_last_chance_plot_list.shuffle(&mut self.random_number_generator);

                for city_state in start_city_state_list.iter() {
                    let tile = self.get_city_state_start_tile(
                        &cs_last_chance_plot_list,
                        &vec![],
                        true,
                        true,
                    );
                    if let Some(tile) = tile {
                        self.place_city_state(map_parameters, city_state, tile);
                        self.city_state_starting_tile_and_region_index
                            .push((tile, None));
                        num_city_states_discarded -= 1;
                    } else {
                        break;
                    }
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

    fn place_city_state(&mut self, map_parameters: &MapParameters, city_state: &str, tile: Tile) {
        self.city_state_and_starting_tile
            .insert(city_state.to_string(), tile);
        // Removes Feature Ice from coasts adjacent to the city state's new location
        self.generate_luxury_plot_lists_at_city_site(map_parameters, tile, 1, true);

        self.place_resource_impact(map_parameters, tile, Layer::CityState, 4);
        self.place_resource_impact(map_parameters, tile, Layer::Luxury, 3);
        // Strategic layer, should be at start point only.
        self.place_resource_impact(map_parameters, tile, Layer::Strategic, 0);
        self.place_resource_impact(map_parameters, tile, Layer::Bonus, 3);
        self.place_resource_impact(map_parameters, tile, Layer::Fish, 3);
        self.place_resource_impact(map_parameters, tile, Layer::Marble, 3);
        self.player_collision_data[tile.index()] = true;
    }

    // function AssignStartingPlots:PlaceCityStateInRegion(city_state_number, region_number)
    fn get_city_state_start_tile_in_region(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> Option<Tile> {
        let (eligible_coastal, eligible_inland) =
            self.obtain_next_section_in_region(map_parameters, region_index, false, false);

        let tile =
            self.get_city_state_start_tile(&eligible_coastal, &eligible_inland, false, false);

        tile
    }

    // function AssignStartingPlots:ObtainNextSectionInRegion
    pub fn obtain_next_section_in_region(
        &self,
        map_parameters: &MapParameters,
        region_index: usize,
        force_it: bool,
        ignore_collisions: bool,
    ) -> (Vec<Tile>, Vec<Tile>) {
        let region = &self.region_list[region_index];
        let rectangle = &region.rectangle;

        let reached_middle = rectangle.width < 4 || rectangle.height < 4;
        let taller = rectangle.height > rectangle.width;

        // Divide the rectangle into 3 parts according to whether it is taller or not.
        // If it is taller, we will divide it vertically, and if it is not, we will divide it horizontally.
        // The center will be 2/3 of the rectangle, and the other two parts will be 1/6 each.
        const CENTER_BIAS: f64 = 2.0 / 3.0;

        let (center_west_x, center_south_y, center_width, center_height);

        if taller {
            let non_center_height =
                ((1. - CENTER_BIAS) / 2.0 * rectangle.height as f64).floor() as i32;

            center_west_x = rectangle.west_x;
            center_south_y =
                (rectangle.south_y + non_center_height) % map_parameters.map_size.height;
            center_width = rectangle.width;
            center_height = rectangle.height - (non_center_height * 2);
        } else {
            let non_center_width =
                ((1. - CENTER_BIAS) / 2.0 * rectangle.width as f64).floor() as i32;

            center_west_x = (rectangle.west_x + non_center_width) % map_parameters.map_size.width;
            center_south_y = rectangle.south_y;
            center_width = rectangle.width - (non_center_width * 2);
            center_height = rectangle.height;
        }

        let center_rectangle = Rectangle {
            west_x: center_west_x,
            south_y: center_south_y,
            width: center_width,
            height: center_height,
        };

        let mut coastal_plot_list = Vec::new();
        let mut inland_plot_list = Vec::new();

        for tile in rectangle.iter_tiles(map_parameters) {
            if reached_middle {
                if tile.can_be_city_state_starting_tile(
                    self,
                    Some(region),
                    force_it,
                    ignore_collisions,
                ) {
                    if tile.is_coastal_land(self, map_parameters) {
                        coastal_plot_list.push(tile);
                    } else {
                        inland_plot_list.push(tile);
                    }
                }
            } else {
                // Process only plots near enough to the region edge.
                // That means plots that are not in the center rectangle.
                if !center_rectangle.contains(map_parameters, tile) {
                    if tile.can_be_city_state_starting_tile(
                        self,
                        Some(region),
                        force_it,
                        ignore_collisions,
                    ) {
                        if tile.is_coastal_land(self, map_parameters) {
                            coastal_plot_list.push(tile);
                        } else {
                            inland_plot_list.push(tile);
                        }
                    }
                }
            }
        }

        (coastal_plot_list, inland_plot_list)
    }

    // function AssignStartingPlots:PlaceCityState
    /// Get a tile for a city state from a list of candidate tiles.
    ///
    /// Coastal plots are prioritized, but if there are no coastal plots, then inland plots are used.
    /// The tile is chosen randomly from the list.
    /// If `check_proximity` is true, then the tile is chosen from the list of tiles that are
    /// not too close to other city states.
    /// If `check_collision` is true, then the tile is chosen from the list of tiles that are
    /// not occupied by other city states.
    fn get_city_state_start_tile(
        &mut self,
        coastal_plot_list: &[Tile],
        inland_plot_list: &[Tile],
        check_proximity: bool,
        check_collision: bool,
    ) -> Option<Tile> {
        let mut chosen_tile = None;
        // `coastal_plot_list` is prioritized, but if it is empty, then use `inland_plot_list`
        let candidate_tile_list = vec![coastal_plot_list, inland_plot_list];
        for candidate_list in candidate_tile_list {
            if candidate_list.len() > 0 {
                let mut candidate_list = candidate_list.to_vec();
                if check_collision {
                    // Place city state, avoiding collision
                    candidate_list.shuffle(&mut self.random_number_generator);
                    for tile in candidate_list {
                        if self.player_collision_data[tile.index()] == false {
                            if !check_proximity
                                || self.layer_data[&Layer::CityState][tile.index()] == 0
                            {
                                chosen_tile = Some(tile);
                                break;
                            }
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
    pub fn assign_city_states_to_regions_or_uninhabited_landmasses(
        &mut self,
        map_parameters: &MapParameters,
    ) {
        let mut num_city_states_unassigned = map_parameters.city_state_num;

        // Store region index which city state is assigned to
        let mut city_state_region_assignments =
            Vec::with_capacity(map_parameters.city_state_num as usize);

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
                city_state_region_assignments.push(Some(region_index));
            }
        }

        num_city_states_unassigned -= city_state_region_assignments.len() as u32;
        /***** Assign the "Per Region" City States to their regions ******/

        /***** Assign city states to uninhabited landmasses ******/
        // Number of City States to be placed on landmasses uninhabited by civs
        let num_city_states_uninhabited;

        let mut land_area_id_and_tiles: HashMap<i32, Vec<_>> = HashMap::new();

        let mut num_civ_landmass_tiles = 0;
        let mut num_uninhabited_landmass_tiles = 0;

        if let RegionDivideMethod::WholeMapRectangle = map_parameters.region_divide_method {
            // Rectangular regional division spanning the entire globe, ALL plots belong to inhabited regions,
            // so all city states must belong to a region!
            num_city_states_uninhabited = 0;
        } else {
            // Possibility of plots that do not belong to any civ's Region. Evaluate these plots and assign an appropriate number of City States to them.
            self.iter_tiles().for_each(|tile| {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                if matches!(terrain_type, TerrainType::Flatland | TerrainType::Hill)
                    && base_terrain != BaseTerrain::Snow
                {
                    if let RegionDivideMethod::CustomRectangle(rectangle) =
                        map_parameters.region_divide_method
                    {
                        if rectangle.contains(map_parameters, tile) {
                            num_civ_landmass_tiles += 1;
                        } else {
                            num_uninhabited_landmass_tiles += 1;
                            if tile.is_coastal_land(self, map_parameters) {
                                self.uninhabited_areas_coastal_tiles.push(tile)
                            } else {
                                self.uninhabited_areas_inland_tiles.push(tile)
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
                // Generate list of inhabited area id.
                let areas_inhabited_by_civs: HashSet<_> = self
                    .region_list
                    .iter()
                    .filter_map(|region| region.landmass_id)
                    .collect();

                for (land_area_id, tiles) in land_area_id_and_tiles.iter() {
                    if areas_inhabited_by_civs.contains(land_area_id) {
                        num_civ_landmass_tiles += tiles.len();
                    } else {
                        num_uninhabited_landmass_tiles += tiles.len();
                        // We should make sure that the uninhabited landmass is enough large to place a city state.
                        if tiles.len() >= 4 {
                            for tile in tiles {
                                // It have checked in the code above. So we don't need to check it again.
                                /* debug_assert!(
                                    matches!(
                                        tile.terrain_type(self),
                                        TerrainType::Flatland | TerrainType::Hill
                                    ) && tile.base_terrain(self) != BaseTerrain::Snow
                                ); */
                                if tile.is_coastal_land(self, map_parameters) {
                                    self.uninhabited_areas_coastal_tiles.push(*tile);
                                } else {
                                    self.uninhabited_areas_inland_tiles.push(*tile);
                                }
                            }
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

            num_city_states_uninhabited =
                min(num_city_states_unassigned, min(max_by_ratio, max_by_method));

            city_state_region_assignments.extend(vec![None; num_city_states_uninhabited as usize]);
            num_city_states_unassigned -= num_city_states_uninhabited;
        }
        /***** Assign city states to uninhabited landmasses ******/

        /***** Assign city states to regions with shared luxury resources ******/
        let mut num_city_states_shared_luxury = 0;
        let num_city_states_low_fertility;

        if num_city_states_unassigned > 0 {
            let mut num_regions_shared_luxury = 0;
            let mut shared_luxury = Vec::new();
            // Determine how many to place in support of regions that share their luxury type with two other regions.
            for (luxury_resource, &luxury_assignment_count) in
                self.luxury_assign_to_region_count.iter()
            {
                if luxury_assignment_count == 3 {
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
                        if &&region.luxury_resource == luxury_resource {
                            city_state_region_assignments.push(Some(region_index));
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
                num_city_states_unassigned = num_city_states_unassigned % num_regions;

                for _ in 0..num_assignments_per_region {
                    for region_index in 0..self.region_list.len() {
                        city_state_region_assignments.push(Some(region_index));
                    }
                }
            }

            if num_city_states_unassigned > 0 {
                let mut region_index_and_fertility_per_land_tile = Vec::new();
                for (region_index, region) in self.region_list.iter().enumerate() {
                    let land_tile_count = region.terrain_statistic.terrain_type_sum
                        [&TerrainType::Flatland]
                        + region.terrain_statistic.terrain_type_sum[&TerrainType::Hill];
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
                    city_state_region_assignments.push(Some(*region_index));
                }
            }
        }
        /***** Assign city states to regions with low fertility ******/

        self.city_state_region_assignments = city_state_region_assignments;
    }
}
