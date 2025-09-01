//! This module defines the [`TileMap`] struct and its associated methods.
//! It provides functionality to manage and manipulate a map of tiles, including
//! querying tile properties, placing resources, and managing layers of data.
//! Its method contains 2 parts:
//! 1. The common methods for map generation, included in the `mod.rs` file.
//! 2. The map generating methods are defined in the [`impls`] module ( which is the submodule of this module).

use crate::{
    grid::{
        direction::Direction,
        hex_grid::{hex::HexOrientation, HexGrid},
    },
    map_parameters::{MapParameters, WorldGrid},
    nation::Nation,
    tile::Tile,
    tile_component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        resource::Resource, terrain_type::TerrainType,
    },
};
use arrayvec::ArrayVec;
use enum_map::{enum_map, Enum, EnumMap};
use rand::{rngs::StdRng, SeedableRng};
use std::{
    cmp::{max, min},
    collections::{BTreeMap, HashMap},
};

pub(crate) mod impls;

use impls::{
    assign_starting_tile::LuxuryResourceRole,
    generate_area_ids::{Area, Landmass},
    generate_regions::Region,
};

#[derive(PartialEq, Debug)]
pub struct TileMap {
    pub random_number_generator: StdRng,
    pub world_grid: WorldGrid,
    pub river_list: Vec<River>,
    // queries
    pub terrain_type_query: Vec<TerrainType>,
    pub base_terrain_query: Vec<BaseTerrain>,
    pub feature_query: Vec<Option<Feature>>,
    pub natural_wonder_query: Vec<Option<NaturalWonder>>,
    pub resource_query: Vec<Option<(Resource, u32)>>,
    pub area_id_query: Vec<usize>,
    /// List of areas in the map. The index is equal to the area id.
    pub area_list: Vec<Area>,
    pub landmass_id_query: Vec<usize>,
    /// List of landmasses in the map. The index is equal to the landmass id.
    pub landmass_list: Vec<Landmass>,
    /// Starting tile and placed civilization.
    pub starting_tile_and_civilization: BTreeMap<Tile, Nation>,
    /// Starting tile and placed city state.
    pub starting_tile_and_city_state: BTreeMap<Tile, Nation>,
    /// List of regions in the map. The index is equal to the region id.
    region_list: ArrayVec<Region, { MapParameters::MAX_CIVILIZATION_NUM as usize }>,
    /// Stores the impact and ripple values of the tiles in the [`Layer`] when an element,
    /// associated with a variant of the [`Layer`], is added to the map.
    ///
    /// It is typically used to ensure that no other elements appear within a defined radius of the placed element,
    /// or that other elements are not too close to the placed element.
    ///
    /// The element may be a starting tile of civilization, a city-state, a natural wonder, a marble, a resource, ...\
    /// The impact and ripple values represent the influence of distance from the added element.
    /// The value is within the range `[0, 99]`.
    ///
    /// # Examples about impact and ripple values
    /// For example, When the `layer` is [`Layer::Civilization`], `layer_data[Layer::Civilization]` stores the "impact and ripple" data
    /// of the starting tile of civilization. About the values of tiles in `layer_data[Layer::Civilization]`:
    /// - `value = 0` means no influence from existing impacts in current tile.
    /// - `value = 99` means an "impact" occurred in current tile, and current tile is a starting tile.
    /// - Values in (0, 99) represent "ripples", indicating that current tile is near a starting tile.
    ///   The larger values, the closer the tile is to the starting tile.
    pub layer_data: EnumMap<Layer, Vec<u32>>,
    /// Stores `impact` data only of start points, to avoid player collisions
    /// It is `true` When the tile has a civ start, CS start, or Natural Wonder.
    pub player_collision_data: Vec<bool>,
    /// City state starting tile and its region index.
    /// If the second element is `None`, then the tile is in the uninhabited area.
    city_state_starting_tile_and_region_index: Vec<(Tile, Option<usize>)>,
    /// Determine every type of luxury resources are the role: assigned to region, city_state, special case, random, or unused.
    luxury_resource_role: LuxuryResourceRole,
    /// The count of luxury resource types assigned to regions.
    ///
    /// Its key is the luxury resource type name, all keys are in the [`LuxuryResourceRole::luxury_assigned_to_regions`].
    /// Its value is the count of assigned luxury resource types, all values should <= [`MapParameters::MAX_REGIONS_PER_EXCLUSIVE_LUXURY_TYPE`].
    ///
    /// It has a maximum length of [`MapParameters::NUM_MAX_ALLOWED_LUXURY_TYPES_FOR_REGIONS`]. See [`TileMap::assign_luxury_to_region`] for more information.
    ///
    /// If a luxury resource type has been assigned to a region, it will be added to this count.
    ///
    /// For example, if the count is 2, it means that one luxury resource type has been assigned to two different regions.
    ///
    /// This count is used to adjust the probability of assigning the same luxury resource to another region.
    /// The higher the count, the lower the chance of assigning that luxury resource to an additional region.
    /// This is achieved by reducing the weight of the resource as the count increases.
    luxury_assign_to_region_count: HashMap<Resource, u32>,
}

impl TileMap {
    /// Creates an empty tile map with the given parameters.
    pub fn new(map_parameters: &MapParameters) -> Self {
        let random_number_generator = StdRng::seed_from_u64(map_parameters.seed);

        let world_grid = map_parameters.world_grid;
        let height = world_grid.grid.size.height;
        let width = world_grid.grid.size.width;

        let size = (height * width) as usize;

        let layer_data = enum_map! {
            _ => vec![0; size],
        };

        let region_list = ArrayVec::new();

        Self {
            random_number_generator,
            world_grid,
            river_list: Vec::new(),
            terrain_type_query: vec![TerrainType::Water; size],
            base_terrain_query: vec![BaseTerrain::Ocean; size],
            feature_query: vec![None; size],
            natural_wonder_query: vec![None; size],
            resource_query: vec![None; size],
            area_id_query: Vec::with_capacity(size),
            landmass_id_query: Vec::with_capacity(size),
            area_list: Vec::new(),
            landmass_list: Vec::new(),
            region_list,
            layer_data,
            player_collision_data: vec![false; size],
            starting_tile_and_civilization: BTreeMap::new(),
            starting_tile_and_city_state: BTreeMap::new(),
            city_state_starting_tile_and_region_index: Vec::new(),
            luxury_resource_role: LuxuryResourceRole::default(),
            luxury_assign_to_region_count: HashMap::new(),
        }
    }

    /// Returns an iterator over all tiles in the map.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn all_tiles(&self) -> impl Iterator<Item = Tile> {
        let size = &self.world_grid.size();
        (0..((size.width * size.height) as usize)).map(Tile::new)
    }

    /// Place impact and ripples for a given tile and layer.
    ///
    /// When you add an element (such as a starting tile of civilization, a city state, a natural wonder, a marble, or a resource...) to the map,
    /// if you want to ensure no other elements appear around the element being added, you can use this function.
    ///
    /// # Arguments
    ///
    /// - `tile`: the tile to place the impact and ripples on.
    /// - `layer`: the layer to place the impact and ripples on. It should be a variant of the [`Layer`] enum.
    /// - `radius`: the radius of the ripple. The ripple will be placed on all tiles within this radius. When it is `0`, only the impact will be placed on the `tile`.
    ///     - When layer is [`Layer::Strategic`], [`Layer::Luxury`] or [`Layer::Bonus`], [`Layer::Fish`], this argument is used to determine the ripple radius.
    ///     - When layer is other variants, this argument is ignored (recommended to use [`u32::MAX`] as placeholder).
    ///
    /// # Notice
    ///
    /// You can place impact and ripples to forbid other elements to appear around a specific tile, even if you are not adding an element to this tile.
    /// See [`TileMap::normalize_civilization_starting_tile`] for an example.
    ///
    pub fn place_impact_and_ripples(&mut self, tile: Tile, layer: Layer, radius: u32) {
        match layer {
            Layer::Strategic | Layer::Luxury | Layer::Bonus | Layer::Fish => {
                self.place_impact_and_ripples_for_resource(tile, layer, radius)
            }
            Layer::CityState => {
                self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 4);

                self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 3);
                // Strategic layer, should be at start point only. That means if we are placing a city state at current tile, forbid to place strategic resources on it.
                self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 0);
                self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 3);
                self.place_impact_and_ripples_for_resource(tile, Layer::Fish, 3);
                self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 3);
            }
            Layer::NaturalWonder => {
                self.place_impact_and_ripples_for_resource(
                    tile,
                    Layer::NaturalWonder,
                    self.world_grid.size().height / 5,
                );
                let natural_wonder = tile.natural_wonder(self);
                if let Some(natural_wonder) = natural_wonder {
                    match natural_wonder {
                        NaturalWonder::MountFuji => {
                            self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 0);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 0);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 0);
                            self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 0);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 1);
                        }
                        NaturalWonder::Krakatoa | NaturalWonder::GreatBarrierReef => {
                            self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 1);
                            // The tile beneath natural wonders on water should block fish resources.
                            self.place_impact_and_ripples_for_resource(tile, Layer::Fish, 1);
                        }
                        _ => {
                            self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 1);
                            self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 1);
                        }
                    }
                }
            }
            Layer::Marble => {
                self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 1);
                self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 6);
            }
            Layer::Civilization => self.place_impact_and_ripples_for_civilization(tile),
        }
    }

    // function AssignStartingPlots:PlaceImpactAndRipples
    /// Places the impact and ripple values for a starting tile of civilization.
    ///
    /// We will place the impact on the tile and then ripple outwards to the surrounding tiles.
    fn place_impact_and_ripples_for_civilization(&mut self, tile: Tile) {
        let grid = self.world_grid.grid;

        let impact_value = 99;
        let ripple_values = [97, 95, 92, 89, 69, 57, 24, 15];

        // Start points need to impact the resource layers.
        self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 3);
        // Strategic layer, should be at start point only. That means if we are placing a civilization at current tile, forbid to place strategic resources on it.
        self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 0);
        self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 3);
        self.place_impact_and_ripples_for_resource(tile, Layer::Fish, 3);
        // Natural Wonders layer, set a minimum distance of 5 tiles (4 ripples) away.
        self.place_impact_and_ripples_for_resource(tile, Layer::NaturalWonder, 4);

        self.layer_data[Layer::Civilization][tile.index()] = impact_value;

        self.player_collision_data[tile.index()] = true;

        self.layer_data[Layer::CityState][tile.index()] = 1;

        for (index, ripple_value) in ripple_values.into_iter().enumerate() {
            let distance = index as u32 + 1;

            tile.tiles_at_distance(distance, grid)
                .for_each(|tile_at_distance| {
                    let mut current_value =
                        self.layer_data[Layer::Civilization][tile_at_distance.index()];
                    if current_value != 0 {
                        // First choose the greater of the two, existing value or current ripple.
                        let stronger_value = max(current_value, ripple_value);
                        // Now increase it by 1.2x to reflect that multiple civs are in range of this plot.
                        let overlap_value = min(97, (stronger_value as f64 * 1.2) as u32);
                        current_value = overlap_value;
                    } else {
                        current_value = ripple_value;
                    }
                    // Update the layer data with the new value.
                    self.layer_data[Layer::Civilization][tile_at_distance.index()] = current_value;

                    if distance <= 6 {
                        self.layer_data[Layer::CityState][tile_at_distance.index()] = 1;
                    }
                })
        }
    }

    // AssignStartingPlots:PlaceResourceImpact
    /// Place impact and ripple for resource on the map.
    ///
    /// We will place the resource impact on the tile and then place a ripple on all tiles within the radius.
    ///
    /// # Arguments
    ///
    /// - `tile`: the tile to place the resource impact on.
    /// - `layer`: the layer to place the resource impact and ripple on. `layer` should not be [`Layer::Civilization`].
    /// - `radius`: the radius of the ripple. The ripple will be placed on all tiles within this radius.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `layer` is [`Layer::Civilization`]. If you want to place impact and ripples on the civilization layer, use [`TileMap::place_impact_and_ripples_for_civilization`].
    fn place_impact_and_ripples_for_resource(&mut self, tile: Tile, layer: Layer, radius: u32) {
        debug_assert_ne!(
            layer,
            Layer::Civilization,
            "`place_impact_and_ripples_for_resource` should not be used for `Layer::Civilization`, use `place_impact_and_ripples_for_civilization` instead."
        );

        let grid = self.world_grid.grid;

        let impact_value = if layer == Layer::Fish || layer == Layer::Marble {
            1
        } else {
            99
        };

        self.layer_data[layer][tile.index()] = impact_value;

        if radius == 0 {
            return;
        }

        if radius > 0 && radius < (grid.size.height / 2) {
            for distance in 1..=radius {
                // `distance` is the distance from the center tile to the current tile.
                // The larger the distance, the smaller the ripple value.
                let ripple_value = radius - distance + 1;
                // Iterate over all tiles at this distance.
                tile.tiles_at_distance(distance, grid)
                    .for_each(|tile_at_distance| {
                        // The current tile's ripple value.
                        let mut current_value = self.layer_data[layer][tile_at_distance.index()];
                        match layer {
                            Layer::Strategic | Layer::Luxury | Layer::Bonus | Layer::NaturalWonder => {
                                if current_value != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(current_value, ripple_value);
                                    // Now increase it by 2 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(50, stronger_value + 2);
                                    current_value = overlap_value;
                                } else {
                                    current_value = ripple_value;
                                }
                            }
                            Layer::Fish => {
                                if current_value != 0 {
                                    // First choose the greater of the two, existing value or current ripple.
                                    let stronger_value = max(current_value, ripple_value);
                                    // Now increase it by 1 to reflect that multiple civs are in range of this plot.
                                    let overlap_value = min(10, stronger_value + 1);
                                    current_value = overlap_value;
                                } else {
                                    current_value = ripple_value;
                                }
                            }
                            Layer::CityState | Layer::Marble => {
                                current_value = 1;
                            }
                            Layer::Civilization => {
                                unreachable!("Civilization layer should not be used in place_resource_impact function.");
                            }
                        }
                        // Update the layer data with the new value.
                        self.layer_data[layer][tile_at_distance.index()] = current_value;
                    })
            }
        }
    }
}

/// The `Layer` enum represents a layer associated with an element added to the map.
/// Each element is linked to a specific variant of the `Layer`.
///
/// The element can be a starting tile for a civilization, a city-state, a natural wonder, a marble, a resource, and more.
///
/// The `Layer` enum is used in [`TileMap::layer_data`]. For more information, see [`TileMap::layer_data`].
///
/// # How to add a new layer
/// For example, when you add an element `Stone` to the map, you want to ensure that no other elements appear around the element being added.
/// To do this, you need to add a new layer to the `Layer` enum. you need to:
/// 1. Add a new variant to the `Layer` enum. for example:
/// ```rust
/// # #[cfg(never)]
/// pub enum Layer {
///    Strategic,
///    Luxury,
///    // ... other existing variants
///    Stone,  // New variant added
/// }
/// ```
///
/// 2. Add a new case to [`TileMap::place_impact_and_ripples`] in the `TileMap` struct. This function is responsible for placing the impact of the element on the map and creating ripples if necessary.
/// ```rust
/// # #[cfg(never)]
/// pub fn place_impact_and_ripples(
///     &mut self,
///     map_parameters: &MapParameters,
///     tile: Tile,
///     layer: Layer,
///     radius: Option<u32>,
/// ) {
///     match layer {
///         // ... other existing cases
///         Layer::Stone => {
///             // ... implementation for the new layer
///         }
///     }
/// }
/// ```
///
/// 3. When you add a `Stone` to the map, you need to call [`TileMap::place_impact_and_ripples`] with the new layer.
///
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Strategic,
    Luxury,
    Bonus,
    Fish,
    CityState,
    NaturalWonder,
    Marble,
    Civilization,
}

/// Represents a river in the tile map.
pub type River = Vec<RiverEdge>;

/// Represents a river edge in the tile map.
/// Multiple consecutive `RiverEdge` can be used to represent a river.
///
/// Usually, we use [`River`] to represent a river.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiverEdge {
    /// The position of the river edge in the tile map.
    pub tile: Tile,
    /// The flow direction of the river edge.
    pub flow_direction: Direction,
}

impl RiverEdge {
    /// Creates a new `RiverEdge` with the given tile and flow direction.
    pub fn new(tile: Tile, flow_direction: Direction) -> Self {
        Self {
            tile,
            flow_direction,
        }
    }

    /// Get the start and end corner directions of the river edge.
    ///
    /// According to the flow direction, we can determine which corners of the tile the river edge starts and ends at.
    ///
    /// # Returns
    ///
    /// Returns an array containing the start and end corner directions of the current tile.
    /// According to the start and end corners, we can draw the river edge on the current tile.
    pub fn start_and_end_corner_directions(&self, grid: HexGrid) -> [Direction; 2] {
        use {Direction::*, HexOrientation::*};

        // Match on both orientation and flow direction simultaneously
        match (grid.layout.orientation, self.flow_direction) {
            // Pointy-top orientation cases
            (Pointy, North) => [SouthEast, NorthEast], // North flow connects SE and NE corners
            (Pointy, NorthEast) => [South, SouthEast], // NE flow connects S and SE corners
            (Pointy, SouthEast) => [SouthWest, South], // SE flow connects SW and S corners
            (Pointy, South) => [NorthEast, SouthEast], // South flow connects NE and SE corners
            (Pointy, SouthWest) => [SouthEast, South], // SW flow connects SE and S corners
            (Pointy, NorthWest) => [South, SouthWest], // NW flow connects S and SW corners

            // Flat-top orientation cases
            (Flat, NorthEast) => [SouthEast, East], // NE flow connects SE and E corners
            (Flat, East) => [SouthWest, SouthEast], // E flow connects SW and SE corners
            (Flat, SouthEast) => [NorthEast, East], // SE flow connects NE and E corners
            (Flat, SouthWest) => [East, SouthEast], // SW flow connects E and SE corners
            (Flat, West) => [SouthEast, SouthWest], // W flow connects SE and SW corners
            (Flat, NorthWest) => [East, NorthEast], // NW flow connects E and NE corners

            // Invalid combinations - directions that don't exist in certain orientations
            (Pointy, East | West) | (Flat, North | South) => {
                panic!("Invalid flow direction for this hex orientation")
            }
        }
    }

    /// Gets the edge direction corresponding to the given flow direction in the current tile.
    ///
    /// According to the flow direction, we can determine which edge of the tile the river edge belongs to.
    ///
    /// # Returns
    ///
    /// Returns the edge direction corresponding to the given flow direction in the current tile.
    pub fn edge_direction(&self, grid: HexGrid) -> Direction {
        use {Direction::*, HexOrientation::*};

        match (grid.layout.orientation, self.flow_direction) {
            // Pointy orientation cases
            (Pointy, North | South) => East,
            (Pointy, NorthEast | SouthWest) => SouthEast,
            (Pointy, NorthWest | SouthEast) => SouthWest,

            // Flat orientation cases
            (Flat, NorthWest | SouthEast) => NorthEast,
            (Flat, NorthEast | SouthWest) => SouthEast,
            (Flat, East | West) => South,

            // Invalid combinations
            _ => panic!("Invalid flow direction for hex orientation"),
        }
    }
}
