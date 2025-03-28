use std::collections::BTreeMap;

use std::collections::HashMap;

use enum_map::EnumMap;
use enum_map::{enum_map, Enum};
use rand::{rngs::StdRng, SeedableRng};
use tile::Tile;
use tile_map_impls::{assign_starting_tile::LuxuryResourceRole, generate_regions::Region};

mod fractal;
mod map_parameters;
mod tile;
mod tile_map_impls;

pub use self::fractal::{CvFractal, Flags};
pub use crate::tile_map::tile_map_impls::generate_regions::RegionType;
use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        resource::Resource, terrain_type::TerrainType,
    },
    grid::Direction,
};
pub use map_parameters::*;

pub struct TileMap {
    pub random_number_generator: StdRng,
    pub map_size: MapSize,
    pub river_list: Vec<Vec<(Tile, Direction)>>,
    // queries
    pub terrain_type_query: Vec<TerrainType>,
    pub base_terrain_query: Vec<BaseTerrain>,
    pub feature_query: Vec<Option<Feature>>,
    pub natural_wonder_query: Vec<Option<NaturalWonder>>,
    pub resource_query: Vec<Option<(Resource, u32)>>,
    pub area_id_query: Vec<i32>,
    pub civilization_and_starting_tile: BTreeMap<String, Tile>,
    pub city_state_and_starting_tile: BTreeMap<String, Tile>,
    /// The area id and the size of the area
    pub area_id_and_size: BTreeMap<i32, u32>,
    region_list: Vec<Region>,
    /// Stores "impact and ripple" data in the layer. like [`TileMap::distance_data`].
    /// Layer contains the following:
    /// - `0`: Strategic
    /// - `1`: Luxury
    /// - `2`: Bonus
    /// - `3`: Fish
    /// - `4`: CityState
    /// - `5`: NaturalWonder
    /// - `6`: Marble
    layer_data: EnumMap<Layer, Vec<u32>>,
    /// Stores "impact and ripple" data of start points as each is placed. like [`TileMap::layer_data`].
    /// The value is in the range `[0, 99]`.
    /// The value is only related to the starting tile of civilization.
    /// - Value of 0 in a tile means no influence from existing Impacts in that tile.
    /// - Value of 99 means an Impact occurred in that tile and it is a starting tile.
    /// - Values > 0 and < 99 are "ripples", meaning that tile is near a starting tile.
    /// Higher values, closer to a starting tile.
    distance_data: Vec<u8>,
    /// Stores `impact` data only of start points, to avoid player collisions
    /// It is `true` When the tile has a civ start, CS start, or Natural Wonder.
    player_collision_data: Vec<bool>,
    // These tile will be as candidates for starting tile for city states
    uninhabited_areas_coastal_tiles: Vec<Tile>,
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

        let height = map_parameters.map_size.height;
        let width = map_parameters.map_size.width;

        let size = (height * width) as usize;

        let layer_data = enum_map! {
            _ => vec![0; size],
        };

        let region_list = Vec::with_capacity(map_parameters.civilization_num as usize);

        let city_state_region_assignments = vec![None; map_parameters.city_state_num as usize];

        Self {
            random_number_generator,
            map_size: map_parameters.map_size,
            river_list: Vec::new(),
            terrain_type_query: vec![TerrainType::Water; size],
            base_terrain_query: vec![BaseTerrain::Ocean; size],
            feature_query: vec![None; size],
            natural_wonder_query: vec![None; size],
            resource_query: vec![None; size],
            area_id_query: vec![-1; size],
            area_id_and_size: BTreeMap::new(),
            region_list,
            layer_data,
            distance_data: vec![0; size],
            player_collision_data: vec![false; size],
            civilization_and_starting_tile: BTreeMap::new(),
            city_state_and_starting_tile: BTreeMap::new(),
            uninhabited_areas_coastal_tiles: Vec::new(),
            uninhabited_areas_inland_tiles: Vec::new(),
            city_state_region_assignments,
            city_state_starting_tile_and_region_index: Vec::new(),
            luxury_resource_role: LuxuryResourceRole::default(),
            luxury_assign_to_region_count: HashMap::new(),
        }
    }

    /// Returns an iterator over all tiles in the map.
    pub fn iter_tiles(&self) -> impl Iterator<Item = Tile> {
        (0..((self.map_size.width * self.map_size.height) as usize)).map(Tile::new)
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// 1
    Strategic,
    /// 2
    Luxury,
    /// 3
    Bonus,
    /// 4
    Fish,
    /// 5
    CityState,
    /// 6
    NaturalWonder,
    /// 7
    Marble,
}
