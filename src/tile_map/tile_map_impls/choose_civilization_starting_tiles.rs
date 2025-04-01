use std::{
    cmp::{max, min},
    collections::HashMap,
};

use enum_map::{Enum, EnumMap};

use crate::{
    base_terrain::BaseTerrain,
    feature::Feature,
    hex::{HexOrientation, Offset},
    terrain_type::TerrainType,
    tile_map::{
        tile::Tile, tile_map_impls::generate_regions::Rectangle, Layer, MapParameters, RegionType,
        TileMap,
    },
    OffsetCoordinate,
};

use super::generate_regions::Region;

impl TileMap {
    // function AssignStartingPlots:ChooseLocations
    /// Get starting tile for each civilization according to region. Every region will have a starting tile for a civilization.
    pub fn choose_civilization_starting_tiles(&mut self, map_parameters: &MapParameters) {
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
    /// Find a starting tile for a region without regard to [Region::landmass_id].
    ///
    /// # Returns
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    /// and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
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
    /// Find a starting tile which is coastal land for a region:
    /// - If the number of coastal land tiles in the region is less than 3, choose inland tile as starting tile (use [`TileMap::find_start`]).
    /// - If the number of coastal land tiles in the region is greater than or equal to 3, choose coastal land tiles as starting tile.
    /// - If there is no eligible starting tile, force a starting tile to be placed.
    ///
    /// # Returns
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    /// and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
    fn find_coastal_land_start(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> (bool, bool) {
        let mut fallback_tile_and_score = Vec::new();

        let coastal_land_sum = (&self.region_list[region_index])
            .terrain_statistic
            .coastal_land_num;

        let success_flag; // Returns true when a start is placed, false when process fails.
        let mut forced_placement_flag; // Returns true if this region had no eligible starts and one was forced to occur.

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
            let mut best_fallback_score = -50;
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
    /// Find a starting tile for a region.
    ///
    /// # Returns
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    /// and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
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
            let mut best_fallback_score = -50;
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
    /// Iterates through a list of candidate tiles and returns the best tile and fallback tile.
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
    /// # Returns
    /// This function returns a tuple:
    /// - first element. The score of the tile.
    /// - second element. A boolean indicating whether the tile meets the minimum requirements. If it does not meet the minimum requirements, it will be used as a fallback tile.
    /// If the tile meets the minimum requirements, it is `true`, otherwise `false`.
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
            let measure_tile_type = self.measure_single_tile(neighbor_tile, region);
            measure_tile_type
                .into_iter()
                .for_each(|(tile_type, measure)| {
                    if measure {
                        match tile_type {
                            TileType::Food => food_total += 1,
                            TileType::Production => production_total += 1,
                            TileType::Good => good_total += 1,
                            TileType::Junk => junk_total += 1,
                        }
                    }
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
                let measure_tile_type = self.measure_single_tile(tile_at_distance_two, region);
                measure_tile_type
                    .into_iter()
                    .for_each(|(tile_type, measure)| {
                        if measure {
                            match tile_type {
                                TileType::Food => food_total += 1,
                                TileType::Production => production_total += 1,
                                TileType::Good => good_total += 1,
                                TileType::Junk => junk_total += 1,
                            }
                        }
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
                let measure_tile_type = self.measure_single_tile(tile_at_distance_three, region);
                measure_tile_type
                    .into_iter()
                    .for_each(|(tile_type, measure)| {
                        if measure {
                            match tile_type {
                                TileType::Food => food_total += 1,
                                TileType::Production => production_total += 1,
                                TileType::Good => good_total += 1,
                                TileType::Junk => junk_total += 1,
                            }
                        }
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
            // Closer to existing start points, lower the score becomes.
            meets_minimum_requirements = false;
            final_score = (final_score as f64 * (100 - self.distance_data[tile.index()]) as f64
                / 100.0) as i32;
        }
        (final_score, meets_minimum_requirements)
    }

    // function AssignStartingPlots:PlaceImpactAndRipples
    /// Places the impact and ripple values for a starting tile of civilization.
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

        self.layer_data[Layer::CityState][tile.index()] = 1;

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
                        self.layer_data[Layer::CityState][tile_at_distance.index()] = 1;
                    }
                })
        }
    }

    // function AssignStartingPlots:MeasureSinglePlot
    /// Measures single tile's type.
    ///
    /// Measures a single tile's type whether it is Food, Production, Good, Junk, or a combination of (Food, Production, Good).
    /// - [`TileType::Food`] may be misleading, as this is the primary mechanism for biasing starting terrain.
    /// It is not strictly equivalent to tile yield. That's because regions with different [`RegionType`] obtain food in various ways.
    // Tundra, Jungle, Forest, Desert, and Plains regions will receive bonus resource support to compensate for food shortages.
    /// For instance, in tundra regions I have tundra tiles set as Food, but grass are not.
    /// A desert region sets Plains as Food but Grass is not, while a Jungle region sets Grass as Food but Plains aren't.
    /// - [`TileType::Good`] act as a hedge, and are the main way of differentiating one candidate site from another,
    /// so that among a group of plots of similar terrain, the best tends to get picked.
    /// - [`TileType::Production`] is used to identify tiles that yield production.
    /// - [`TileType::Junk`] is used to identify tiles that yield nothing.
    fn measure_single_tile(&self, tile: Tile, region: &Region) -> EnumMap<TileType, bool> {
        let region_type = region.region_type;
        // Notice: "Food" is not strictly equivalent to tile yield.
        //
        // Different regions obtain food in various ways.
        // Tundra, Jungle, Forest, Desert, and Plains regions will receive bonus resource support
        // to compensate for food shortages.
        //
        // `data` hold the results, all starting as `false`。
        let mut data = EnumMap::default();

        match tile.terrain_type(self) {
            TerrainType::Water => {
                if tile.feature(self) == Some(Feature::Ice) {
                    data[TileType::Junk] = true;
                } else if tile.base_terrain(self) == BaseTerrain::Lake {
                    data[TileType::Food] = true;
                    data[TileType::Good] = true;
                } else if region.landmass_id.is_none()
                    && tile.base_terrain(self) == BaseTerrain::Coast
                {
                    data[TileType::Good] = true;
                }
                return data;
            }
            TerrainType::Mountain => {
                data[TileType::Junk] = true;
                return data;
            }
            TerrainType::Flatland | TerrainType::Hill => (),
        }

        // Tackle with the tile's terrain type is hill or flatland and has feature.
        if let Some(feature) = tile.feature(self) {
            match feature {
                Feature::Forest => {
                    data[TileType::Production] = true;
                    data[TileType::Good] = true;
                    if region_type == RegionType::Forest || region_type == RegionType::Tundra {
                        data[TileType::Food] = true;
                    }
                    return data;
                }
                Feature::Jungle => {
                    if region_type != RegionType::Grassland {
                        data[TileType::Food] = true;
                        data[TileType::Good] = true;
                    } else if tile.terrain_type(self) == TerrainType::Hill {
                        data[TileType::Production] = true;
                    }
                    return data;
                }
                Feature::Marsh => {
                    return data;
                }
                Feature::Oasis | Feature::Floodplain => {
                    data[TileType::Food] = true;
                    data[TileType::Good] = true;
                    return data;
                }
                _ => (),
            }
        }

        // Tackle with the tile's terrain type is hill and has no feature.
        if tile.terrain_type(self) == TerrainType::Hill {
            data[TileType::Production] = true;
            data[TileType::Good] = true;
            return data;
        }

        // Tackle with tile's terrain type is flatland and has no feature.
        match tile.base_terrain(self) {
            BaseTerrain::Grassland => {
                data[TileType::Good] = true;
                if region_type == RegionType::Jungle
                    || region_type == RegionType::Forest
                    || region_type == RegionType::Hill
                    || region_type == RegionType::Grassland
                    || region_type == RegionType::Hybrid
                {
                    data[TileType::Food] = true;
                }
                return data;
            }
            BaseTerrain::Desert => {
                if region_type != RegionType::Desert {
                    data[TileType::Junk] = true;
                }
                return data;
            }
            BaseTerrain::Plain => {
                data[TileType::Good] = true;
                if region_type == RegionType::Tundra
                    || region_type == RegionType::Desert
                    || region_type == RegionType::Hill
                    || region_type == RegionType::Plain
                    || region_type == RegionType::Hybrid
                {
                    data[TileType::Food] = true;
                }
                return data;
            }
            BaseTerrain::Tundra => {
                if region_type == RegionType::Tundra {
                    data[TileType::Food] = true;
                    data[TileType::Good] = true;
                }
                return data;
            }
            BaseTerrain::Snow => {
                data[TileType::Junk] = true;
                return data;
            }
            _ => (),
        }

        data
    }
}

#[derive(Enum)]
pub enum TileType {
    /// Tile yield food, or tile will be placed bonus resource to yield food according to region type in the future.
    Food,
    /// Tile yield production.
    Production,
    /// "Good" tiles act as a hedge, helping differentiate candidate sites.
    /// Among similar terrain tiles, they ensure the best one is selected.
    /// For example, when using [`TileMap::evaluate_candidate_tile`],
    /// the candidate tile with the most "Good" tiles within a 3-tile radius will get a higher score.
    Good,
    /// Tile yield nothing.
    Junk,
}
