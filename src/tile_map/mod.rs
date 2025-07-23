use std::cmp::max;
use std::cmp::min;
use std::collections::BTreeMap;

use std::collections::HashMap;

pub(crate) mod impls;

use enum_map::{enum_map, Enum, EnumMap};
use impls::generate_area_ids::Area;
use impls::generate_area_ids::Landmass;
use impls::{assign_starting_tile::LuxuryResourceRole, generate_regions::Region};
use rand::{rngs::StdRng, SeedableRng};

use crate::map_parameters::WorldGrid;
use crate::{
    component::map_component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        resource::Resource, terrain_type::TerrainType,
    },
    grid::direction::Direction,
    map_parameters::MapParameters,
    tile::Tile,
};

pub struct TileMap {
    pub random_number_generator: StdRng,
    pub world_grid: WorldGrid,
    pub river_list: Vec<Vec<(Tile, Direction)>>,
    // queries
    pub terrain_type_query: Vec<TerrainType>,
    pub base_terrain_query: Vec<BaseTerrain>,
    pub feature_query: Vec<Option<Feature>>,
    pub natural_wonder_query: Vec<Option<NaturalWonder>>,
    pub resource_query: Vec<Option<(Resource, u32)>>,
    pub area_id_query: Vec<usize>,
    pub landmass_id_query: Vec<usize>,
    pub civilization_and_starting_tile: BTreeMap<String, Tile>,
    pub city_state_and_starting_tile: BTreeMap<String, Tile>,
    /// List of areas in the map. The index is equal to the area id.
    pub area_list: Vec<Area>,
    /// List of landmasses in the map. The index is equal to the landmass id.
    pub landmass_list: Vec<Landmass>,
    region_list: Vec<Region>,
    /// Stores the impact and ripple values of the tiles in the [`Layer`] when an element,
    /// associated with a variant of the `Layer`, is added to the map.
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
    // These tile will be as candidates for starting tile for city states
    uninhabited_areas_coastal_land_tiles: Vec<Tile>,
    // These tile will be as candidates for starting tile for city states
    uninhabited_areas_inland_tiles: Vec<Tile>,
    /// Store region index which city_state is assigned to,
    /// if it is `None`, city state will be assigned to uninhabited area.
    /// It's length is equal to the number of city states.
    city_state_region_assignments: Vec<Option<usize>>,
    /// City state starting tile and its region index.
    /// Its order is same as `city_state_region_assignments`,
    /// that means `city_state_starting_tile[i]` is in the region `city_state_region_assignments[i]`.
    /// If `city_state_region_assignments[i]` is `None`, then `city_state_starting_tile[i]` is in the uninhabited area.
    city_state_starting_tile_and_region_index: Vec<(Tile, Option<usize>)>,
    /// Determine every type of luxury resources are the role: assigned to region, city_state, special case, random, or unused.
    luxury_resource_role: LuxuryResourceRole,
    /// The count of luxury resource types assigned to regions.
    ///
    /// In CIV5, the maximum number of luxury resource types that can be assigned to regions is 8.
    /// This value has a maximum length of 8. See [`TileMap::assign_luxury_to_region`] for more information.
    ///
    /// If a luxury resource type has been assigned to a region, it will be added to this count.
    ///
    /// For example, if the count is 2, it means that one luxury resource type has been assigned to two different regions.
    ///
    /// This count is used to adjust the probability of assigning the same luxury resource to another region.
    /// The higher the count, the lower the chance of assigning that luxury resource to an additional region.
    /// This is achieved by reducing the weight of the resource as the count increases.
    luxury_assign_to_region_count: HashMap<String, u32>,
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

        let region_list = Vec::with_capacity(map_parameters.civilization_num as usize);

        let city_state_region_assignments = vec![None; map_parameters.city_state_num as usize];

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
            civilization_and_starting_tile: BTreeMap::new(),
            city_state_and_starting_tile: BTreeMap::new(),
            uninhabited_areas_coastal_land_tiles: Vec::new(),
            uninhabited_areas_inland_tiles: Vec::new(),
            city_state_region_assignments,
            city_state_starting_tile_and_region_index: Vec::new(),
            luxury_resource_role: LuxuryResourceRole::default(),
            luxury_assign_to_region_count: HashMap::new(),
        }
    }

    /// Returns an iterator over all tiles in the map.
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
    /// - `radius`: the radius of the ripple. The ripple will be placed on all tiles within this radius.
    ///     - When layer is [`Layer::Strategic`], [`Layer::Luxury`] or [`Layer::Bonus`], [`Layer::Fish`], this argument is used to determine the ripple radius.
    ///     - When layer is other variants, this argument is ignored (recommended to use [`u32::MAX`] as placeholder).
    pub fn place_impact_and_ripples(&mut self, tile: Tile, layer: Layer, radius: u32) {
        match layer {
            Layer::Strategic | Layer::Luxury | Layer::Bonus | Layer::Fish => {
                self.place_impact_and_ripples_for_resource(tile, layer, radius)
            }
            Layer::CityState => {
                self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 4);

                self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 3);
                // Strategic layer, should be at start point only.
                self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 0);
                self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 3);
                self.place_impact_and_ripples_for_resource(tile, Layer::Fish, 3);
                // Natural Wonders layer, set a minimum distance of 5 tiles (4 ripples) away.
                self.place_impact_and_ripples_for_resource(tile, Layer::NaturalWonder, 4);
                self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 3);
            }
            Layer::NaturalWonder => {
                self.place_impact_and_ripples_for_resource(
                    tile,
                    Layer::NaturalWonder,
                    self.world_grid.size().height / 5,
                );
                self.place_impact_and_ripples_for_resource(tile, Layer::Strategic, 1);
                self.place_impact_and_ripples_for_resource(tile, Layer::Luxury, 1);
                self.place_impact_and_ripples_for_resource(tile, Layer::Bonus, 1);
                self.place_impact_and_ripples_for_resource(tile, Layer::CityState, 1);
                self.place_impact_and_ripples_for_resource(tile, Layer::Marble, 1);
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
        // Strategic layer, should be at start point only.
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
    /// Panics on dev mode if `layer` is [`Layer::Civilization`]. If you want to place impact and ripples on the civilization layer, use [`TileMap::place_impact_and_ripples_for_civilization`].
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
