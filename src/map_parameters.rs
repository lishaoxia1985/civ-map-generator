//! This module defines the [MapParameters] struct that contains all the parameters for generating maps.

use core::debug_assert;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::grid::{
    GridSize, Rectangle, Size, WorldSizeType, WrapFlags,
    hex_grid::{HexGrid, HexLayout, HexOrientation, Offset},
};

/// The parameters for generating a map.
pub struct MapParameters {
    /// The seed used to generate the map.
    ///
    /// This seed is used to ensure that the map is reproducible and can be generated again with the same parameters.
    pub seed: u64,
    /// The type of map to generate.
    ///
    /// This can be either [`MapType::Fractal`] or [`MapType::Pangaea`] or other custom map types.
    pub map_type: MapType,
    /// The grid representing the world.
    ///
    /// This grid is used to generate the map and contains information about the layout, size, the type of size, wrapping, and other properties of the map.
    pub world_grid: WorldGrid,
    /// The profile related to the world size type of the map.
    pub world_size_type_profile: WorldSizeTypeProfile,
    /// The number of large lakes to generate on the map.
    /// The count excludes lakes formed during terrain type generation, only including those created in the lake-adding process.
    ///
    /// A `large lake` is defined as a contiguous lake area covering 4 or more tiles.
    pub num_large_lakes: u32,
    /// The max area size of a lake.
    ///
    /// The water areas with size less than or equal to this value, which are surrounded by land, will be considered as lakes.
    pub max_lake_area_size: u32,
    /// Store the chance of each eligible tile to become a coast in each iteration.
    ///
    /// - Its 'length' is the number of iterations. The more iterations, the more coasts will be generated.
    /// - its 'element' is the chance for each eligible tile to become an expansion coast in each iteration. `0.0` means no chance, `1.0` means 100% chance.\
    ///   If it is empty the coast will not expand, and then only the water tiles adjacent to land can become coast.
    pub coast_expand_chance: Vec<f64>,
    /// The sea level of the map. It affect only terrain type generation.
    pub sea_level: SeaLevel,
    /// The age of the world. It affect only terrain type generation.
    pub world_age: WorldAge,
    /// The temperature of the map. It affect only base terrain generation.
    pub temperature: Temperature,
    /// The rainfall of the map. It affect only feature generation.
    pub rainfall: Rainfall,
    /// The method used to divide the map into regions.
    pub region_divide_method: RegionDivideMethod,
    /// Whether the civilization starting tile must be coastal land.
    ///
    /// - If true, the civilization starting tile only can be coastal land.
    /// - If false, the civilization starting tile can be any hill/flatland tile (including coastal land tiles).
    pub civ_require_coastal_land_start: bool,
    /// The resource setting of the map.
    pub resource_setting: ResourceSetting,
}

impl MapParameters {
    /// The maximum number of civilizations that can be placed on the map.
    pub const MAX_CIVILIZATION_NUM: u32 = 22;

    /// The maximum number of city states that can be placed on the map.
    pub const MAX_CITY_STATE_NUM: u32 = 41;

    /// The maximum number of regions that can share a regional-exclusive luxury resource type.
    ///
    /// All the regional exclusive luxury resources are in the [`LuxuryResourceRole::luxury_assigned_to_regions`](crate::tile_map::LuxuryResourceRole::luxury_assigned_to_regions).
    ///
    /// For example, when set to 3, each regionally-exclusive luxury resource type will be
    /// distributed to no more than 3 distinct regions in the game world.
    pub const MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE: u32 = 3;

    /// The maximum number of distinct luxury resource types that can be exclusively assigned to regions.
    ///
    /// This is used to determine the maximum number of luxury resources that can be assigned to regions
    /// based on the number of civilizations and the maximum number of regions per exclusive luxury.
    ///
    /// Because in original CIV5, the same regional luxury resource can only be found in at most 3 regions on the map.
    /// And there are a maximum of 22 civilizations (each representing a region) in the game, 3 * 8  = 24, it's enough for all civilizations.
    pub const NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS: usize =
        Self::MAX_CIVILIZATION_NUM.div_ceil(Self::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE) as usize;

    /// The maximum number of distinct luxury resource types that can be exclusively assigned to city states.
    ///
    /// This is used to limit the number of luxury resource types that can be exclusively assigned to city states.
    ///
    /// In original CIV5, this value is 3.
    pub const NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_CITY_STATES: usize = 3;
}

impl Default for MapParameters {
    fn default() -> Self {
        MapParametersBuilder::new(WorldGrid::default()).build()
    }
}

/// A builder for constructing [`MapParameters`].
///
/// This builder allows for the flexible configuration of map generation settings.
/// It separates the construction process from the final object representation,
/// allowing for more granular control over the map parameters.
pub struct MapParametersBuilder {
    seed: u64,
    world_grid: WorldGrid,
    map_type: MapType,
    world_size_type_profile: WorldSizeTypeProfile,
    num_large_lakes: u32,
    max_lake_area_size: u32,
    coast_expand_chance: Vec<f64>,
    sea_level: SeaLevel,
    world_age: WorldAge,
    temperature: Temperature,
    rainfall: Rainfall,
    region_divide_method: RegionDivideMethod,
    civ_require_coastal_land_start: bool,
    resource_setting: ResourceSetting,
}

impl MapParametersBuilder {
    /// Creates a new `MapParametersBuilder` with the mandatory core parameters.
    ///
    /// # Arguments
    ///
    /// - `world_grid`: The grid definition for the world layout.
    pub fn new(world_grid: WorldGrid) -> Self {
        Self {
            seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .try_into()
                .unwrap(),
            world_grid,
            map_type: Default::default(),
            world_size_type_profile: WorldSizeTypeProfile::from_world_size_type(
                world_grid.world_size(),
            ),
            num_large_lakes: 2,
            max_lake_area_size: 9,
            coast_expand_chance: vec![0.25, 0.25], // Default to two iterations with 25% chance each.
            sea_level: SeaLevel::Normal,
            world_age: WorldAge::Normal,
            temperature: Temperature::Normal,
            rainfall: Rainfall::Normal,
            region_divide_method: RegionDivideMethod::Continent,
            civ_require_coastal_land_start: false,
            resource_setting: ResourceSetting::Standard,
        }
    }

    // --- Chainable Setter Methods ---

    /// Sets the seed for the map generation.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets the type of map to generate (e.g., Fractal, Pangaea).
    pub fn map_type(mut self, map_type: MapType) -> Self {
        self.map_type = map_type;
        self
    }

    /// Sets the profile related to the world size type.
    pub fn world_size_type_profile(mut self, profile: WorldSizeTypeProfile) -> Self {
        // TODO: We may need to validate that the provided profile is consistent with the world size type of the world grid.
        // For example, the world size is too small for the number of civilizations specified in the profile.
        self.world_size_type_profile = profile;
        self
    }

    /// Sets the number of large lakes to generate.
    pub fn num_large_lakes(mut self, count: u32) -> Self {
        self.num_large_lakes = count;
        self
    }

    /// Sets the maximum area size for a lake.
    pub fn max_lake_area_size(mut self, size: u32) -> Self {
        self.max_lake_area_size = size;
        self
    }

    /// Sets the probability vector for coast expansion in each iteration.
    pub fn coast_expand_chance(mut self, chances: Vec<f64>) -> Self {
        self.coast_expand_chance = chances;
        self
    }

    /// Sets the sea level configuration.
    pub fn sea_level(mut self, sea_level: SeaLevel) -> Self {
        self.sea_level = sea_level;
        self
    }

    /// Sets the age of the world.
    pub fn world_age(mut self, age: WorldAge) -> Self {
        self.world_age = age;
        self
    }

    /// Sets the temperature configuration.
    pub fn temperature(mut self, temperature: Temperature) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the rainfall configuration.
    pub fn rainfall(mut self, rainfall: Rainfall) -> Self {
        self.rainfall = rainfall;
        self
    }

    /// Sets the method used to divide the map into regions.
    pub fn region_divide_method(mut self, method: RegionDivideMethod) -> Self {
        self.region_divide_method = method;
        self
    }

    /// Sets whether the civilization starting tile is required to be coastal land.
    pub fn civ_require_coastal_land_start(mut self, require: bool) -> Self {
        self.civ_require_coastal_land_start = require;
        self
    }

    /// Sets the resource generation settings.
    pub fn resource_setting(mut self, setting: ResourceSetting) -> Self {
        self.resource_setting = setting;
        self
    }

    /// Finalizes the construction and returns the `MapParameters` instance.
    pub fn build(self) -> MapParameters {
        MapParameters {
            map_type: self.map_type,
            world_grid: self.world_grid,
            seed: self.seed,
            world_size_type_profile: self.world_size_type_profile,
            num_large_lakes: self.num_large_lakes,
            max_lake_area_size: self.max_lake_area_size,
            coast_expand_chance: self.coast_expand_chance,
            sea_level: self.sea_level,
            world_age: self.world_age,
            temperature: self.temperature,
            rainfall: self.rainfall,
            region_divide_method: self.region_divide_method,
            civ_require_coastal_land_start: self.civ_require_coastal_land_start,
            resource_setting: self.resource_setting,
        }
    }
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
///    automatically determining the [`WorldSizeType`] based on grid dimensions:
/// ```rust
/// use civ_map_generator::grid::{*,hex_grid::*};
/// use civ_map_generator::map_parameters::*;
///
/// let grid = HexGrid::new(
///     Size { width: 80, height: 40 }, // Custom grid size
///     HexLayout {
///         orientation: HexOrientation::Flat,
///         size: [8., 8.],
///         origin: [0., 0.],
///     }, // Hex layout
///     Offset::Odd, // Odd offset for hexagonal grid
///     WrapFlags::WrapX, // Wrap horizontally
/// );
///
/// let world_grid = WorldGrid::from_grid(grid);
/// ```
///
/// 2. Explicit [`WorldSizeType`] specification - Creates with default grid dimensions
///    for a standardized world size:
/// ```rust
/// use civ_map_generator::grid::{*,hex_grid::*};
/// use civ_map_generator::map_parameters::*;
///
/// let world_size_type = WorldSizeType::Standard;
/// let mut grid = HexGrid::new(
///     HexGrid::default_size(world_size_type), // Default dimensions based on world size classification
///     HexLayout {
///         orientation: HexOrientation::Flat,
///         size: [8., 8.],
///         origin: [0., 0.],
///     }, // Hex layout
///     Offset::Odd, // Odd offset for hexagonal grid
///     WrapFlags::WrapX, // Wrap horizontally
/// );
///
/// let world_grid = WorldGrid::new(grid, world_size_type);
/// ```
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct WorldGrid {
    pub grid: HexGrid,
    pub world_size_type: WorldSizeType,
}

impl WorldGrid {
    /// Creates a new `WorldGrid` with the specified grid and world size.
    ///
    /// # Notice
    ///
    /// Before calling this function, ensure that the grid's size matches the specified world size.
    /// This check is performed at runtime through `debug_assert!`, which only activates in debug mode.
    ///
    /// # Usage
    ///
    /// This function should be used exclusively with the initialization syntax shown below.
    /// Direct initialization with the `new` function outside of this pattern is not supported:
    ///
    /// ```rust
    /// use civ_map_generator::grid::{*,hex_grid::*};
    /// use civ_map_generator::map_parameters::*;
    ///
    /// let world_size_type = WorldSizeType::Standard;
    /// let mut grid = HexGrid::new(
    ///     HexGrid::default_size(world_size_type), // Default dimensions based on world size classification
    ///     HexLayout {
    ///         orientation: HexOrientation::Flat,
    ///         size: [8., 8.],
    ///         origin: [0., 0.],
    ///     }, // Hex layout
    ///     Offset::Odd, // Odd offset for hexagonal grid
    ///     WrapFlags::WrapX, // Wrap horizontally
    /// );
    ///
    /// let world_grid = WorldGrid::new(grid, world_size_type);
    /// ```
    ///
    pub fn new(grid: HexGrid, world_size: WorldSizeType) -> Self {
        debug_assert!(
            grid.world_size_type() == world_size,
            "Grid size does not match the specified world size"
        );
        Self {
            grid,
            world_size_type: world_size,
        }
    }

    pub fn from_grid(grid: HexGrid) -> Self {
        let world_size = grid.world_size_type();
        Self {
            grid,
            world_size_type: world_size,
        }
    }

    /// Get the size of the grid.
    pub fn size(&self) -> Size {
        self.grid.size
    }

    /// Get the world size of the grid.
    pub fn world_size(&self) -> WorldSizeType {
        self.world_size_type
    }
}

impl Default for WorldGrid {
    fn default() -> Self {
        let world_size = WorldSizeType::Standard;
        let grid = HexGrid {
            size: HexGrid::default_size(world_size),
            layout: HexLayout {
                orientation: HexOrientation::Pointy,
                size: [50., 50.],
                origin: [0., 0.],
            },
            wrap_flags: WrapFlags::WrapX,
            offset: Offset::Odd,
        };
        Self {
            grid,
            world_size_type: world_size,
        }
    }
}

/// The type of map to generate.
#[derive(Default)]
pub enum MapType {
    #[default]
    Fractal,
    Pangaea,
}

/// The sea level of the map. It affect only terrain type generation.
#[derive(Default)]
pub enum SeaLevel {
    /// Fewer water tiles will be generated on the map than [`SeaLevel::Normal`].
    Low,
    /// The water tiles will be generated on the map as usual.
    #[default]
    Normal,
    /// More water tiles will be generated on the map than [`SeaLevel::Normal`].
    High,
    /// A random sea level between [`SeaLevel::Low`] and [`SeaLevel::High`].
    Random,
}

/// The age of the world. It affect only base terrain generation.
///
/// This value determines:
/// - How many tectonic plates will be used to generate terrain types on the map.
///   A simple Voronoi diagram to simulate tectonic plates is used to generate ridgelines of mountains and hills.
///   The older the world, the less active the plates are.
/// - The number of mountains and hills on the map.
///   The older the world, the fewer mountains and hills on the map.
#[derive(Default)]
pub enum WorldAge {
    /// 5 Billion Years
    ///
    /// Few plates will be used to generate terrain types on the map than [`WorldAge::Normal`].
    /// There will be fewer mountains and hills on the map.
    Old,
    /// 4 Billion Years
    ///
    /// Plates will be used to generate terrain types on the map as usual.
    /// There will be a normal number of mountains and hills on the map.
    #[default]
    Normal,
    /// 3 Billion Years
    ///
    /// More plates will be used to generate terrain types on the map than [`WorldAge::Normal`].
    /// There will be more mountains and hills on the map.
    New,
}

/// The temperature of the map. It affect only base terrain generation.
#[derive(Default)]
pub enum Temperature {
    /// More tundra and snow, less desert.
    Cool,
    /// The base terrain will be generated on the map as usual.
    #[default]
    Normal,
    /// More desert, less tundra and snow.
    Hot,
}

/// The rainfall of the map. It affect only feature generation.
#[derive(Default)]
pub enum Rainfall {
    /// Less forest, jungle, and marsh.
    Arid,
    /// The features will be generated on the map as usual.
    #[default]
    Normal,
    /// More forest, jungle, and marsh.
    Wet,
    /// Random rainfall.
    Random,
}

/// Defines the method used to divide regions for civilizations in the game. This enum is used to determine how civilizations are assigned to different regions on the map.
#[derive(Default)]
pub enum RegionDivideMethod {
    /// All civilizations start on the biggest landmass.
    ///
    /// This method places all civs on a single, largest landmass.
    Pangaea,
    /// Civs are assigned to continents. Any continents with more than one civ are divided.
    #[default]
    Continent,
    /// This method is primarily used for Archipelago or other maps with many small islands.
    ///
    /// The entire map is treated as one large rectangular region.
    /// [`RegionDivideMethod::WholeMapRectangle`] is equivalent to [`RegionDivideMethod::CustomRectangle`] when [`Rectangle`] encompasses the entire map area.
    /// We will ignore the area ID when method is set to WholeMapRectangle.
    WholeMapRectangle,
    /// Civs start within a custom-defined rectangle.
    ///
    /// We will ignore the area ID when method is set to CustomRectangle.
    CustomRectangle(Rectangle),
}

/// The resource setting of the map.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ResourceSetting {
    /// Few resources will be placed on the map than [`ResourceSetting::Standard`].
    Sparse,
    /// Standard number of resources will be placed on the map.
    #[default]
    Standard,
    /// More resources will be placed on the map than [`ResourceSetting::Standard`].
    Abundant,
    /// More resources will be placed around the starting tile of each civilization.
    LegendaryStart,
    /// Every civilization will begin with a starting tile containing approximately the same amount of strategic resources.
    StrategicBalance,
}

/// Stores the profile related to the world size type of the map.
pub struct WorldSizeTypeProfile {
    /// The number of civilizations, excluding city states.
    ///
    /// This value must be in the range of **[2, [`MapParameters::MAX_CIVILIZATION_NUM`]]**.
    pub num_civilizations: u32,
    /// The number of city states.
    ///
    /// This value must be in the range of **[0, [`MapParameters::MAX_CITY_STATE_NUM`]]**.
    pub num_city_states: u32,
    /// The number of wonders that will be placed on the map.
    pub num_natural_wonders: u32,

    //////////////////////////////////////////////////////////////////////////////
    /* The fields below are not used now, but may be used in the future */
    /// Maximum number of active religions allowed in the game.
    pub max_religions: u32,

    /// Base unhappiness penalty per owned city.
    pub unhappiness_per_city: f32,

    /// Additional unhappiness penalty per annexed city.
    pub unhappiness_annexed: f32,

    /// Base technology cost modifier (as decimal, e.g., 100% = 1.0, 110% = 1.1)
    pub tech_cost_base: f32,

    /// Per-city technology cost increase (as decimal, e.g., 5% = 0.05)
    pub tech_cost_per_city: f32,

    /// Per-city policy cost increase (as decimal, e.g., 10% = 0.10)
    pub policy_cost_per_city: f32,
}

impl WorldSizeTypeProfile {
    /// Creates a new `WorldSizeTypeProfile` with the specified parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        num_civilizations: u32,
        num_city_states: u32,
        num_natural_wonders: u32,
        max_religions: u32,
        unhappiness_per_city: f32,
        unhappiness_annexed: f32,
        tech_cost_base: f32,
        tech_cost_per_city: f32,
        policy_cost_per_city: f32,
    ) -> Self {
        Self {
            num_civilizations,
            num_city_states,
            num_natural_wonders,
            max_religions,
            unhappiness_per_city,
            unhappiness_annexed,
            tech_cost_base,
            tech_cost_per_city,
            policy_cost_per_city,
        }
    }

    /// Creates a new `WorldSizeTypeProfile` from the specified `WorldSizeType`.
    pub fn from_world_size_type(world_size_type: WorldSizeType) -> Self {
        match world_size_type {
            WorldSizeType::Duel => Self::new(2, 4, 2, 2, 3.0, 5.0, 1.0, 0.05, 0.10),
            WorldSizeType::Tiny => Self::new(4, 8, 3, 3, 3.0, 5.0, 1.0, 0.05, 0.10),
            WorldSizeType::Small => Self::new(6, 12, 4, 4, 3.0, 5.0, 1.0, 0.05, 0.10),
            WorldSizeType::Standard => Self::new(8, 16, 5, 5, 3.0, 5.0, 1.1, 0.05, 0.10),
            WorldSizeType::Large => Self::new(10, 20, 6, 6, 2.4, 4.0, 1.2, 0.03, 0.075),
            WorldSizeType::Huge => Self::new(12, 24, 7, 7, 1.8, 3.0, 1.3, 0.02, 0.05),
        }
    }
}
