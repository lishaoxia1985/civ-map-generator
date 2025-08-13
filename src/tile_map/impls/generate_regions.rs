use std::cmp::min;

use enum_map::{enum_map, EnumMap};
use serde::{Deserialize, Serialize};

use crate::{
    grid::{hex_grid::HexGrid, offset_coordinate::OffsetCoordinate, WrapFlags},
    map_parameters::{Rectangle, RegionDivideMethod},
    tile::Tile,
    tile_component::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType},
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    // function AssignStartingPlots:GenerateRegions(args)
    /// Generates regions for the map according civilization number and region divide method.
    ///
    /// The number of regions is equal to the number of civilizations.
    pub fn generate_regions(&mut self, map_parameters: &MapParameters) {
        let grid = self.world_grid.grid;

        let civilization_num = map_parameters.civilization_num;

        match map_parameters.region_divide_method {
            RegionDivideMethod::Pangaea => {
                // -- Identify the biggest landmass.
                let biggest_landmass_id = self.get_biggest_area_id();

                let landmass_region = Region::landmass_region(self, biggest_landmass_id);

                self.divide_into_regions(civilization_num, landmass_region);
            }
            RegionDivideMethod::Continent => {
                let mut landmass_region_list: Vec<_> = self
                    .area_list
                    .iter()
                    .filter(|area| !area.is_water)
                    .map(|area| Region::landmass_region(self, area.id))
                    .collect();

                landmass_region_list.sort_by_key(|region| region.fertility_sum);

                let landmass_num = landmass_region_list.len() as u32;

                // If less players than landmasses, we will ignore the extra landmasses.
                let relevant_landmass_num = min(landmass_num, civilization_num);

                // Create a new list containing the most fertile land areas by reversing the sorted list and selecting the top `relevant_landmass_num` items.
                let best_landmass_region_list = landmass_region_list
                    .into_iter()
                    .rev() // Reverse the iterator so the most fertile regions (which are at the end of the sorted list) come first.
                    .take(relevant_landmass_num as usize) // Take the top `relevant_landmass_num` elements from the reversed list.
                    .collect::<Vec<_>>();

                let mut number_of_civs_on_landmass = vec![0; relevant_landmass_num as usize];

                // Calculate how to distribute civilizations across regions based on fertility
                // The goal is to place civilizations where the fertility per civ is highest

                // First, create a list tracking each region's total fertility (initial average when 1 civ is placed)
                let mut average_fertility_per_civ: Vec<f64> = best_landmass_region_list
                    .iter()
                    .map(|region| region.fertility_sum as f64)
                    .collect();

                // Distribute all civilizations one by one
                for _ in 0..civilization_num {
                    // Find the most fertile region (where adding a civ would give highest fertility per civ)
                    let (best_index, _) = average_fertility_per_civ
                        .iter()
                        .enumerate()
                        .max_by(|&(_, a), &(_, b)| a.total_cmp(b))
                        .expect("Should always find a region - empty list checked earlier");

                    // Place one civilization in this best region
                    number_of_civs_on_landmass[best_index] += 1;

                    // Update this region's fertility-per-civ value:
                    // Divide total fertility by (current civ count + 1) to represent what the average would be if we add another civ
                    average_fertility_per_civ[best_index] =
                        best_landmass_region_list[best_index].fertility_sum as f64
                            / (number_of_civs_on_landmass[best_index] as f64 + 1.);
                }

                for (index, region) in best_landmass_region_list.into_iter().enumerate() {
                    if number_of_civs_on_landmass[index] > 0 {
                        self.divide_into_regions(number_of_civs_on_landmass[index], region);
                    }
                }
            }
            RegionDivideMethod::WholeMapRectangle => {
                let rectangle = Rectangle::new(
                    OffsetCoordinate::new(0, 0),
                    grid.size.width,
                    grid.size.height,
                    grid,
                );

                let region = Region::rectangle_region(self, grid, rectangle);
                self.divide_into_regions(civilization_num, region);
            }
            RegionDivideMethod::CustomRectangle(rectangle) => {
                let region = Region::rectangle_region(self, grid, rectangle);
                self.divide_into_regions(civilization_num, region);
            }
        }
    }

    // function AssignStartingPlots:DivideIntoRegions
    /// Divides the region into subdivisions and get a vec of the subdivisions region.
    ///
    /// # Arguments
    ///
    /// - `divisions_num`: The number of divisions to make. In origin code, this should <= 22.
    /// - `region`: The region to divide.
    fn divide_into_regions(&mut self, divisions_num: u32, region: Region) {
        let grid = self.world_grid.grid;

        let mut stack = vec![(region, divisions_num)];

        while let Some((mut current_region, current_divisions_num)) = stack.pop() {
            match current_divisions_num {
                1 => {
                    // If we have only one division, it does not need to be divided further. So we just add it to the region list.
                    current_region.measure_terrain(self);
                    current_region.determine_region_type();
                    self.region_list.push(current_region);
                }
                2 => {
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, 50.0);
                    stack.push((first_section, 1));
                    stack.push((second_section, 1));
                }
                3 => {
                    let (first_section, second_section, third_section) =
                        current_region.chop_into_three_regions(grid);
                    stack.push((first_section, 1));
                    stack.push((second_section, 1));
                    stack.push((third_section, 1));
                }
                5 => {
                    let chop_percent = 3. / 5. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 3));
                    stack.push((second_section, 2));
                }
                7 => {
                    let chop_percent = 3. / 7. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 3));
                    stack.push((second_section, 4));
                }
                11 => {
                    let chop_percent = 3. / 11. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 3));
                    stack.push((second_section, 8));
                }
                13 => {
                    let chop_percent = 5. / 13. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 5));
                    stack.push((second_section, 8));
                }
                17 => {
                    let chop_percent = 9. / 17. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 9));
                    stack.push((second_section, 8));
                }
                19 => {
                    let chop_percent = 7. / 19. * 100.0;
                    let (first_section, second_section) =
                        current_region.chop_into_two_regions(grid, chop_percent);
                    stack.push((first_section, 7));
                    stack.push((second_section, 12));
                }
                _ => {
                    if current_divisions_num % 3 == 0 {
                        let subdivisions = current_divisions_num / 3;
                        let (first_section, second_section, third_section) =
                            current_region.chop_into_three_regions(grid);
                        stack.push((first_section, subdivisions));
                        stack.push((second_section, subdivisions));
                        stack.push((third_section, subdivisions));
                    } else if current_divisions_num % 2 == 0 {
                        let subdivisions = current_divisions_num / 2;
                        let (first_section, second_section) =
                            current_region.chop_into_two_regions(grid, 50.0);
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

    // function AssignStartingPlots:MeasureStartPlacementFertilityOfLandmass
    /// Returns a list of fertility values for all tiles in the landmass rectangle.
    fn measure_start_placement_fertility_of_landmass(
        &self,
        area_id: usize,
        landmass_rectangle: Rectangle,
    ) -> Vec<i32> {
        let tile_count = landmass_rectangle.width() * landmass_rectangle.height();

        let mut area_fertility_list = Vec::with_capacity(tile_count as usize);

        for tile in landmass_rectangle.all_tiles(self.world_grid.grid) {
            if tile.area_id(self) != area_id {
                area_fertility_list.push(0);
            } else {
                let tile_fertility = self.measure_start_placement_fertility_of_tile(tile, true);
                area_fertility_list.push(tile_fertility);
            }
        }

        area_fertility_list
    }

    // function AssignStartingPlots:MeasureStartPlacementFertilityInRectangle
    /// Returns a list of fertility values for all tiles in the rectangle.
    fn measure_start_placement_fertility_in_rectangle(&self, rectangle: Rectangle) -> Vec<i32> {
        let tile_count = rectangle.width() * rectangle.height();

        let mut area_fertility_list = Vec::with_capacity(tile_count as usize);

        for tile in rectangle.all_tiles(self.world_grid.grid) {
            // Check for coastal land is disabled.
            let tile_fertility = self.measure_start_placement_fertility_of_tile(tile, false);
            area_fertility_list.push(tile_fertility);
        }

        area_fertility_list
    }

    // function AssignStartingPlots:MeasureStartPlacementFertilityOfPlot
    /// Returns the fertility of a tile for starting placement.
    fn measure_start_placement_fertility_of_tile(
        &self,
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

        if tile.has_river(self) {
            tile_fertility += 1;
        }

        if tile.is_freshwater(self) {
            tile_fertility += 1;
        }

        if check_for_coastal_land && tile.is_coastal_land(self) {
            tile_fertility += 2;
        }

        tile_fertility
    }

    /// Get landmass rectangle for the region.
    fn obtain_landmass_boundaries(&self, area_id: usize) -> Rectangle {
        let grid = self.world_grid.grid;
        let map_height = grid.size.height;
        let map_width = grid.size.width;
        // -- Set up variables that will be returned by this function.
        let mut wrap_x = false;
        let mut wrap_y = false;
        let mut west_x = 0;
        let mut east_x = 0;
        let mut south_y = 0;
        let mut north_y = 0;

        // Check if the landmass wraps around the map horizontally.
        // Check if the first and last columns of the map contain tiles that belong to the area.
        // If so, the landmass wraps around the map horizontally.
        // If not, the landmass does not wrap around the map horizontally.
        if grid.wrap_flags.contains(WrapFlags::WrapX) {
            wrap_x = (0..map_height).any(|y| {
                let first_column_tile = Tile::from_offset(OffsetCoordinate::from([0, y]), grid);
                first_column_tile.area_id(self) == area_id
            }) && (0..map_height).any(|y| {
                let last_column_tile =
                    Tile::from_offset(OffsetCoordinate::from([map_width - 1, y]), grid);
                last_column_tile.area_id(self) == area_id
            });
        }

        // Check if the landmass wraps around the map vertically.
        // Check if the first and last rows of the map contain tiles that belong to the area.
        // If so, the landmass wraps around the map vertically.
        // If not, the landmass does not wrap around the map vertically.
        if grid.wrap_flags.contains(WrapFlags::WrapY) {
            wrap_y = (0..map_width).any(|x| {
                let first_row_tile = Tile::from_offset(OffsetCoordinate::from([x, 0]), grid);
                first_row_tile.area_id(self) == area_id
            }) && (0..map_width).any(|x| {
                let last_row_tile =
                    Tile::from_offset(OffsetCoordinate::from([x, map_height - 1]), grid);
                last_row_tile.area_id(self) == area_id
            });
        }

        // Find West and East edges of this landmass.
        if !wrap_x {
            // If the landmass does not wrap around the map horizontally.
            // Check for any area membership one column at a time, left to right.
            for x in 0..map_width {
                if (0..map_height).any(|y| {
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
                    tile.area_id(self) == area_id
                }) {
                    west_x = x;
                    break;
                }
            }

            // Check for any area membership one column at a time, right to left.
            for x in (0..map_width).rev() {
                if (0..map_height).any(|y| {
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
                    tile.area_id(self) == area_id
                }) {
                    south_y = y;
                    break;
                }
            }

            // Check for any area membership one row at a time, top to bottom.
            for y in (0..map_height).rev() {
                if (0..map_width).any(|x| {
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
                    let offset_coordinate = OffsetCoordinate::from([x, y]);
                    let tile = Tile::from_offset(offset_coordinate, grid);
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
            east_x + map_width - west_x + 1
        } else {
            east_x - west_x + 1
        };

        let height = if wrap_y {
            north_y + map_height - south_y + 1
        } else {
            north_y - south_y + 1
        };

        Rectangle::new(
            OffsetCoordinate::from([west_x, south_y]),
            width,
            height,
            grid,
        )
    }

    /// Get the biggest AreaID.
    fn get_biggest_area_id(&self) -> usize {
        self.area_list
            .iter()
            .filter(|area| !area.is_water)
            .max_by_key(|area| area.size)
            .expect("No area found!") // Ensure that there's at least one area.
            .id
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
pub struct Region {
    /// The rectangle that defines the region.
    pub rectangle: Rectangle,
    /// The area ID of the landmass this region belongs to.
    /// When landmass_id is `None`, it means that we will consider all landmass in the region when we divide it into sub-regions or other operations.
    pub area_id: Option<usize>,
    /// List of fertility values for each tile in the region.
    ///
    /// In the edge rows and edge columns of a region, it is not allowed for all tiles' fertility to be 0.
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
    /// The exclusive luxury resource of the region.
    ///
    /// In CIV5, this same luxury resource can only be found in at most 3 regions on the map.
    ///
    /// When we run [`TileMap::assign_luxury_roles`], this luxury resource must be in [`TileMap::luxury_resource_role`]'s `luxury_assigned_to_regions` field.
    pub exclusive_luxury: String,
}

impl Region {
    fn new(rectangle: Rectangle, landmass_id: Option<usize>, fertility_list: Vec<i32>) -> Self {
        let fertility_sum = fertility_list.iter().sum();
        let tile_count = fertility_list.len() as i32;

        Region {
            rectangle,
            area_id: landmass_id,
            fertility_list,
            fertility_sum,
            tile_count,
            terrain_statistic: TerrainStatistic::default(),
            region_type: RegionType::Undefined,
            starting_tile: Tile::new(usize::MAX),
            start_location_condition: StartLocationCondition::default(),
            exclusive_luxury: String::new(),
        }
    }

    /// Get the average fertility of the region.
    pub const fn average_fertility(&self) -> f64 {
        self.fertility_sum as f64 / self.tile_count as f64
    }

    /// Get the region of the landmass according to the given `landmass_id`.
    ///
    /// # Notice
    ///
    /// We don't need to call [`Region::remove_dead_row_and_column()`] in this function,
    /// because [`TileMap::obtain_landmass_boundaries()`] has already ensured that there are no dead rows and columns in the rectangle.
    fn landmass_region(tile_map: &TileMap, landmass_id: usize) -> Self {
        let rectangle = tile_map.obtain_landmass_boundaries(landmass_id);

        let fertility_list =
            tile_map.measure_start_placement_fertility_of_landmass(landmass_id, rectangle);

        Self::new(rectangle, Some(landmass_id), fertility_list)
    }

    fn rectangle_region(tile_map: &TileMap, grid: HexGrid, rectangle: Rectangle) -> Self {
        let fertility_list = tile_map.measure_start_placement_fertility_in_rectangle(rectangle);

        let mut region = Self::new(rectangle, None, fertility_list);
        region.remove_dead_row_and_column(grid);
        region
    }

    // function AssignStartingPlots:ChopIntoTwoRegions
    /// Divide the region into two smaller regions.
    ///
    /// At first, we check if the region is taller or wider. If it is taller, we divide it into two regions horizontally. If it is wider, we divide it vertically.
    /// The first region will have a fertility sum that is `chop_percent` percent of the total fertility sum of the region.
    /// The second region will have the remaining fertility sum.
    fn chop_into_two_regions(&self, grid: HexGrid, chop_percent: f32) -> (Region, Region) {
        let taller = self.rectangle.height() > self.rectangle.width();

        // Now divide the region.
        let target_fertility = (self.fertility_sum as f32 * chop_percent / 100.) as i32;

        let first_region_west_x = self.rectangle.west_x();
        let first_region_south_y = self.rectangle.south_y();

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
            first_region_width = self.rectangle.width();
            second_region_west_x = self.rectangle.west_x();
            second_region_width = self.rectangle.width();

            let rect_y = (0..self.rectangle.height())
                .find(|&y| {
                    // Calculate the fertility of the current row
                    let current_row_fertility: i32 = (0..self.rectangle.width())
                        .map(|x| {
                            let fert_index = y * self.rectangle.width() + x;
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
            second_region_south_y = self.rectangle.south_y() + first_region_height as i32;
            second_region_height = self.rectangle.height() - first_region_height;

            second_region_fertility_list
                .reserve((second_region_width * second_region_height) as usize);

            for rect_y in first_region_height..self.rectangle.height() {
                for rect_x in 0..self.rectangle.width() {
                    let fert_index = rect_y * self.rectangle.width() + rect_x;
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
            first_region_height = self.rectangle.height();
            second_region_south_y = self.rectangle.south_y();
            second_region_height = self.rectangle.height();

            let rect_x = (0..self.rectangle.width())
                .find(|&x| {
                    // Calculate the fertility of the current column
                    let current_column_fertility: i32 = (0..self.rectangle.height())
                        .map(|y| {
                            let fert_index = y * self.rectangle.width() + x;
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
            second_region_west_x = self.rectangle.west_x() + first_region_width as i32;
            second_region_width = self.rectangle.width() - first_region_width;

            second_region_fertility_list
                .reserve((second_region_width * second_region_height) as usize);

            // Process the second region
            for rect_y in 0..self.rectangle.height() {
                for rect_x in first_region_width..self.rectangle.width() {
                    let fert_index = rect_y * self.rectangle.width() + rect_x;
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
            for rect_y in 0..self.rectangle.height() {
                for rect_x in 0..first_region_width {
                    let fert_index = rect_y * self.rectangle.width() + rect_x;
                    let tile_fertility = self.fertility_list[fert_index as usize];

                    // Record this plot in a new fertility table. (Needed for further subdivisions).
                    first_region_fertility_list.push(tile_fertility);
                }
            }
        }

        let first_region_rectangle = Rectangle::new(
            OffsetCoordinate::new(first_region_west_x, first_region_south_y),
            first_region_width,
            first_region_height,
            grid,
        );

        let second_region_rectangle = Rectangle::new(
            OffsetCoordinate::new(second_region_west_x, second_region_south_y),
            second_region_width,
            second_region_height,
            grid,
        );

        let mut first_region = Region::new(
            first_region_rectangle,
            self.area_id,
            first_region_fertility_list,
        );

        let mut second_region = Region::new(
            second_region_rectangle,
            self.area_id,
            second_region_fertility_list,
        );

        first_region.remove_dead_row_and_column(grid);

        second_region.remove_dead_row_and_column(grid);

        (first_region, second_region)
    }

    // function AssignStartingPlots:ChopIntoThreeRegions
    /// Divides the region into three smaller regions.
    ///
    /// At first, the region is divided into two regions. Then, the second region is divided into two smaller regions.
    /// The fertility of each region is 1/3 of the original region's fertility.
    ///
    /// # Notice
    ///
    /// We don't need to call [`Region::remove_dead_row_and_column`] in this function,
    /// because the function has been called in [`Region::chop_into_two_regions`] function.
    fn chop_into_three_regions(&self, grid: HexGrid) -> (Region, Region, Region) {
        let (first_section_region, remaining_region) = self.chop_into_two_regions(grid, 33.3);

        let (second_section_region, third_section_region) =
            remaining_region.chop_into_two_regions(grid, 50.0);

        (
            first_section_region,
            second_section_region,
            third_section_region,
        )
    }

    // function AssignStartingPlots:RemoveDeadRows
    /// Removes the edge rows and columns of the region where all tiles' fertility is 0.
    fn remove_dead_row_and_column(&mut self, grid: HexGrid) {
        let width = self.rectangle.width();
        let height = self.rectangle.height();

        // Calculate the number of rows which need to be removed from the south edge.
        let adjust_south = (0..height)
            .take_while(|&y| (0..width).all(|x| self.fertility_list[(y * width + x) as usize] == 0))
            .count();

        // Calculate the number of rows which need to be removed from the north edge.
        let adjust_north = (0..height)
            .rev()
            .take_while(|&y| (0..width).all(|x| self.fertility_list[(y * width + x) as usize] == 0))
            .count();

        // Calculate the number of columns which need to be removed from the west edge.
        let adjust_west = (0..width)
            .take_while(|&x| {
                (0..height).all(|y| self.fertility_list[(y * width + x) as usize] == 0)
            })
            .count();

        // Calculate the number of columns which need to be removed from the east edge.
        let adjust_east = (0..width)
            .rev()
            .take_while(|&x| {
                (0..height).all(|y| self.fertility_list[(y * width + x) as usize] == 0)
            })
            .count();

        // Early return if no adjustments needed
        if adjust_south == 0 && adjust_north == 0 && adjust_west == 0 && adjust_east == 0 {
            return;
        }

        let adjusted_west_x = self.rectangle.west_x() + adjust_west as i32;
        let adjusted_south_y = self.rectangle.south_y() + adjust_south as i32;
        let adjusted_width = (self.rectangle.width() - adjust_west as u32) - adjust_east as u32;
        let adjusted_height = (self.rectangle.height() - adjust_south as u32) - adjust_north as u32;

        let fertility_list = &self.fertility_list;

        let adjusted_fertility_list = (0..adjusted_height)
            .flat_map(|y| {
                let row_start =
                    (y + adjust_south as u32) * self.rectangle.width() + adjust_west as u32;
                (0..adjusted_width).map(move |x| fertility_list[(row_start + x) as usize])
            })
            .collect::<Vec<_>>();

        // Update region properties.
        // # Notice
        // - `landmass_id` does not need to update.
        // - `fertility_sum` does not need to update, because we removed the rows and columns with 0 fertility,
        //   so the fertility sum will not change.
        self.rectangle = Rectangle::new(
            OffsetCoordinate::new(adjusted_west_x, adjusted_south_y),
            adjusted_width,
            adjusted_height,
            grid,
        );

        self.fertility_list = adjusted_fertility_list;

        self.tile_count = self.fertility_list.len() as i32;
    }

    /// Measures the terrain in the region, and sets the [`Region::terrain_statistic`] field.
    ///
    /// Terrain statistics include the num of flatland and hill tiles, the sum of fertility, and the sum of coastal land tiles, .., etc.
    /// When `landmass_id` is `None`, it will ignore the landmass ID and measure all the land and water terrain in the region.
    /// Otherwise, it will only measure the terrain which is Water/Mountain or whose `area_id` equal to the region's `landmass_id`.
    pub fn measure_terrain(&mut self, tile_map: &TileMap) {
        let grid = tile_map.world_grid.grid;

        let mut terrain_statistic = TerrainStatistic::default();

        for tile in self.rectangle.all_tiles(grid) {
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
                    if Some(area_id) == self.area_id || self.area_id.is_none() {
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

                        if tile.has_river(tile_map) {
                            terrain_statistic.river_num += 1;
                        }

                        if tile.is_coastal_land(tile_map) {
                            terrain_statistic.coastal_land_num += 1;
                        }

                        // Check if the tile is land and not coastal land, and if it has a neighbor that is coastal land
                        if !tile.is_coastal_land(tile_map)
                            && tile
                                .neighbor_tiles(grid)
                                .any(|neighbor_tile| neighbor_tile.is_coastal_land(tile_map))
                        {
                            terrain_statistic.next_to_coastal_land_num += 1;
                        }
                    }
                }
                TerrainType::Flatland => {
                    if Some(area_id) == self.area_id || self.area_id.is_none() {
                        terrain_statistic.terrain_type_num[terrain_type] += 1;

                        terrain_statistic.base_terrain_num[base_terrain] += 1;

                        if let Some(feature) = feature {
                            terrain_statistic.feature_num[feature] += 1;
                        }

                        if tile.has_river(tile_map) {
                            terrain_statistic.river_num += 1;
                        }

                        if tile.is_coastal_land(tile_map) {
                            terrain_statistic.coastal_land_num += 1;
                        }

                        // Check if the tile is land and not coastal land, and if it has a neighbor that is coastal land
                        if !tile.is_coastal_land(tile_map)
                            && tile
                                .neighbor_tiles(grid)
                                .any(|neighbor_tile| neighbor_tile.is_coastal_land(tile_map))
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
        } else if feature_num[Feature::Jungle] >= flatland_and_hill_num * 30 / 100
            || ((feature_num[Feature::Jungle] >= flatland_and_hill_num * 20 / 100)
                && (feature_num[Feature::Jungle] + feature_num[Feature::Forest]
                    >= flatland_and_hill_num * 35 / 100))
        {
            self.region_type = RegionType::Jungle;
        } else if feature_num[Feature::Forest] >= flatland_and_hill_num * 30 / 100
            || ((feature_num[Feature::Forest] >= flatland_and_hill_num * 20 / 100)
                && (feature_num[Feature::Jungle] + feature_num[Feature::Forest]
                    >= flatland_and_hill_num * 35 / 100))
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
///
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

#[derive(Debug, Default)]
pub struct StartLocationCondition {
    /// Whether the start location is coastal land.
    pub along_ocean: bool,
    /// Whether there is a lake in 2-tile radius of the start location.
    pub next_to_lake: bool,
    /// Whether the start location has a river.
    pub is_river: bool,
    /// Whether there is a river in 2-tile radius of the start location.
    /// NOTICE: This is only check whether there is a river in 2-tile radius of the start location, not contain the start location itself.
    pub near_river: bool,
    /// Whether there is a mountain in 2-tile radius of the start location.
    pub near_mountain: bool,
    /// The number of forest tiles in 2-tile radius of the start location.
    /// NOTICE: This is only check the number of forest tiles in 2-tile radius of the start location, not contain the start location itself.
    pub forest_count: i32,
    /// The number of jungle tiles in 2-tile radius of the start location.
    /// NOTICE: This is only check the number of jungle tiles in 2-tile radius of the start location, not contain the start location itself.
    pub jungle_count: i32,
}
