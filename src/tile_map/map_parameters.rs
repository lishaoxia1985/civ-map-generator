use std::time::{SystemTime, UNIX_EPOCH};

use glam::DVec2;

use crate::grid::{
    hex::{HexLayout, HexOrientation, Offset},
    Direction,
};

use super::{tile::Tile, tile_map_impls::generate_regions::Rectangle};

pub struct MapParameters {
    pub name: String,
    pub map_size: MapSize,
    pub map_type: MapType,
    pub hex_layout: HexLayout,
    pub map_wrapping: MapWrapping,
    /// the map use which type of offset coordinate
    pub offset: Offset,
    pub no_ruins: bool,
    pub seed: u64,
    pub large_lake_num: u32,
    /// The max area size of a lake.
    pub lake_max_area_size: u32,
    /// Store the chance of each eligible plot to become a coast in each iteration.
    ///
    /// - Its 'length' is the number of iterations. if 'length' is 3, it means that the max coast length is 4 (3 + 1, because the water tiles adjacent to land must be coast).
    /// - its 'element' is the chance for each eligible plot to become an expansion coast in each iteration. `0.0` means no chance, `1.0` means 100% chance.\
    /// If it is empty the coast will not expand, and then only the water tiles adjacent to land can become coast.
    pub coast_expand_chance: Vec<f64>,
    pub sea_level: SeaLevel,
    pub world_age: WorldAge,
    pub temperature: Temperature,
    pub rainfall: Rainfall,
    /// TODO: In fact, it is related to map size, we don't need to set singlely.
    pub natural_wonder_num: u32,
    /// The number of civilizations, excluding city states.
    pub civilization_num: u32,
    /// The number of city states.
    pub city_state_num: u32,
    pub region_divide_method: RegionDivideMethod,
    /// If true, the civilization starting tile must be coastal land. Otherwise, it can be any hill/flatland tile.
    pub civilization_starting_tile_must_be_coastal_land: bool,
    pub resource_setting: ResourceSetting,
}

#[derive(Clone, Copy)]
pub struct MapSize {
    pub width: i32,
    pub height: i32,
    pub world_size: WorldSize,
}

impl MapSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            world_size: WorldSize::new(width, height),
        }
    }

    pub const fn from_world_size(world_size: WorldSize) -> Self {
        let (width, height) = match world_size {
            WorldSize::Duel => (40, 24),
            WorldSize::Tiny => (56, 36),
            WorldSize::Small => (66, 42),
            WorldSize::Standard => (80, 52),
            WorldSize::Large => (104, 64),
            WorldSize::Huge => (128, 80),
        };

        MapSize {
            width,
            height,
            world_size,
        }
    }
}

#[derive(Clone, Copy)]
pub enum WorldSize {
    Duel,
    Tiny,
    Small,
    Standard,
    Large,
    Huge,
}

impl WorldSize {
    // Function to determine the WorldSize based on the width and height product
    fn new(width: i32, height: i32) -> WorldSize {
        let area = width * height;
        match area {
            // When area <= 40 * 24, set the WorldSize to "Duel" and give a warning message
            area if area < 960 => {
                eprintln!(
                    "The map size is too small. The provided dimensions are {}x{}, which gives an area of {}. The minimum area is 40 * 24 = 960 in the original CIV5 game.",
                    width, height, area
                );
                WorldSize::Duel
            }
            // For "Duel" size: area <= 56 * 36
            area if area < 2016 => WorldSize::Duel,
            // For "Tiny" size: area <= 66 * 42
            area if area < 2772 => WorldSize::Tiny,
            // For "Small" size: area <= 80 * 52
            area if area < 4160 => WorldSize::Small,
            // For "Standard" size: area <= 104 * 64
            area if area < 6656 => WorldSize::Standard,
            // For "Large" size: area <= 128 * 80
            area if area < 10240 => WorldSize::Large,
            // For "Huge" size: area >= 128 * 80
            _ => WorldSize::Huge,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
/// The map wrapping settings.
pub struct MapWrapping {
    /// - If `x` is `WrapType::Wrap`, the map will wrap around in the x direction.
    /// - If `x` is `WrapType::Polar`, the left and right edges of the map will be all water tiles.
    /// - If `x` is `WrapType::None`, the map will not wrap around in the x direction and the edges of the map will not be processed.
    pub x: WrapType,
    /// - If `y` is `WrapType::Wrap`, the map will wrap around in the y direction.
    /// - If `y` is `WrapType::Polar`, the top and bottom edges of the map will be all water tiles.
    /// - If `y` is `WrapType::None`, the map will not wrap around in the y direction and the edges of the map will not be processed.
    pub y: WrapType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WrapType {
    /// The map will not wrap around.
    None,
    /// The map will wrap around.
    Wrap,
    /// The edges of the map will be all water tiles.
    Polar,
}

pub enum MapType {
    Fractal,
    Pangaea,
}

pub enum SeaLevel {
    Low,
    Normal,
    High,
    Random,
}

pub enum WorldAge {
    /// 5 Billion Years
    Old,
    /// 4 Billion Years
    Normal,
    /// 3 Billion Years
    New,
}

pub enum Temperature {
    Cool,
    Normal,
    Hot,
}

pub enum Rainfall {
    Arid,
    Normal,
    Wet,
    Random,
}

/// Defines the method used to divide regions for civilizations in the game. This enum is used to determine how civilizations are assigned to different regions on the map.
pub enum RegionDivideMethod {
    /// All civilizations start on the biggest landmass.
    ///
    /// This method places all civs on a single, largest landmass.
    Pangaea,
    /// Civs are assigned to continents. Any continents with more than one civ are divided.
    Continent,
    /// This method is primarily used for Archipelago or other maps with many small islands.
    /// The entire map is treated as one large rectangular region.
    /// The `WholeMapRectangle` method is equivalent to `CustomRectangle(Rectangle)` when the `Rectangle` encompasses the entire map area.
    /// We will ignore the landmass id when method is set to WholeMapRectangle.
    WholeMapRectangle,
    /// Civs start within a custom-defined rectangle.
    /// We will ignore the landmass id when method is set to CustomRectangle.
    CustomRectangle(Rectangle),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResourceSetting {
    /// 1
    Sparse,
    /// 2
    Standard,
    /// 3
    Abundant,
    /// 4
    LegendaryStart,
    /// 5
    StrategicBalance,
}

impl Default for MapParameters {
    fn default() -> Self {
        let mut map_parameters = Self {
            name: "perlin map".to_owned(),
            map_size: MapSize::from_world_size(WorldSize::Standard),
            map_type: MapType::Fractal,
            hex_layout: HexLayout {
                orientation: HexOrientation::Flat,
                size: DVec2::new(8., 8.),
                origin: DVec2::new(0., 0.),
            },
            map_wrapping: MapWrapping {
                x: WrapType::Wrap,
                y: WrapType::Polar,
            },
            offset: Offset::Odd,
            no_ruins: false,
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .try_into()
                .unwrap(),
            large_lake_num: 2,
            lake_max_area_size: 9,
            coast_expand_chance: vec![0.25, 0.25],
            sea_level: SeaLevel::Normal,
            world_age: WorldAge::Normal,
            temperature: Temperature::Normal,
            rainfall: Rainfall::Normal,
            natural_wonder_num: 6,
            civilization_num: 4,
            city_state_num: 8,
            region_divide_method: RegionDivideMethod::Continent,
            civilization_starting_tile_must_be_coastal_land: false,
            resource_setting: ResourceSetting::Standard,
        };
        map_parameters.update_origin();
        map_parameters
    }
}

impl MapParameters {
    pub fn update_origin(&mut self) {
        let width = self.map_size.width;
        let height = self.map_size.height;

        let (min_offset_x, min_offset_y) = [0, 1, width].into_iter().fold(
            (0.0_f64, 0.0_f64),
            |(min_offset_x, min_offset_y), index| {
                let hex = Tile::new(index as usize).to_hex_coordinate(self);

                let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
                (min_offset_x.min(offset_x), min_offset_y.min(offset_y))
            },
        );

        let (max_offset_x, max_offset_y) = [
            width * (height - 1) - 1,
            width * height - 2,
            width * height - 1,
        ]
        .into_iter()
        .fold((0.0_f64, 0.0_f64), |(max_offset_x, max_offset_y), index| {
            let hex = Tile::new(index as usize).to_hex_coordinate(self);

            let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
            (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
        });

        self.hex_layout.origin =
            -(DVec2::new(min_offset_x, min_offset_y) + DVec2::new(max_offset_x, max_offset_y)) / 2.;
    }

    pub const fn edge_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.edge_direction()
    }

    pub const fn corner_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.corner_direction()
    }
}
