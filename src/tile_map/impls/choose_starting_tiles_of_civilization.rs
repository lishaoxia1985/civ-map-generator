use std::collections::HashMap;

use bitflags::bitflags;
use enum_map::{Enum, EnumMap};

use crate::{
    grid::{
        Rectangle,
        hex_grid::{HexOrientation, Offset},
        offset_coordinate::OffsetCoordinate,
    },
    map_parameters::MapParameters,
    tile::Tile,
    tile_component::{BaseTerrain, Feature, TerrainType},
    tile_map::{Layer, TileMap},
};

use super::generate_regions::{Region, RegionType};

impl TileMap {
    // function AssignStartingPlots:ChooseLocations
    /// Get starting tile for each civilization according to region. Every region will have a starting tile for a civilization.
    pub fn choose_starting_tiles_of_civilization(&mut self, map_parameters: &MapParameters) {
        // Sort the region list by average fertility
        self.region_list
            .sort_by(|a, b| a.average_fertility().total_cmp(&b.average_fertility()));

        // When map_parameters.region_divide_method is `RegionDivideMethod::WholeMapRectangle` or `RegionDivideMethod::CustomRectangle`, all region's landmass_id is always `None`.
        let ignore_landmass_id = self.region_list[0].area_id.is_none();

        (0..self.region_list.len()).for_each(|region_index| {
            if ignore_landmass_id {
                self.find_start_without_regard_to_area_id(map_parameters, region_index);
            } else if map_parameters.civ_require_coastal_land_start {
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
    ///
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    ///   and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
    fn find_start_without_regard_to_area_id(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> (bool, bool) {
        let grid = self.world_grid.grid;

        let region = &self.region_list[region_index];

        let mut fallback_tile_and_score = Vec::new();

        let mut area_id_and_fertility = HashMap::new();

        // Store the candidate starting tile in each area (different area_id means different area)
        // At first, the candidate starting tile is flatland or hill, and then it should meet one of the following conditions:
        // 1. It is a coastal land tile
        // 2. It is not a coastal land tile, and it does not have any coastal land tiles as neighbors
        let mut area_id_and_candidate_tiles: HashMap<usize, Vec<Tile>> = HashMap::new();

        for (i, tile) in region
            .rectangle
            .all_cells(&grid)
            .map(Tile::from_cell)
            .enumerate()
        {
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
                self.iterate_through_candidate_tile_list(tile_list, region);

            if let Some(election1_tile) = eletion1_tile {
                self.region_list[region_index]
                    .starting_tile
                    .set(election1_tile)
                    .unwrap();
                self.place_impact_and_ripples(election1_tile, Layer::Civilization, u32::MAX);
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
            self.region_list[region_index]
                .starting_tile
                .set(max_score_tile)
                .unwrap();
            self.place_impact_and_ripples(max_score_tile, Layer::Civilization, u32::MAX);
            (true, false)
        } else {
            let origin = region.rectangle.origin();

            let tile = Tile::from_offset(origin, grid);
            tile.set_terrain_type(self, TerrainType::Flatland);
            tile.set_base_terrain(self, BaseTerrain::Grassland);
            tile.clear_feature(self);
            tile.clear_natural_wonder(self);
            self.region_list[region_index]
                .starting_tile
                .set(tile)
                .unwrap();
            self.place_impact_and_ripples(tile, Layer::Civilization, u32::MAX);
            (false, true)
        }
    }

    // function AssignStartingPlots:FindCoastalStart
    /// Find a starting tile which is coastal land for a region:
    /// - If the number of coastal land tiles in the region is less than 3, choose inland tile as starting tile (use [`TileMap::find_start`]).
    /// - If the number of coastal land tiles in the region is greater than or equal to 3, choose coastal land tiles as starting tile.
    /// - If there is no eligible starting tile, force a starting tile to be placed.
    ///
    /// # Returns
    ///
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    ///   and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
    fn find_coastal_land_start(
        &mut self,
        map_parameters: &MapParameters,
        region_index: usize,
    ) -> (bool, bool) {
        let grid = self.world_grid.grid;

        let mut fallback_tile_and_score = Vec::new();

        let terrain_statistic = self.region_list[region_index]
            .terrain_statistic
            .get()
            .unwrap();

        let coastal_land_sum = terrain_statistic.coastal_land_count;

        if coastal_land_sum < 3 {
            // This region cannot support an Along Ocean start.
            // Try instead to find an inland start for it.
            // When `success_flag` is `false`,
            // We don't need write the code to force a starting tile to be placed, because the `find_start` function will do it for us.
            let (success_flag, forced_placement_flag) =
                self.find_start(map_parameters, region_index);

            return (success_flag, forced_placement_flag);
        }

        let rectangle = self.region_list[region_index].rectangle;

        // Positioner defaults. These are the controls for the "Center Bias" placement method for civ starts in regions.
        const CENTER_BIAS: f64 = 1. / 3.; // d% of radius from region center to examine first
        const MIDDLE_BIAS: f64 = 2. / 3.; // d% of radius from region center to check second

        // Get the rectangle whose width and height is `CENTER_BIAS` times of the original rectangle, and it is in the center of the original rectangle.
        let center_rectangle = rectangle.scaled_center_crop(CENTER_BIAS, &grid);

        // Get the rectangle whose width and height is `MIDDLE_BIAS` times of the original rectangle, and it is in the middle of the original rectangle.
        let middle_rectangle = rectangle.scaled_center_crop(MIDDLE_BIAS, &grid);

        let mut center_coastal_tiles = Vec::new();
        let mut center_tiles_on_river = Vec::new();
        let mut center_fresh_tiles = Vec::new();
        let mut center_dry_tiles = Vec::new();

        let mut middle_coastal_tiles = Vec::new();
        let mut middle_tiles_on_river = Vec::new();
        let mut middle_fresh_tiles = Vec::new();
        let mut middle_dry_tiles = Vec::new();

        let mut outer_coastal_tiles = Vec::new();

        for tile in rectangle.all_cells(&grid).map(Tile::from_cell) {
            if tile.can_be_civilization_starting_tile(self, map_parameters) {
                let area_id = tile.area_id(self);
                let landmass_id = self.region_list[region_index].area_id;
                if landmass_id == Some(area_id) {
                    if center_rectangle.contains(tile.to_cell(), &grid) {
                        // Center Bias
                        center_coastal_tiles.push(tile);
                        if tile.has_river(self) {
                            center_tiles_on_river.push(tile);
                        } else if tile.is_freshwater(self) {
                            center_fresh_tiles.push(tile);
                        } else {
                            center_dry_tiles.push(tile);
                        }
                    } else if middle_rectangle.contains(tile.to_cell(), &grid) {
                        // Middle Bias
                        middle_coastal_tiles.push(tile);
                        if tile.has_river(self) {
                            middle_tiles_on_river.push(tile);
                        } else if tile.is_freshwater(self) {
                            middle_fresh_tiles.push(tile);
                        } else {
                            middle_dry_tiles.push(tile);
                        }
                    } else {
                        outer_coastal_tiles.push(tile);
                    }
                }
            }
        }

        let region = &self.region_list[region_index];

        if center_coastal_tiles.len() + middle_coastal_tiles.len() > 0 {
            let candidate_lists = [
                center_tiles_on_river,
                center_fresh_tiles,
                center_dry_tiles,
                middle_tiles_on_river,
                middle_fresh_tiles,
                middle_dry_tiles,
            ];

            for tile_list in candidate_lists.iter() {
                let (eletion1_tile, election2_tile, _, election2_tile_score) =
                    self.iterate_through_candidate_tile_list(tile_list, region);

                if let Some(election1_tile) = eletion1_tile {
                    self.region_list[region_index]
                        .starting_tile
                        .set(election1_tile)
                        .unwrap();
                    self.place_impact_and_ripples(election1_tile, Layer::Civilization, u32::MAX);
                    return (true, false);
                }
                if let Some(election_2_tile) = election2_tile {
                    fallback_tile_and_score.push((election_2_tile, election2_tile_score));
                }
            }
        }

        if !outer_coastal_tiles.is_empty() {
            let mut outer_eligible_list = Vec::new();
            let mut found_eligible = false;
            let mut found_fallback = false;
            let mut best_fallback_score = -50;
            let mut best_fallback_index = None;

            // Process list of candidate tiles.
            for tile in outer_coastal_tiles.into_iter() {
                let (score, meets_minimum_requirements) =
                    self.evaluate_candidate_tile(tile, region);

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
                // Iterate through eligible tiles and choose the one closest to the center of the region.
                let mut closest_tile = None;
                let mut closest_distance =
                    u32::max(self.world_grid.size().width, self.world_grid.size().height) as f64;

                // Because west_x >= 0, bullseye_x will always be >= 0.
                let mut bullseye_x = rectangle.west_x() as f64 + (rectangle.width() as f64 / 2.0);
                // Because south_y >= 0, bullseye_y will always be >= 0.
                let mut bullseye_y = rectangle.south_y() as f64 + (rectangle.height() as f64 / 2.0);

                match (grid.layout.orientation, grid.offset) {
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
                    let offset_coordinate = tile.to_offset(grid);

                    let [x, y] = offset_coordinate.to_array();

                    let mut adjusted_x = x as f64;
                    let mut adjusted_y = y as f64;

                    match (grid.layout.orientation, grid.offset) {
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

                    if x < rectangle.west_x() {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_x += self.world_grid.size().width as f64;
                    }
                    if y < rectangle.south_y() {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_y += self.world_grid.size().height as f64;
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
                    // Re-get tile score for inclusion in start tile data.
                    let (_score, _meets_minimum_requirements) =
                        self.evaluate_candidate_tile(closest_tile, region);

                    // Assign this tile as the start for this region.
                    self.region_list[region_index]
                        .starting_tile
                        .set(closest_tile)
                        .unwrap();
                    self.place_impact_and_ripples(closest_tile, Layer::Civilization, u32::MAX);
                    return (true, false);
                }
            }

            // Add the fallback tile (best scored tile) from the Outer region to the fallback list.
            if found_fallback && let Some(best_fallback_index) = best_fallback_index {
                fallback_tile_and_score.push((best_fallback_index, best_fallback_score));
            }
        }

        let max_score_tile = fallback_tile_and_score
            .iter()
            .max_by_key(|&(_, score)| score)
            .map(|&(tile, _)| tile);

        if let Some(max_score_tile) = max_score_tile {
            self.region_list[region_index]
                .starting_tile
                .set(max_score_tile)
                .unwrap();
            self.place_impact_and_ripples(max_score_tile, Layer::Civilization, u32::MAX);
            (true, false)
        } else {
            // This region cannot support an Along Ocean start.
            // Try instead to find an inland start for it.
            // When `success_flag` is `false`,
            // We don't need write the code to force a starting tile to be placed, because the `find_start` function will do it for us.
            let (success_flag, forced_placement_flag) =
                self.find_start(map_parameters, region_index);
            (success_flag, forced_placement_flag)
        }
    }

    // function AssignStartingPlots:FindStart
    /// Find a starting tile for a region.
    ///
    /// # Returns
    ///
    /// This function returns a tuple:
    /// - first element. If a starting tile was found in the region, it is `true`, otherwise `false`.
    /// - second element. If the region had no eligible starting tiles and a starting tile was forced to be placed,
    ///   and then first element is `false`, and the second element is `true`. If first element is `true`, then the second element is always `false`.
    fn find_start(&mut self, map_parameters: &MapParameters, region_index: usize) -> (bool, bool) {
        let grid = self.world_grid.grid;

        let mut fallback_tile_and_score = Vec::new();

        let region = &self.region_list[region_index];

        let rectangle = region.rectangle;

        // Positioner defaults. These are the controls for the "Center Bias" placement method for civ starts in regions.
        const CENTER_BIAS: f64 = 1. / 3.; // d% of radius from region center to examine first
        const MIDDLE_BIAS: f64 = 2. / 3.; // d% of radius from region center to check second

        // Get the rectangle whose width and height is `CENTER_BIAS` times of the original rectangle, and it is in the center of the original rectangle.
        let center_rectangle = rectangle.scaled_center_crop(CENTER_BIAS, &grid);

        // Get the rectangle whose width and height is `MIDDLE_BIAS` times of the original rectangle, and it is in the middle of the original rectangle.
        let middle_rectangle = rectangle.scaled_center_crop(MIDDLE_BIAS, &grid);

        let mut center_candidates = Vec::new();
        let mut center_river = Vec::new();
        let mut center_coastal_land_and_freshwater = Vec::new();
        let mut center_inland_dry_land = Vec::new();

        let mut middle_candidates = Vec::new();
        let mut middle_river = Vec::new();
        let mut middle_coastal_land_and_freshwater = Vec::new();
        let mut middle_inland_dry_land = Vec::new();

        let mut outer_tiles = Vec::new();

        for tile in region.rectangle.all_cells(&grid).map(Tile::from_cell) {
            if tile.can_be_civilization_starting_tile(self, map_parameters) {
                let area_id = tile.area_id(self);
                if region.area_id == Some(area_id) {
                    if center_rectangle.contains(tile.to_cell(), &grid) {
                        // Center Bias
                        center_candidates.push(tile);
                        if tile.has_river(self) {
                            center_river.push(tile);
                        } else if tile.is_freshwater(self) || tile.is_coastal_land(self) {
                            center_coastal_land_and_freshwater.push(tile);
                        } else {
                            center_inland_dry_land.push(tile);
                        }
                    } else if middle_rectangle.contains(tile.to_cell(), &grid) {
                        // Middle Bias
                        middle_candidates.push(tile);
                        if tile.has_river(self) {
                            middle_river.push(tile);
                        } else if tile.is_freshwater(self) || tile.is_coastal_land(self) {
                            middle_coastal_land_and_freshwater.push(tile);
                        } else {
                            middle_inland_dry_land.push(tile);
                        }
                    } else {
                        outer_tiles.push(tile);
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
                    self.iterate_through_candidate_tile_list(tile_list, region);

                if let Some(election1_tile) = eletion1_tile {
                    self.region_list[region_index]
                        .starting_tile
                        .set(election1_tile)
                        .unwrap();
                    self.place_impact_and_ripples(election1_tile, Layer::Civilization, u32::MAX);
                    return (true, false);
                }
                if let Some(election_2_tile) = election2_tile {
                    fallback_tile_and_score.push((election_2_tile, election2_tile_score));
                }
            }
        }

        if !outer_tiles.is_empty() {
            let mut outer_eligible_list = Vec::new();
            let mut found_eligible = false;
            let mut found_fallback = false;
            let mut best_fallback_score = -50;
            let mut best_fallback_index = None;

            // Process list of candidate tiles.
            for tile in outer_tiles.into_iter() {
                let (score, meets_minimum_requirements) =
                    self.evaluate_candidate_tile(tile, region);

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
                // Iterate through eligible tiles and choose the one closest to the center of the region.
                let mut closest_tile = None;
                let mut closest_distance =
                    u32::max(self.world_grid.size().width, self.world_grid.size().height) as f64;

                // Because west_x >= 0, bullseye_x will always be >= 0.
                let mut bullseye_x = rectangle.west_x() as f64 + (rectangle.width() as f64 / 2.0);
                // Because south_y >= 0, bullseye_y will always be >= 0.
                let mut bullseye_y = rectangle.south_y() as f64 + (rectangle.height() as f64 / 2.0);

                match (grid.layout.orientation, grid.offset) {
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
                    let offset_coordinate = tile.to_offset(grid);

                    let [x, y] = offset_coordinate.to_array();

                    let mut adjusted_x = x as f64;
                    let mut adjusted_y = y as f64;

                    match (grid.layout.orientation, grid.offset) {
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

                    if x < region.rectangle.west_x() {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_x += self.world_grid.size().width as f64;
                    }
                    if y < region.rectangle.south_y() {
                        // wrapped around: un-wrap it for test purposes.
                        adjusted_y += self.world_grid.size().height as f64;
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
                    // Re-get tile score for inclusion in start tile data.
                    let (_score, _meets_minimum_requirements) =
                        self.evaluate_candidate_tile(closest_tile, region);

                    // Assign this tile as the start for this region.
                    self.region_list[region_index]
                        .starting_tile
                        .set(closest_tile)
                        .unwrap();
                    self.place_impact_and_ripples(closest_tile, Layer::Civilization, u32::MAX);
                    return (true, false);
                }
            }

            // Add the fallback tile (best scored tile) from the Outer region to the fallback list.
            if found_fallback && let Some(best_fallback_index) = best_fallback_index {
                fallback_tile_and_score.push((best_fallback_index, best_fallback_score));
            }
        }

        let max_score_tile = fallback_tile_and_score
            .iter()
            .max_by_key(|&(_, score)| score)
            .map(|&(tile, _)| tile);

        if let Some(max_score_tile) = max_score_tile {
            self.region_list[region_index]
                .starting_tile
                .set(max_score_tile)
                .unwrap();
            self.place_impact_and_ripples(max_score_tile, Layer::Civilization, u32::MAX);
            (true, false)
        } else {
            let origin = region.rectangle.origin();

            let tile = Tile::from_offset(origin, grid);
            tile.set_terrain_type(self, TerrainType::Flatland);
            tile.set_base_terrain(self, BaseTerrain::Grassland);
            tile.clear_feature(self);
            tile.clear_natural_wonder(self);
            self.region_list[region_index]
                .starting_tile
                .set(tile)
                .unwrap();
            self.place_impact_and_ripples(tile, Layer::Civilization, u32::MAX);
            (false, true)
        }
    }

    // function AssignStartingPlots:IterateThroughCandidatePlotList
    /// Iterates through a list of candidate tiles and returns the best tile and fallback tile.
    ///
    /// This function assumes all candidate tiles can have a city built on them.
    /// Any tiles not allowed to have a city should be weeded out when building the candidate list.
    fn iterate_through_candidate_tile_list(
        &self,
        candidate_tile_list: &[Tile],
        region: &Region,
    ) -> (Option<Tile>, Option<Tile>, i32, i32) {
        let mut best_tile_score = -5000;
        let mut best_tile = None;
        let mut best_fallback_score = -5000;
        let mut best_fallback_tile = None;

        for &tile in candidate_tile_list {
            let (score, meets_minimum_requirements) = self.evaluate_candidate_tile(tile, region);

            if meets_minimum_requirements {
                if score > best_tile_score {
                    best_tile_score = score;
                    best_tile = Some(tile);
                }
            } else if score > best_fallback_score {
                best_fallback_score = score;
                best_fallback_tile = Some(tile);
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
    ///
    /// This function returns a tuple:
    /// - first element. The score of the tile.
    /// - second element. A boolean indicating whether the tile meets the minimum requirements. If it does not meet the minimum requirements, it will be used as a fallback tile.
    ///   If the tile meets the minimum requirements, it is `true`, otherwise `false`.
    fn evaluate_candidate_tile(&self, tile: Tile, region: &Region) -> (i32, bool) {
        let grid = self.world_grid.grid;

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

        if tile.is_coastal_land(self) {
            coastal_land_score = 40;
        }

        // Usually, the tile have 6 neighbors. If not, we count the missing neighbors as junk.
        junk_total += 6 - tile.neighbor_tiles(grid).count() as i32;

        tile.neighbor_tiles(grid).for_each(|neighbor_tile| {
            let yield_flags = self.measure_tile_yield(neighbor_tile, region);
            if yield_flags.contains(YieldFlags::Food) {
                food_total += 1;
            }
            if yield_flags.contains(YieldFlags::Production) {
                production_total += 1;
            }
            if yield_flags.contains(YieldFlags::Good) {
                good_total += 1;
            }
            if yield_flags.contains(YieldFlags::Junk) {
                junk_total += 1;
            }
            if neighbor_tile.has_river(self) {
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

        // Usually, there are 12 tiles at distance 2. If not, we count the missing tiles as junk.
        junk_total += 6 * 2 - tile.tiles_at_distance(2, grid).count() as i32;

        tile.tiles_at_distance(2, grid)
            .for_each(|tile_at_distance_two| {
                let yield_flags = self.measure_tile_yield(tile_at_distance_two, region);
                if yield_flags.contains(YieldFlags::Food) {
                    food_total += 1;
                }
                if yield_flags.contains(YieldFlags::Production) {
                    production_total += 1;
                }
                if yield_flags.contains(YieldFlags::Good) {
                    good_total += 1;
                }
                if yield_flags.contains(YieldFlags::Junk) {
                    junk_total += 1;
                }
                if tile_at_distance_two.has_river(self) {
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

        // Usually, there are 18 tiles at distance 3. If not, we count the missing tiles as junk.
        junk_total += 6 * 3 - tile.tiles_at_distance(3, grid).count() as i32;

        tile.tiles_at_distance(3, grid)
            .for_each(|tile_at_distance_three| {
                let yield_flags = self.measure_tile_yield(tile_at_distance_three, region);
                if yield_flags.contains(YieldFlags::Food) {
                    food_total += 1;
                }
                if yield_flags.contains(YieldFlags::Production) {
                    production_total += 1;
                }
                if yield_flags.contains(YieldFlags::Good) {
                    good_total += 1;
                }
                if yield_flags.contains(YieldFlags::Junk) {
                    junk_total += 1;
                }
                if tile_at_distance_three.has_river(self) {
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
        if self.layer_data[Layer::Civilization][tile.index()] != 0 {
            // This candidate is near an already placed start. This invalidates its
            // eligibility for first-pass placement; but it may still qualify as a
            // fallback site, so we will reduce its Score according to the bias factor.
            // Closer to existing start points, lower the score becomes.
            meets_minimum_requirements = false;
            final_score = (final_score as f64
                * (100 - self.layer_data[Layer::Civilization][tile.index()]) as f64
                / 100.0) as i32;
        }
        (final_score, meets_minimum_requirements)
    }

    // function AssignStartingPlots:MeasureSinglePlot
    /// Measures current tile's yield and returns the corresponding flags.
    ///
    /// Measures a single tile's yield whether it is Food, Production, Good, Junk, or a combination of (Food, Production, Good).
    /// - [`YieldFlags::Food`] may be misleading, as this is the primary mechanism for biasing starting terrain.
    ///   It is not strictly equivalent to tile yield. That's because regions with different [`RegionType`] obtain food in various ways.
    //    Tundra, Jungle, Forest, Desert, and Plains regions will receive bonus resource support to compensate for food shortages.
    ///   For example, in tundra regions I have tundra tiles set as Food, but grass are not.
    ///   A desert region sets Plains as Food but Grass is not, while a Jungle region sets Grass as Food but Plains aren't.
    /// - [`YieldFlags::Good`] act as a hedge, and are the main way of differentiating one candidate site from another.
    ///   Among similar terrain tiles, they ensure the best one is selected.
    ///   For example, if the candidate tiles have the same yield value,
    ///   then the tile with more `"Good"` tiles within a 3-tile radius will receive a higher selection priority.
    /// - [`YieldFlags::Production`] is used to identify tiles that yield production.
    /// - [`YieldFlags::Junk`] is used to identify tiles that yield nothing.
    fn measure_tile_yield(&self, tile: Tile, region: &Region) -> YieldFlags {
        let region_type = *region.region_type.get().unwrap();

        let mut yield_flags = YieldFlags::empty();

        let terrain_type = tile.terrain_type(self);
        let base_terrain = tile.base_terrain(self);
        let feature = tile.feature(self);

        // Handle Flatland and Hill with features first
        // Notes: These feature below only occurs on flatland and hill
        if let Some(feature) = feature {
            match feature {
                Feature::Forest => {
                    yield_flags |= YieldFlags::Production | YieldFlags::Good;
                    if region_type == RegionType::Forest || region_type == RegionType::Tundra {
                        yield_flags |= YieldFlags::Food;
                    }
                    return yield_flags;
                }
                Feature::Jungle => {
                    if region_type != RegionType::Grassland {
                        yield_flags |= YieldFlags::Food | YieldFlags::Good;
                    } else if terrain_type == TerrainType::Hill {
                        yield_flags |= YieldFlags::Production;
                    }
                    return yield_flags;
                }
                Feature::Marsh => {
                    return yield_flags;
                }
                Feature::Oasis | Feature::Floodplain => {
                    yield_flags |= YieldFlags::Food | YieldFlags::Good;
                    return yield_flags;
                }
                _ => (),
            }
        }

        match (terrain_type, base_terrain, feature) {
            (TerrainType::Water, _, Some(Feature::Ice)) => {
                yield_flags |= YieldFlags::Junk;
                return yield_flags;
            }
            (TerrainType::Water, BaseTerrain::Lake, _) => {
                yield_flags |= YieldFlags::Food | YieldFlags::Good;
                return yield_flags;
            }
            (TerrainType::Water, BaseTerrain::Coast, _) if region.area_id.is_none() => {
                yield_flags |= YieldFlags::Good;
                return yield_flags;
            }
            (TerrainType::Water, _, _) => {
                return yield_flags;
            }
            (TerrainType::Mountain, _, _) => {
                yield_flags |= YieldFlags::Junk;
                return yield_flags;
            }
            (TerrainType::Hill, _, None) => {
                yield_flags |= YieldFlags::Production | YieldFlags::Good;
                return yield_flags;
            }
            (TerrainType::Flatland, BaseTerrain::Grassland, None) => {
                yield_flags |= YieldFlags::Good;
                if matches!(
                    region_type,
                    RegionType::Jungle
                        | RegionType::Forest
                        | RegionType::Hill
                        | RegionType::Grassland
                        | RegionType::Hybrid
                ) {
                    yield_flags |= YieldFlags::Food;
                }
                return yield_flags;
            }
            (TerrainType::Flatland, BaseTerrain::Desert, None) => {
                if region_type != RegionType::Desert {
                    yield_flags |= YieldFlags::Junk;
                }
                return yield_flags;
            }
            (TerrainType::Flatland, BaseTerrain::Plain, None) => {
                yield_flags |= YieldFlags::Good;
                if matches!(
                    region_type,
                    RegionType::Tundra
                        | RegionType::Desert
                        | RegionType::Hill
                        | RegionType::Plain
                        | RegionType::Hybrid
                ) {
                    yield_flags |= YieldFlags::Food;
                }
                return yield_flags;
            }
            (TerrainType::Flatland, BaseTerrain::Tundra, None) => {
                if region_type == RegionType::Tundra {
                    yield_flags |= YieldFlags::Food | YieldFlags::Good;
                }
                return yield_flags;
            }
            (TerrainType::Flatland, BaseTerrain::Snow, None) => {
                yield_flags |= YieldFlags::Junk;
                return yield_flags;
            }
            (_, _, _) => (),
        }

        yield_flags
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct YieldFlags: u8 {
        /// Tile yield food, or tile will be placed bonus resource to yield food according to region type in the future.
        const Food = 1 << 0;
        /// Tile yield production.
        const Production = 1 << 1;
        /// `"Good"` tiles act as a hedge, helping differentiate candidate sites.
        /// Among similar terrain tiles, they ensure the best one is selected.
        /// For example, when calling [`TileMap::evaluate_candidate_tile`],
        /// and the candidate tiles have the same yield value,
        /// then the tile with more `"Good"` tiles within a 3-tile radius will receive a higher selection priority.
        const Good = 1 << 2;
        /// Tile yield nothing.
        ///
        /// # Notes
        ///
        /// Junk is mutually exclusive with other flags.
        /// When a tile is Junk, it should not have any other flags set.
        const Junk = 1 << 3;
    }
}
