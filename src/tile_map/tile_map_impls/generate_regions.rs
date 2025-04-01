use std::cmp::min;

use std::collections::{HashMap, HashSet};

use enum_map::{enum_map, Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::{
    component::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType},
    grid::OffsetCoordinate,
    tile_map::{tile::Tile, MapParameters, RegionDivideMethod, TileMap, WrapType},
};

impl TileMap {
    // function AssignStartingPlots:GenerateRegions(args)
    /// Generates regions for the map according civilization number and region divide method.
    /// The number of regions is equal to the number of civilizations.
    pub fn generate_regions(&mut self, map_parameters: &MapParameters) {
        let civilization_num = map_parameters.civilization_num;

        match map_parameters.region_divide_method {
            RegionDivideMethod::Pangaea => {
                // -- Identify the biggest landmass.
                let biggest_landmass_id = self.get_biggest_landmass_id();

                let landmass_region =
                    Region::landmass_region(self, map_parameters, biggest_landmass_id);

                self.divide_into_regions(map_parameters, civilization_num, landmass_region);
            }
            RegionDivideMethod::Continent => {
                let mut area_id_and_terrain_type: HashMap<i32, HashSet<_>> = HashMap::new();

                self.iter_tiles().for_each(|tile| {
                    let area_id = tile.area_id(self);
                    let terrain_type = tile.terrain_type(self);
                    area_id_and_terrain_type
                        .entry(area_id)
                        .or_default()
                        .insert(terrain_type);
                });

                let only_water_terrain_type: HashSet<TerrainType> =
                    HashSet::from([TerrainType::Water]);
                let only_mountain_terrain_type: HashSet<TerrainType> =
                    HashSet::from([TerrainType::Mountain]);
                // Get all landmass ids
                // - landmasses are areas that don't have only water or only mountain tiles
                // - Filter out the areas that are only water or only mountains
                let mut landmass_ids: Vec<_> = area_id_and_terrain_type
                    .iter()
                    .filter(|(_, terrain_types)| {
                        terrain_types != &&only_water_terrain_type
                            && terrain_types != &&only_mountain_terrain_type
                    })
                    .map(|(&area_id, _)| (area_id))
                    .collect();

                landmass_ids.sort_unstable();

                let mut landmass_region_list: Vec<_> = landmass_ids
                    .into_iter()
                    .map(|landmass_id| Region::landmass_region(self, map_parameters, landmass_id))
                    .collect();

                landmass_region_list.sort_by_key(|region| region.fertility_sum);

                let landmass_num = landmass_region_list.len() as u32;

                // If less players than landmasses, we will ignore the extra landmasses.
                let relevant_landmass_num = min(landmass_num, civilization_num);

                // Create a new list containing the most fertile land areas by reversing the sorted list and selecting the top `relevant_land_areas_num` items.
                let best_landmass_region_list = landmass_region_list
                    .into_iter()
                    .rev() // Reverse the iterator so the most fertile regions (which are at the end of the sorted list) come first.
                    .take(relevant_landmass_num as usize) // Take the top `relevant_land_areas_num` elements from the reversed list.
                    .collect::<Vec<_>>();

                let mut number_of_civs_on_landmass = vec![0; relevant_landmass_num as usize];

                for _ in 0..civilization_num {
                    best_landmass_region_list
                        .iter()
                        .enumerate()
                        .max_by(|&(index_a, region_a), &(index_b, region_b)| {
                            let score_a = region_a.fertility_sum as f64
                                / (number_of_civs_on_landmass[index_a] as f64 + 1.);
                            let score_b = region_b.fertility_sum as f64
                                / (number_of_civs_on_landmass[index_b] as f64 + 1.);
                            score_a.total_cmp(&score_b)
                        })
                        .map(|(index, _)| number_of_civs_on_landmass[index] += 1);
                }

                for (index, region) in best_landmass_region_list.into_iter().enumerate() {
                    if number_of_civs_on_landmass[index] > 0 {
                        self.divide_into_regions(
                            map_parameters,
                            number_of_civs_on_landmass[index],
                            region,
                        );
                    }
                }
            }
            RegionDivideMethod::WholeMapRectangle => {
                let rectangle = Rectangle {
                    west_x: 0,
                    south_y: 0,
                    width: map_parameters.map_size.width,
                    height: map_parameters.map_size.height,
                };

                let region = Region::rectangle_region(self, map_parameters, rectangle);
                self.divide_into_regions(map_parameters, civilization_num, region);
            }
            RegionDivideMethod::CustomRectangle(rectangle) => {
                let region = Region::rectangle_region(self, map_parameters, rectangle);
                self.divide_into_regions(map_parameters, civilization_num, region);
            }
        }
    }

    // function AssignStartingPlots:DivideIntoRegions
    /// Divides the region into subdivisions and get a vec of the subdivisions region.
    /// # Arguments
    /// * `map_parameters` - The map parameters.
    /// * `divisions_num` - The number of divisions to make. In origin code, this should <= 22.
    /// * `region` - The region to divide.
    fn divide_into_regions(
        &mut self,
        map_parameters: &MapParameters,
        divisions_num: u32,
        region: Region,
    ) {
        let mut stack = vec![(region, divisions_num)];

        while let Some((mut current_region, current_divisions_num)) = stack.pop() {
            if current_divisions_num == 1 {
                current_region.measure_terrain(self, map_parameters);
                current_region.determine_region_type();
                self.region_list.push(current_region);
            } else {
                match current_divisions_num {
                    2 => {
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, 50.0);
                        stack.push((first_section, 1));
                        stack.push((second_section, 1));
                    }
                    3 => {
                        let (first_section, second_section, third_section) =
                            current_region.chop_into_three_regions(map_parameters);
                        stack.push((first_section, 1));
                        stack.push((second_section, 1));
                        stack.push((third_section, 1));
                    }
                    5 => {
                        let chop_percent = 3. / 5. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 3));
                        stack.push((second_section, 2));
                    }
                    7 => {
                        let chop_percent = 3. / 7. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 3));
                        stack.push((second_section, 4));
                    }
                    11 => {
                        let chop_percent = 3. / 11. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 3));
                        stack.push((second_section, 8));
                    }
                    13 => {
                        let chop_percent = 5. / 13. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 5));
                        stack.push((second_section, 8));
                    }
                    17 => {
                        let chop_percent = 9. / 17. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 9));
                        stack.push((second_section, 8));
                    }
                    19 => {
                        let chop_percent = 7. / 19. * 100.0;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(map_parameters, chop_percent);
                        stack.push((first_section, 7));
                        stack.push((second_section, 12));
                    }
                    _ => {
                        if current_divisions_num % 3 == 0 {
                            let subdivisions = current_divisions_num / 3;
                            let (first_section, second_section, third_section) =
                                current_region.chop_into_three_regions(map_parameters);
                            stack.push((first_section, subdivisions));
                            stack.push((second_section, subdivisions));
                            stack.push((third_section, subdivisions));
                        } else if current_divisions_num % 2 == 0 {
                            let subdivisions = current_divisions_num / 2;
                            let (first_section, second_section) =
                                current_region.chop_into_two_regions(map_parameters, 50.0);
                            stack.push((first_section, subdivisions));
                            stack.push((second_section, subdivisions));
                        } else {
                            eprintln!(
                                "Erroneous number of regional divisions: {}",
                                current_divisions_num
                            );
                        }
                    }
                }
            }
        }
    }

    // function AssignStartingPlots:MeasureStartPlacementFertilityOfLandmass
    /// Returns a list of fertility values for all tiles in the landmass rectangle.
    fn measure_start_placement_fertility_of_landmass(
        &self,
        map_parameters: &MapParameters,
        area_id: i32,
        landmass_rectangle: Rectangle,
    ) -> Vec<i32> {
        let tile_count = landmass_rectangle.width * landmass_rectangle.height;

        let mut area_fertility_list = Vec::with_capacity(tile_count as usize);

        for tile in landmass_rectangle.iter_tiles(map_parameters) {
            if tile.area_id(self) != area_id {
                area_fertility_list.push(0);
            } else {
                let tile_fertility =
                    self.measure_start_placement_fertility_of_tile(map_parameters, tile, true);
                area_fertility_list.push(tile_fertility);
            }
        }

        area_fertility_list
    }

    // function AssignStartingPlots:MeasureStartPlacementFertilityInRectangle
    /// Returns a list of fertility values for all tiles in the rectangle.
    fn measure_start_placement_fertility_in_rectangle(
        &self,
        map_parameters: &MapParameters,
        rectangle: Rectangle,
    ) -> Vec<i32> {
        let tile_count = rectangle.width * rectangle.height;

        let mut area_fertility_list = Vec::with_capacity(tile_count as usize);

        for tile in rectangle.iter_tiles(map_parameters) {
            // Check for coastal land is disabled.
            let tile_fertility =
                self.measure_start_placement_fertility_of_tile(map_parameters, tile, false);
            area_fertility_list.push(tile_fertility);
        }

        area_fertility_list
    }

    // function AssignStartingPlots:MeasureStartPlacementFertilityOfPlot
    /// Returns the fertility of a tile for starting placement.
    fn measure_start_placement_fertility_of_tile(
        &self,
        map_parameters: &MapParameters,
        tile: Tile,
        check_for_coastal_land: bool,
    ) -> i32 {
        let mut tile_fertility = 0;
        let terrain_type = tile.terrain_type(self);
        let base_terrain = tile.base_terrain(self);
        let feature_type = tile.feature(self);

        // Measure Fertility -- Any cases absent from the process have a 0 value.
        match terrain_type {
            TerrainType::Mountain => {
                // Note, mountains cannot belong to a landmass AreaID, so they usually go unmeasured.
                return -2;
            }
            TerrainType::Hill => {
                tile_fertility += 1;
            }
            _ => {}
        }

        match base_terrain {
            BaseTerrain::Snow => {
                return -1;
            }
            BaseTerrain::Grassland => {
                tile_fertility += 3;
            }
            BaseTerrain::Plain => {
                tile_fertility += 4;
            }
            BaseTerrain::Coast | BaseTerrain::Lake | BaseTerrain::Tundra => {
                tile_fertility += 2;
            }
            BaseTerrain::Desert => {
                tile_fertility += 1;
            }
            _ => {}
        }

        if let Some(feature_type) = feature_type {
            match feature_type {
                Feature::Oasis => {
                    return 4; // Reducing Oasis value slightly.
                }
                Feature::Floodplain => {
                    return 5; // Reducing Flood Plains value slightly.
                }
                Feature::Forest => tile_fertility += 0,
                Feature::Jungle | Feature::Ice => tile_fertility -= 1,
                Feature::Marsh => tile_fertility -= 2,
                _ => {}
            }
        }

        if tile.has_river(self, map_parameters) {
            tile_fertility += 1;
        }

        if tile.is_freshwater(self, map_parameters) {
            tile_fertility += 1;
        }

        if check_for_coastal_land && tile.is_coastal_land(self, map_parameters) {
            tile_fertility += 2;
        }

        tile_fertility
    }

    /// Get landmass rectangle for the region.
    fn obtain_landmass_boundaries(
        &self,
        map_parameters: &MapParameters,
        area_id: i32,
    ) -> Rectangle {
        let map_height = map_parameters.map_size.height as i32;
        let map_width = map_parameters.map_size.width as i32;
        // -- Set up variables that will be returned by this function.
        let mut wrap_x = false;
        let mut wrap_y = false;
        let mut west_x = 0;
        let mut east_x = 0;
        let mut south_y = 0;
        let mut north_y = 0;

        // Check if the landmass wraps around the map horizontally.
        if map_parameters.map_wrapping.x == WrapType::Wrap {
            let mut found_first_column = false;
            let mut found_last_column = false;

            for y in 0..map_height {
                if !found_first_column {
                    let first_offset_coordinate = OffsetCoordinate::new(0, y);
                    let tile_first_index =
                        Tile::from_offset_coordinate(map_parameters, first_offset_coordinate)
                            .expect("Offset coordinate is outside the map!");
                    if tile_first_index.area_id(self) == area_id {
                        // Found a tile belonging to current area in first column.
                        found_first_column = true;
                    }
                }

                if !found_last_column {
                    let last_offset_coordinate = OffsetCoordinate::new(map_width - 1, y);
                    let tile_last_index =
                        Tile::from_offset_coordinate(map_parameters, last_offset_coordinate)
                            .expect("Offset coordinate is outside the map!");
                    if tile_last_index.area_id(self) == area_id {
                        // Found a tile belonging to current area in last column.
                        found_last_column = true;
                    }
                }

                // Break early if current area has tiles on both sides of map edge.
                if found_first_column && found_last_column {
                    wrap_x = true;
                    break;
                }
            }
        }

        // Check if the landmass wraps around the map vertically.
        if map_parameters.map_wrapping.y == WrapType::Wrap {
            let mut found_first_row = false;
            let mut found_last_row = false;
            for x in 0..map_width {
                if !found_first_row {
                    let first_offset_coordinate = OffsetCoordinate::new(x, 0);
                    let tile_first_index =
                        Tile::from_offset_coordinate(map_parameters, first_offset_coordinate)
                            .expect("Offset coordinate is outside the map!");
                    if tile_first_index.area_id(self) == area_id {
                        // Found a tile belonging to current area in first row.
                        found_first_row = true;
                    }
                }

                if !found_last_row {
                    let last_offset_coordinate = OffsetCoordinate::new(x, map_height - 1);
                    let tile_last_index =
                        Tile::from_offset_coordinate(map_parameters, last_offset_coordinate)
                            .expect("Offset coordinate is outside the map!");
                    if tile_last_index.area_id(self) == area_id {
                        // Found a tile belonging to current area in last row.
                        found_last_row = true;
                    }
                }

                // Break early if current area has tiles on both sides of map edge.
                if found_first_row && found_last_row {
                    wrap_y = true;
                    break;
                }
            }
        }

        // Find West and East edges of this landmass.
        if !wrap_x {
            // If the landmass does not wrap around the map horizontally.
            // Check for any area membership one column at a time, left to right.
            for x in 0..map_width {
                if (0..map_height).any(|y| {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    tile.area_id(self) == area_id
                }) {
                    west_x = x;
                    break;
                }
            }

            // Check for any area membership one column at a time, right to left.
            for x in (0..map_width).rev() {
                if (0..map_height).any(|y| {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    tile.area_id(self) == area_id
                }) {
                    east_x = x;
                    break;
                }
            }
        } else {
            // If the landmass wraps around the map horizontally.

            let mut landmass_spans_entire_world_x = true;

            // Check for end of area membership one column at a time, right to left.
            // When map is wrap_x, there must exist tiles in the column '0' and 'width-1' that are the area memberships.
            // So we don't need to check the column '0' and 'width-1' for area membership.
            for x in (1..(map_width - 1)).rev() {
                let mut found_area_in_column = false;

                for y in 0..map_height {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    if tile.area_id(self) == area_id {
                        found_area_in_column = true;
                    }
                }
                if !found_area_in_column {
                    west_x = x + 1;
                    landmass_spans_entire_world_x = false;
                    break;
                }
            }

            // Check for end of area membership one column at a time, left to right.
            // When map is wrap_x, there must exist tiles in the column '0' and 'width-1' that are the area memberships.
            // So we don't need to check the column '0' and 'width-1' for area membership.
            for x in 1..(map_width - 1) {
                let mut found_area_in_column = false;

                for y in 0..map_height {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    if tile.area_id(self) == area_id {
                        found_area_in_column = true;
                    }
                }
                if !found_area_in_column {
                    east_x = x - 1;
                    landmass_spans_entire_world_x = false;
                    break;
                }
            }

            // If landmass spans entire world, we'll treat it as if it does not wrap.
            if landmass_spans_entire_world_x {
                wrap_x = false;
                west_x = 0;
                east_x = map_width - 1;
            }
        }

        // Find South and North edges of this landmass.
        if !wrap_y {
            // If the landmass does not wrap around the map vertically.
            // Check for any area membership one row at a time, bottom to top.
            for y in 0..map_height {
                if (0..map_width).any(|x| {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    tile.area_id(self) == area_id
                }) {
                    south_y = y;
                    break;
                }
            }

            // Check for any area membership one row at a time, top to bottom.
            for y in (0..map_height).rev() {
                if (0..map_width).any(|x| {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    tile.area_id(self) == area_id
                }) {
                    north_y = y;
                    break;
                }
            }
        } else {
            // If the landmass wraps around the map vertically.
            let mut landmass_spans_entire_world_y = true;

            // Check for end of area membership one row at a time, top to bottom.
            // When map is wrap_y, there must exist tiles in the row '0' and 'map_height - 1' that are the area memberships.
            // So we don't need to check the row '0' and 'map_height - 1' for area membership.
            for y in (1..(map_height - 1)).rev() {
                let mut found_area_in_row = false;
                for x in 0..map_width {
                    // Checking row.
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    if tile.area_id(self) == area_id {
                        // Found a plot belonging to i_area_id, will have to check the next row too.
                        found_area_in_row = true;
                    }
                }
                if !found_area_in_row {
                    // Found empty row, which is just south of SouthY.
                    south_y = y + 1;
                    landmass_spans_entire_world_y = false;
                    break;
                }
            }

            // Check for end of area membership one row at a time, bottom to top.
            // When map is wrap_y, there must exist tiles in the row '0' and 'map_height - 1' that are the area memberships.
            // So we don't need to check the row '0' and 'map_height - 1' for area membership.
            for y in 1..(map_height - 1) {
                let mut found_area_in_row = false;
                for x in 0..map_width {
                    // Checking row.
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    let tile = Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                        .expect("Offset coordinate is outside the map!");
                    if tile.area_id(self) == area_id {
                        // Found a plot belonging to i_area_id, will have to check the next row too.
                        found_area_in_row = true;
                    }
                }
                if !found_area_in_row {
                    // Found empty column, which is just north of NorthY.
                    north_y = y - 1;
                    landmass_spans_entire_world_y = false;
                    break;
                }
            }

            // If landmass spans entire world, we'll treat it as if it does not wrap.
            if landmass_spans_entire_world_y {
                wrap_y = false;
                south_y = 0;
                north_y = map_height - 1;
            }
        }

        // Convert east_x and north_y into width and height.
        let width = if wrap_x {
            east_x - west_x + 1 + map_width
        } else {
            east_x - west_x + 1
        };

        let height = if wrap_y {
            north_y - south_y + 1 + map_height
        } else {
            north_y - south_y + 1
        };

        Rectangle {
            west_x,
            south_y,
            width,
            height,
        }
    }

    /// Get the biggest landmass id.
    fn get_biggest_landmass_id(&self) -> i32 {
        let mut area_id_and_terrain_type: HashMap<i32, HashSet<_>> = HashMap::new();

        self.iter_tiles().for_each(|tile| {
            let area_id = tile.area_id(self);
            let terrain_type = tile.terrain_type(self);
            area_id_and_terrain_type
                .entry(area_id)
                .or_default()
                .insert(terrain_type);
        });

        let only_water_terrain_type: HashSet<TerrainType> = HashSet::from([TerrainType::Water]);
        let only_mountain_terrain_type: HashSet<TerrainType> =
            HashSet::from([TerrainType::Mountain]);
        // Get all landmass ids
        // - landmasses are areas that don't have only water or only mountain tiles
        // - Filter out the areas that are only water or only mountains
        let landmass_id_and_size: Vec<_> = area_id_and_terrain_type
            .iter()
            .filter(|(_, terrain_types)| {
                terrain_types != &&only_water_terrain_type
                    && terrain_types != &&only_mountain_terrain_type
            })
            .map(|(&area_id, _)| (area_id, self.area_id_and_size[&area_id]))
            .collect();

        // Find the biggest landmass id
        landmass_id_and_size
            .iter()
            .max_by_key(|&(_, size)| size)
            .expect("`landmass_id_and_size` should not be empty!")
            .0
    }
}

#[derive(Debug, Clone, Copy)]
/// This struct is used to describe a rectangular region of the map.
/// We can use it to get all tiles in this region.
pub struct Rectangle {
    /// `west_x` should in the range `[0, map_width - 1]`. We will write these check in the future.
    pub west_x: i32,
    /// `south_y` should in the range `[0, map_height - 1]`. We will write these check in the future.
    pub south_y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    /// Returns an iterator over all tiles in current rectangle region of the map.
    pub fn iter_tiles<'a>(
        &'a self,
        map_parameters: &'a MapParameters,
    ) -> impl Iterator<Item = Tile> + 'a {
        (self.south_y..self.south_y + self.height).flat_map(move |y| {
            (self.west_x..self.west_x + self.width).map(move |x| {
                let offset_coordinate = OffsetCoordinate::new(x, y);
                Tile::from_offset_coordinate(map_parameters, offset_coordinate)
                    .expect("Offset coordinate is outside the map!")
            })
        })
    }

    /// Checks if the given tile is inside the current rectangle.
    ///
    /// Returns `true` if the given tile is inside the current rectangle.
    pub fn contains(&self, map_parameters: &MapParameters, tile: Tile) -> bool {
        let [mut x, mut y] = tile.to_offset_coordinate(map_parameters).to_array();

        // We should consider the map is wrapped around horizontally.
        if x < self.west_x {
            x += map_parameters.map_size.width;
        }

        // We should consider the map is wrapped around vertically.
        if y < self.south_y {
            y += map_parameters.map_size.height;
        }

        x >= self.west_x
            && x < self.west_x + self.width
            && y >= self.south_y
            && y < self.south_y + self.height
    }
}

/// The terrain statistic of the region.
/// Ensure that method [`Region::measure_terrain`] has been called before accessing this field, as it will be meaningless otherwise.
#[derive(Debug)]
pub struct TerrainStatistic {
    /// Each terrain type's number in the region.
    pub terrain_type_num: EnumMap<TerrainType, u32>,
    /// Each base terrain's number in the region.
    pub base_terrain_num: EnumMap<BaseTerrain, u32>,
    /// Each feature's number in the region.
    pub feature_num: EnumMap<Feature, u32>,
    /// The number of tiles with rivers in the region.
    pub river_num: u32,
    /// The number of tiles which are coastal land in the region.
    pub coastal_land_num: u32,
    /// The number of tiles which are land, not coastal land, but are next to coastal land in the region.
    pub next_to_coastal_land_num: u32,
}

impl Default for TerrainStatistic {
    fn default() -> Self {
        let terrain_type_num = enum_map! {
            _ => 0,
        };

        let base_terrain_num = enum_map! {
            _ => 0,
        };

        let feature_num = enum_map! {
            _ => 0,
        };

        TerrainStatistic {
            terrain_type_num,
            base_terrain_num,
            feature_num,
            river_num: 0,
            coastal_land_num: 0,
            next_to_coastal_land_num: 0,
        }
    }
}

#[derive(Debug)]
/// Region is a rectangular area of tiles.
/// In the edge rows and edge columns of a region, it is not allowed for all tiles' fertility to be 0.
pub struct Region {
    /// The rectangle that defines the region.
    pub rectangle: Rectangle,
    /// The 'area_id' of the landmass this region belongs to.
    /// When landmass_id is `None`, it means that we will consider all landmass in the region when we divide it into sub-regions or other operations.
    pub landmass_id: Option<i32>,
    /// List of fertility values for each tile in the region.
    pub fertility_list: Vec<i32>,
    /// Total fertility value of all tiles in the region.
    pub fertility_sum: i32,
    /// The number of tiles in the region.
    pub tile_count: i32,
    /// The terrain statistic of the region. Ensure that method [`Region::measure_terrain`] has been called before accessing this field, as it will be meaningless otherwise.
    pub terrain_statistic: TerrainStatistic,
    /// The type of the region. Ensure that method [`Region::determine_region_type`] has been called before accessing this field, as it will be meaningless otherwise.
    pub region_type: RegionType,
    /// The starting tile of the civilization in this region.
    pub starting_tile: Tile,
    /// The start location condition of the region.
    pub start_location_condition: StartLocationCondition,
    /// The luxury resource of the region.
    pub luxury_resource: String,
}

impl Region {
    fn new(rectangle: Rectangle, landmass_id: Option<i32>, fertility_list: Vec<i32>) -> Self {
        let fertility_sum = fertility_list.iter().sum();
        let tile_count = fertility_list.len() as i32;

        Region {
            rectangle,
            landmass_id,
            fertility_list,
            fertility_sum,
            tile_count,
            terrain_statistic: TerrainStatistic::default(),
            region_type: RegionType::Undefined,
            starting_tile: Tile::new(usize::MAX),
            start_location_condition: StartLocationCondition::default(),
            luxury_resource: String::new(),
        }
    }

    /// Get the average fertility of the region.
    pub const fn average_fertility(&self) -> f64 {
        self.fertility_sum as f64 / self.tile_count as f64
    }

    /// Get the region of the landmass according to the given `landmass_id`.
    ///
    /// # Notice
    /// We don't need to run `remove_dead_rows_and_columns()` here because the method `obtain_landmass_boundaries()` already did it.
    fn landmass_region(
        tile_map: &TileMap,
        map_parameters: &MapParameters,
        landmass_id: i32,
    ) -> Self {
        let rectangle = tile_map.obtain_landmass_boundaries(map_parameters, landmass_id);

        let fertility_list = tile_map.measure_start_placement_fertility_of_landmass(
            map_parameters,
            landmass_id,
            rectangle,
        );

        Self::new(rectangle, Some(landmass_id), fertility_list)
    }

    fn rectangle_region(
        tile_map: &TileMap,
        map_parameters: &MapParameters,
        rectangle: Rectangle,
    ) -> Self {
        let fertility_list =
            tile_map.measure_start_placement_fertility_in_rectangle(map_parameters, rectangle);

        let mut region = Self::new(rectangle, None, fertility_list);
        region.remove_dead_row_and_column(map_parameters);
        region
    }

    // function AssignStartingPlots:ChopIntoTwoRegions
    /// Divide the region into two smaller regions.
    ///
    /// At first, we check if the region is taller or wider. If it is taller, we divide it into two regions horizontally. If it is wider, we divide it vertically.
    /// The first region will have a fertility sum that is `chop_percent` percent of the total fertility sum of the region.
    /// The second region will have the remaining fertility sum.
    fn chop_into_two_regions(
        &self,
        map_parameters: &MapParameters,
        chop_percent: f32,
    ) -> (Region, Region) {
        let map_height = map_parameters.map_size.height as i32;
        let map_width = map_parameters.map_size.width as i32;

        let taller = self.rectangle.height > self.rectangle.width;

        // Now divide the region.
        let target_fertility = (self.fertility_sum as f32 * chop_percent / 100.) as i32;

        let first_region_west_x = self.rectangle.west_x;
        let first_region_south_y = self.rectangle.south_y;

        // Scope variables that get decided conditionally.
        let first_region_width;
        let first_region_height;
        let second_region_west_x;
        let second_region_south_y;
        let second_region_width;
        let second_region_height;

        let mut first_region_fertility_sum = 0;
        // We don't need to calculate the fertility of the second region,
        // because it will be automatically calculated when we create 'second_region'.
        /* let mut second_region_fertility_sum = 0; */
        let mut first_region_fertility_list = Vec::new();
        let mut second_region_fertility_list = Vec::new();

        if taller {
            first_region_width = self.rectangle.width;
            second_region_west_x = self.rectangle.west_x;
            second_region_width = self.rectangle.width;

            let rect_y = (0..self.rectangle.height)
                .find(|&y| {
                    // Calculate the fertility of the current row
                    let current_row_fertility: i32 = (0..self.rectangle.width)
                        .map(|x| {
                            let fert_index = y * self.rectangle.width + x;
                            let tile_fertility = self.fertility_list[fert_index as usize];
                            // Record this plot's fertility in a new fertility table. (Needed for further subdivisions).
                            first_region_fertility_list.push(tile_fertility);
                            tile_fertility
                        })
                        .sum();

                    // Add the current row's fertility to the total fertility of the first region
                    first_region_fertility_sum += current_row_fertility;

                    // Check if the total fertility of the first region has reached the target fertility
                    first_region_fertility_sum >= target_fertility
                })
                .expect("No suitable row found for chop_into_two_regions");

            first_region_height = rect_y + 1;
            second_region_south_y = (self.rectangle.south_y + first_region_height) % map_height;
            second_region_height = self.rectangle.height - first_region_height;

            second_region_fertility_list
                .reserve((second_region_width * second_region_height) as usize);

            for rect_y in first_region_height..self.rectangle.height {
                for rect_x in 0..self.rectangle.width {
                    let fert_index = rect_y * self.rectangle.width + rect_x;
                    let tile_fertility = self.fertility_list[fert_index as usize];

                    // Record this plot in a new fertility table. (Needed for further subdivisions).
                    second_region_fertility_list.push(tile_fertility);

                    // We don't need to calculate the fertility of the second region,
                    // because it will be automatically calculated when we create 'second_region'.
                    // Add this plot's fertility to the region total so far
                    /* second_region_fertility_sum += tile_fertility; */
                }
            }
        } else {
            first_region_height = self.rectangle.height;
            second_region_south_y = self.rectangle.south_y;
            second_region_height = self.rectangle.height;

            let rect_x = (0..self.rectangle.width)
                .find(|&x| {
                    // Calculate the fertility of the current column
                    let current_column_fertility: i32 = (0..self.rectangle.height)
                        .map(|y| {
                            let fert_index = y * self.rectangle.width + x;
                            self.fertility_list[fert_index as usize]
                        })
                        .sum();

                    // Add the current column's fertility to the region total so far
                    first_region_fertility_sum += current_column_fertility;

                    // Check if the total fertility of the first region has reached the target fertility
                    first_region_fertility_sum >= target_fertility
                })
                .expect("No suitable column found for chop_into_two_regions");

            first_region_width = rect_x + 1;
            second_region_west_x = (self.rectangle.west_x + first_region_width) % map_width;
            second_region_width = self.rectangle.width - first_region_width;

            second_region_fertility_list
                .reserve((second_region_width * second_region_height) as usize);

            // Process the second region
            for rect_y in 0..self.rectangle.height {
                for rect_x in first_region_width..self.rectangle.width {
                    let fert_index = rect_y * self.rectangle.width + rect_x;
                    let tile_fertility = self.fertility_list[fert_index as usize];

                    // Record this plot in a new fertility table. (Needed for further subdivisions).
                    second_region_fertility_list.push(tile_fertility);

                    // We don't need to calculate the fertility of the second region,
                    // because it will be automatically calculated when we create 'second_region'.
                    // Add this plot's fertility to the region total so far
                    /* second_region_fertility_sum += tile_fertility; */
                }
            }

            // Process the first region
            for rect_y in 0..self.rectangle.height {
                for rect_x in 0..first_region_width {
                    let fert_index = rect_y * self.rectangle.width + rect_x;
                    let tile_fertility = self.fertility_list[fert_index as usize];

                    // Record this plot in a new fertility table. (Needed for further subdivisions).
                    first_region_fertility_list.push(tile_fertility);
                }
            }
        }

        let first_region_rectangle = Rectangle {
            west_x: first_region_west_x,
            south_y: first_region_south_y,
            width: first_region_width,
            height: first_region_height,
        };

        let second_region_rectangle = Rectangle {
            west_x: second_region_west_x,
            south_y: second_region_south_y,
            width: second_region_width,
            height: second_region_height,
        };

        let mut first_region = Region::new(
            first_region_rectangle,
            self.landmass_id,
            first_region_fertility_list,
        );

        let mut second_region = Region::new(
            second_region_rectangle,
            self.landmass_id,
            second_region_fertility_list,
        );

        first_region.remove_dead_row_and_column(map_parameters);

        second_region.remove_dead_row_and_column(map_parameters);

        (first_region, second_region)
    }

    // function AssignStartingPlots:ChopIntoThreeRegions
    /// Divides the region into three smaller regions.
    ///
    /// At first, the region is divided into two regions. Then, the second region is divided into two smaller regions.
    /// The fertility of each region is 1/3 of the original region's fertility.
    /// # Notice
    /// We don't need to run `remove_dead_rows_and_columns()` here because the `remove_dead_row_and_column()` function has been called in the `chop_into_two_regions` function.
    fn chop_into_three_regions(&self, map_parameters: &MapParameters) -> (Region, Region, Region) {
        let (first_section_region, remaining_region) =
            self.chop_into_two_regions(map_parameters, 33.3);

        let (second_section_region, third_section_region) =
            remaining_region.chop_into_two_regions(map_parameters, 50.0);

        (
            first_section_region,
            second_section_region,
            third_section_region,
        )
    }

    // function AssignStartingPlots:RemoveDeadRows
    /// Removes the edge rows and columns of the region where all tiles' fertility is 0.
    fn remove_dead_row_and_column(&mut self, map_parameters: &MapParameters) {
        let map_height = map_parameters.map_size.height as i32;
        let map_width = map_parameters.map_size.width as i32;

        // Check for rows to remove on the bottom.
        let mut adjust_south = 0;
        for y in 0..self.rectangle.height {
            // check if the row has any non-zero fertility values
            let keep_this_row = (0..self.rectangle.width).any(|x| {
                let i = (y * self.rectangle.width + x) as usize;
                self.fertility_list[i] != 0
            });

            if keep_this_row {
                break;
            } else {
                adjust_south += 1;
            }
        }

        // Check for rows to remove on the top.
        let mut adjust_north = 0;
        for y in (0..self.rectangle.height).rev() {
            // check if the row has any non-zero fertility values
            let keep_this_row = (0..self.rectangle.width).any(|x| {
                let i = (y * self.rectangle.width + x) as usize;
                self.fertility_list[i] != 0
            });

            if keep_this_row {
                break;
            } else {
                adjust_north += 1;
            }
        }

        // Check for columns to remove on the left.
        let mut adjust_west = 0;
        for x in 0..self.rectangle.width {
            // check if the column has any non-zero fertility values
            let keep_this_column = (0..self.rectangle.height).any(|y| {
                let i = (y * self.rectangle.width + x) as usize;
                self.fertility_list[i] != 0
            });

            if keep_this_column {
                break;
            } else {
                adjust_west += 1;
            }
        }

        // Check for columns to remove on the right.
        let mut adjust_east = 0;
        for x in (0..self.rectangle.width).rev() {
            // check if the column has any non-zero fertility values
            let keep_this_column = (0..self.rectangle.height).any(|y| {
                let i = (y * self.rectangle.width + x) as usize;
                self.fertility_list[i] != 0
            });

            if keep_this_column {
                break;
            } else {
                adjust_east += 1;
            }
        }

        // If adjustments were made, truncate the region.
        if adjust_south > 0 || adjust_north > 0 || adjust_west > 0 || adjust_east > 0 {
            let adjusted_west_x = (self.rectangle.west_x + adjust_west) % map_width;
            let adjusted_south_y = (self.rectangle.south_y + adjust_south) % map_height;
            let adjusted_width = (self.rectangle.width - adjust_west) - adjust_east;
            let adjusted_height = (self.rectangle.height - adjust_south) - adjust_north;

            let mut adjusted_fertility_list =
                Vec::with_capacity((adjusted_width * adjusted_height) as usize);

            for y in 0..adjusted_height {
                for x in 0..adjusted_width {
                    let i =
                        ((y + adjust_south) * self.rectangle.width + (x + adjust_west)) as usize;
                    let tile_fertility = self.fertility_list[i];
                    adjusted_fertility_list.push(tile_fertility);
                }
            }

            // Update region properties.
            // Notice: landmass_id does not need to update.
            self.rectangle = Rectangle {
                west_x: adjusted_west_x,
                south_y: adjusted_south_y,
                width: adjusted_width,
                height: adjusted_height,
            };

            self.fertility_list = adjusted_fertility_list;

            self.fertility_sum = self.fertility_list.iter().sum();

            self.tile_count = self.fertility_list.len() as i32;
        }
    }

    /// Measures the terrain in the region, and sets the [`Region::terrain_statistic`] field.
    ///
    /// Terrain statistics include the num of flatland and hill tiles, the sum of fertility, and the sum of coastal land tiles, .., etc.
    /// When `landmass_id` is `None`, it will ignore the landmass id and measure all the land and water terrain in the region.
    /// Otherwise, it will only measure the terrain which is Water/Mountain or whose `area_id` equal to the region's `landmass_id`.
    pub fn measure_terrain(&mut self, tile_map: &TileMap, map_parameters: &MapParameters) {
        let mut terrain_statistic = TerrainStatistic::default();

        for tile in self.rectangle.iter_tiles(map_parameters) {
            let terrain_type = tile.terrain_type(tile_map);
            let base_terrain = tile.base_terrain(tile_map);
            let feature = tile.feature(tile_map);

            let area_id = tile.area_id(tile_map);

            match terrain_type {
                TerrainType::Mountain => {
                    terrain_statistic.terrain_type_num[terrain_type] += 1;
                }
                TerrainType::Water => {
                    terrain_statistic.terrain_type_num[terrain_type] += 1;

                    terrain_statistic.base_terrain_num[base_terrain] += 1;

                    if let Some(feature) = feature {
                        terrain_statistic.feature_num[feature] += 1;
                    }
                }
                TerrainType::Hill => {
                    if Some(area_id) == self.landmass_id || self.landmass_id.is_none() {
                        terrain_statistic.terrain_type_num[terrain_type] += 1;
                        // We don't need to count the base terrain of hill tiles, because its base terrain bonus is invalid when it is a hill.
                        // For exmple in the original game, if a tile is a hill:
                        // 1. If feature is None:
                        //      (1) When base terrain is not Snow, the tile always produces 2 production.
                        //      (2) When base terrain is Snow, the tile has no output.
                        // 2. If feature is Some, its outpuput is determined by the feature.
                        /* terrain_statistic.base_terrain_num[base_terrain] += 1; */

                        if let Some(feature) = feature {
                            terrain_statistic.feature_num[feature] += 1;
                        }

                        if tile.has_river(tile_map, map_parameters) {
                            terrain_statistic.river_num += 1;
                        }

                        if tile.is_coastal_land(tile_map, map_parameters) {
                            terrain_statistic.coastal_land_num += 1;
                        }

                        // Check if the tile is land and not coastal land, and if it has a neighbor that is coastal land
                        if !tile.is_coastal_land(tile_map, map_parameters)
                            && tile
                                .neighbor_tiles(map_parameters)
                                .iter()
                                .any(|neighbor_tile| {
                                    neighbor_tile.is_coastal_land(tile_map, map_parameters)
                                })
                        {
                            terrain_statistic.next_to_coastal_land_num += 1;
                        }
                    }
                }
                TerrainType::Flatland => {
                    if Some(area_id) == self.landmass_id || self.landmass_id.is_none() {
                        terrain_statistic.terrain_type_num[terrain_type] += 1;

                        terrain_statistic.base_terrain_num[base_terrain] += 1;

                        if let Some(feature) = feature {
                            terrain_statistic.feature_num[feature] += 1;
                        }

                        if tile.has_river(tile_map, map_parameters) {
                            terrain_statistic.river_num += 1;
                        }

                        if tile.is_coastal_land(tile_map, map_parameters) {
                            terrain_statistic.coastal_land_num += 1;
                        }

                        // Check if the tile is land and not coastal land, and if it has a neighbor that is coastal land
                        if !tile.is_coastal_land(tile_map, map_parameters)
                            && tile
                                .neighbor_tiles(map_parameters)
                                .iter()
                                .any(|neighbor_tile| {
                                    neighbor_tile.is_coastal_land(tile_map, map_parameters)
                                })
                        {
                            terrain_statistic.next_to_coastal_land_num += 1;
                        }
                    }
                }
            }
        }

        self.terrain_statistic = terrain_statistic;
    }

    /// Determines region type based on [Region::terrain_statistic] and sets [Region::region_type] field.
    pub fn determine_region_type(&mut self) {
        let terrain_statistic = &self.terrain_statistic;
        let terrain_type_num = &terrain_statistic.terrain_type_num;
        let base_terrain_num = &terrain_statistic.base_terrain_num;
        let feature_num = &terrain_statistic.feature_num;

        // Flatland and hill are the terrain type that cities, mens, and improvements can be built on
        let flatland_and_hill_num = terrain_type_num[TerrainType::Flatland]
            + terrain_statistic.terrain_type_num[TerrainType::Hill];

        if (base_terrain_num[BaseTerrain::Tundra] + base_terrain_num[BaseTerrain::Snow])
            >= flatland_and_hill_num * 30 / 100
        {
            self.region_type = RegionType::Tundra;
        } else if feature_num[Feature::Jungle] >= flatland_and_hill_num * 30 / 100 {
            self.region_type = RegionType::Jungle;
        } else if (feature_num[Feature::Jungle] >= flatland_and_hill_num * 20 / 100)
            && (feature_num[Feature::Jungle] + feature_num[Feature::Forest]
                >= flatland_and_hill_num * 35 / 100)
        {
            self.region_type = RegionType::Jungle;
        } else if feature_num[Feature::Forest] >= flatland_and_hill_num * 30 / 100 {
            self.region_type = RegionType::Forest;
        } else if (feature_num[Feature::Forest] >= flatland_and_hill_num * 20 / 100)
            && (feature_num[Feature::Jungle] + feature_num[Feature::Forest]
                >= flatland_and_hill_num * 35 / 100)
        {
            self.region_type = RegionType::Forest;
        } else if base_terrain_num[BaseTerrain::Desert] >= flatland_and_hill_num * 25 / 100 {
            self.region_type = RegionType::Desert;
        } else if terrain_type_num[TerrainType::Hill] >= flatland_and_hill_num * 415 / 1000 {
            self.region_type = RegionType::Hill;
        } else if (base_terrain_num[BaseTerrain::Plain] >= flatland_and_hill_num * 30 / 100)
            && (base_terrain_num[BaseTerrain::Plain] * 70 / 100
                > base_terrain_num[BaseTerrain::Grassland])
        {
            self.region_type = RegionType::Plain;
        } else if (base_terrain_num[BaseTerrain::Grassland] >= flatland_and_hill_num * 30 / 100)
            && (base_terrain_num[BaseTerrain::Grassland] * 70 / 100
                > base_terrain_num[BaseTerrain::Plain])
        {
            self.region_type = RegionType::Grassland;
        } else if (base_terrain_num[BaseTerrain::Grassland]
            + base_terrain_num[BaseTerrain::Plain]
            + base_terrain_num[BaseTerrain::Desert]
            + base_terrain_num[BaseTerrain::Tundra]
            + base_terrain_num[BaseTerrain::Snow]
            + terrain_type_num[TerrainType::Hill]
            + terrain_type_num[TerrainType::Mountain])
            > flatland_and_hill_num * 80 / 100
        {
            self.region_type = RegionType::Hybrid;
        } else {
            self.region_type = RegionType::Undefined;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Region type.
/// Fields are defined in order of priority, except for [`RegionType::Undefined`].
/// The priority is typically used to sort the regions.
pub enum RegionType {
    Undefined, //-- 0.
    Tundra,    //-- 1.
    Jungle,    //-- 2.
    Forest,    //-- 3.
    Desert,    //-- 4.
    Hill,      //-- 5.
    Plain,     //-- 6.
    Grassland, //-- 7.
    Hybrid,    //-- 8.
}

#[derive(Debug)]
pub struct StartLocationCondition {
    /// Whether the start location is coastal land.
    pub along_ocean: bool,
    /// Whether there is a lake in 2-tile radius of the start location.
    pub next_to_lake: bool,
    /// Whether the start location has a river.
    pub is_river: bool,
    /// Whether there is a river in 2-tile radius of the start location.
    /// Notice: This is only check whether there is a river in 2-tile radius of the start location, not contain the start location itself.
    pub near_river: bool,
    /// Whether there is a mountain in 2-tile radius of the start location.
    pub near_mountain: bool,
    /// The number of forest tiles in 2-tile radius of the start location.
    /// Notice: This is only check the number of forest tiles in 2-tile radius of the start location, not contain the start location itself.
    pub forest_count: i32,
    /// The number of jungle tiles in 2-tile radius of the start location.
    /// Notice: This is only check the number of jungle tiles in 2-tile radius of the start location, not contain the start location itself.
    pub jungle_count: i32,
}

impl Default for StartLocationCondition {
    fn default() -> Self {
        Self {
            along_ocean: false,
            next_to_lake: false,
            is_river: false,
            near_river: false,
            near_mountain: false,
            forest_count: 0,
            jungle_count: 0,
        }
    }
}
