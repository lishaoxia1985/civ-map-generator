use std::time::{SystemTime, UNIX_EPOCH};

use glam::Vec2;

use crate::{
    grid::{
        hex_grid::{
            hex::{HexLayout, HexOrientation, Offset},
            HexGrid,
        },
        offset_coordinate::OffsetCoordinate,
        Grid, Size, WrapFlags,
    },
    tile::Tile,
};

pub struct MapParameters {
    pub map_type: MapType,
    pub world_grid: WorldGrid,
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
    /// The number of civilizations, excluding city states.
    pub civilization_num: u32,
    /// The number of city states.
    pub city_state_num: u32,
    pub region_divide_method: RegionDivideMethod,
    /// If true, the civilization starting tile must be coastal land. Otherwise, it can be any hill/flatland tile.
    pub civilization_starting_tile_must_be_coastal_land: bool,
    pub resource_setting: ResourceSetting,
}

/// Represents a game world composed of grids.
///
/// Combines physical grid representation with logical world size classification
/// for map generation and game scaling purposes.
///
/// # Instantiation
///
/// `WorldGrid` instances can only be created through two supported methods:
///
/// 1. [`WorldGrid::from_grid`] constructor - Creates from a custom-sized grid,
///    automatically determining the [`WorldSize`] based on grid dimensions:
/// ```rust
/// use civ_map_generator::grid::*;
/// use civ_map_generator::grid::hex_grid::*;
/// use civ_map_generator::map_parameters::*;
/// use glam::Vec2;
///
/// let grid = HexGrid {
///     size: Size { width: 80, height: 40 },
///     hex_layout: HexLayout {
///         orientation: HexOrientation::Flat,
///         size: Vec2::new(8., 8.),
///         origin: Vec2::new(0., 0.),
///     },
///     wrap_flags: WrapFlags::WrapX,
///     offset: Offset::Odd,
/// };
///
/// let world_grid = WorldGrid::from_grid(grid);
/// ```
///
/// 2. Explicit [`WorldSize`] specification - Creates with default grid dimensions
///    for a standardized world size:
/// ```rust
/// use civ_map_generator::grid::*;
/// use civ_map_generator::grid::hex_grid::*;
/// use civ_map_generator::map_parameters::*;
/// use glam::Vec2;
///
/// let world_size = WorldSize::Standard;
/// // Create a new HexGrid with 0 dimensions.
/// let mut grid = HexGrid {
///    size: Size { width: 0, height: 0 },
///    hex_layout: HexLayout {
///        orientation: HexOrientation::Flat,
///        size: Vec2::new(8., 8.),
///        origin: Vec2::new(0., 0.),
///    },
///    wrap_flags: WrapFlags::WrapX,
///    offset: Offset::Odd,
/// };
///
/// // Sets default dimensions based on world size classification
/// grid.set_default_size(world_size);
///
/// let world_grid = WorldGrid {
///     grid,
///     world_size,
/// };
/// ```
///
#[derive(Clone, Copy)]
pub struct WorldGrid {
    pub grid: HexGrid,
    pub world_size: WorldSize,
}

impl WorldGrid {
    pub fn from_grid(grid: HexGrid) -> Self {
        let world_size = grid.get_world_size();
        Self { grid, world_size }
    }

    /// Get the size of the grid.
    pub fn size(&self) -> Size {
        self.grid.size
    }

    /// Get the world size of the grid.
    pub fn world_size(&self) -> WorldSize {
        self.world_size
    }
}

/// Defines standard world size presets for game maps or environments.
///
/// Variants represent different scale levels from smallest to largest.
#[derive(Clone, Copy)]
pub enum WorldSize {
    Duel,
    Tiny,
    Small,
    Standard,
    Large,
    Huge,
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
    /// [`RegionDivideMethod::WholeMapRectangle`] is equivalent to [`RegionDivideMethod::CustomRectangle()`] when [`Rectangle`] encompasses the entire map area.
    /// We will ignore the landmass ID when method is set to WholeMapRectangle.
    WholeMapRectangle,
    /// Civs start within a custom-defined rectangle.
    /// We will ignore the landmass ID when method is set to CustomRectangle.
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
        let world_size = WorldSize::Standard;
        let mut grid = HexGrid {
            size: Size {
                width: 0,
                height: 0,
            },
            layout: HexLayout {
                orientation: HexOrientation::Flat,
                size: Vec2::new(8., 8.),
                origin: Vec2::new(0., 0.),
            },
            wrap_flags: WrapFlags::WrapX,
            offset: Offset::Odd,
        };
        grid.set_default_size(world_size);

        let world_grid = WorldGrid { grid, world_size };
        Self {
            map_type: MapType::Fractal,
            world_grid,
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
            civilization_num: 4,
            city_state_num: 8,
            region_divide_method: RegionDivideMethod::Continent,
            civilization_starting_tile_must_be_coastal_land: false,
            resource_setting: ResourceSetting::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Defines a rectangular region within a tile-based map coordinate system.
///
/// Provides functionality to retrieve all tiles contained within the region.
pub struct Rectangle {
    /// The origin point in offset coordinates.
    ///
    /// # Grid Interpretation
    /// - Represents the south-west corner (bottom-left in visual terms)
    ///
    /// # Coordinate Constraints
    /// - x ∈ [0, width)
    /// - y ∈ [0, height)
    ///
    /// where `width` and `height` are the dimensions of the containing grid.
    origin: OffsetCoordinate,
    /// The horizontal extent of the rectangle in tile units.
    ///
    /// # Requirements
    /// - Must be a positive integer (≥1)
    width: u32,
    /// The vertical extent of the rectangle in tile units.
    ///
    /// # Requirements
    /// - Must be a positive integer (≥1)
    height: u32,
}

impl Rectangle {
    /// Creates a new rectangle with the given origin, width, height, and grid.
    ///
    /// # Parameters
    /// - `origin`: The origin of the rectangle in offset coordinates.
    /// This represents the bottom-left (south-west) corner of the rectangle in the grid.
    /// It can be any valid offset coordinate,
    /// we will process this origin to ensure its x is in the range [0, map_width - 1] and y is in the range [0, map_height - 1].
    /// - `width`: The width of the rectangle in tiles.
    /// - `height`: The height of the rectangle in tiles.
    /// - `grid`: The grid of the map. It is used to determine the map boundaries and wrapping behavior.
    ///
    /// # Panics
    /// This function will panic if the rectangle is not valid.
    pub fn new(origin: OffsetCoordinate, width: u32, height: u32, grid: HexGrid) -> Self {
        // Debug-only validation
        debug_assert!(
            width > 0 && height > 0,
            "Rectangle dimensions must be positive (got {}x{})",
            width,
            height
        );
        debug_assert!(
            width <= grid.width() && height <= grid.height(),
            "Rectangle dimensions {}x{} exceed grid size {}x{}",
            width,
            height,
            grid.width(),
            grid.height()
        );

        let normalize_origin = grid
            .normalize_offset(origin)
            .expect(&format!("Offset coordinate out of bounds: {:?}", origin));

        Self {
            origin: normalize_origin,
            width,
            height,
        }
    }

    /// Creates a new rectangle from the given origin and top-left corner.
    ///
    /// # Parameters
    /// - `origin`: The origin of the rectangle in offset coordinates.
    /// This represents the bottom-left (south-west) corner of the rectangle in the grid.
    /// It can be any valid offset coordinate,
    /// we will process this origin to ensure its x is in the range [0, map_width - 1] and y is in the range [0, map_height - 1].
    /// - `top_right_corner`: The top-right corner of the rectangle in offset coordinates.
    /// This represents the top-right (north-east) corner of the rectangle in the grid.
    /// It can be any valid offset coordinate.
    /// - `grid`: The grid of the map. It is used to determine the map boundaries and wrapping behavior.
    ///
    /// # Panics
    /// This function will panic if the rectangle is not valid.
    pub fn from_corners(
        origin: OffsetCoordinate,
        top_right_corner: OffsetCoordinate,
        grid: HexGrid,
    ) -> Self {
        let normalize_origin = grid
            .normalize_offset(origin)
            .expect(&format!("Offset coordinate out of bounds: {:?}", origin));

        let [mut width, mut height] = (top_right_corner.0 - normalize_origin.0 + 1).to_array();

        if grid.wrap_flags.contains(WrapFlags::WrapX) {
            width = width.rem_euclid(grid.width() as i32);
        }
        if grid.wrap_flags.contains(WrapFlags::WrapY) {
            height = height.rem_euclid(grid.height() as i32);
        }

        debug_assert!(
            width > 0
                && width <= grid.width() as i32
                && height > 0
                && height <= grid.height() as i32,
            "The rectangle does not exist"
        );

        Self {
            origin,
            width: width as u32,
            height: height as u32,
        }
    }

    #[inline]
    pub fn origin(&self) -> OffsetCoordinate {
        self.origin
    }

    #[inline]
    pub fn west_x(&self) -> i32 {
        self.origin.0.x
    }

    #[inline]
    pub fn south_y(&self) -> i32 {
        self.origin.0.y
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns an iterator over all tiles in current rectangle region of the map.
    pub fn iter_tiles<'a>(&'a self, grid: HexGrid) -> impl Iterator<Item = Tile> + 'a {
        (self.south_y()..self.south_y() + self.height as i32).flat_map(move |y| {
            (self.west_x()..self.west_x() + self.width as i32).map(move |x| {
                let offset_coordinate = OffsetCoordinate::new(x, y);
                Tile::from_offset(offset_coordinate, grid)
            })
        })
    }

    /// Checks if the given tile is inside the current rectangle.
    ///
    /// Returns `true` if the given tile is inside the current rectangle.
    pub fn contains(&self, grid: HexGrid, tile: Tile) -> bool {
        let [mut x, mut y] = tile.to_offset(grid).to_array();

        // We should consider the map is wrapped around horizontally.
        if x < self.west_x() {
            x += grid.width() as i32;
        }

        // We should consider the map is wrapped around vertically.
        if y < self.south_y() {
            y += grid.height() as i32;
        }

        x >= self.west_x()
            && x < self.west_x() + self.width as i32
            && y >= self.south_y()
            && y < self.south_y() + self.height as i32
    }
}
