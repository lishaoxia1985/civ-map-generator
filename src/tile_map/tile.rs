use glam::DVec2;

use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        resource::Resource, terrain_type::TerrainType,
    },
    grid::{
        hex::{Hex, HexOrientation},
        Direction, OffsetCoordinate,
    },
    ruleset::Ruleset,
};

use super::{tile_map_impls::generate_regions::Region, Layer, MapParameters, TileMap, WrapType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// `Tile` represents a tile on the map, where the `usize` is the index of the current tile.
///
/// The index indicates the tile's position on the map, typically used to access or reference specific tiles.
pub struct Tile(usize);

impl Tile {
    #[inline]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Get the index of the tile。
    ///
    /// The index indicates the tile's position on the map, typically used to access or reference specific tiles.
    #[inline]
    pub const fn index(&self) -> usize {
        self.0
    }

    /// Converts an offset coordinate to the corresponding tile based on map parameters.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to the map parameters, which includes map size and wrapping behavior.
    /// - `offset_coordinate`: The offset coordinate to convert.
    ///
    /// # Returns
    /// - `Result<Self, String>`: Returns an instance of `Self` if the coordinate is valid,
    ///   or an error message if the coordinate is outside the map bounds.
    pub fn from_offset_coordinate(
        map_parameters: &MapParameters,
        offset_coordinate: OffsetCoordinate,
    ) -> Result<Self, String> {
        let map_size = map_parameters.map_size;
        let width = map_parameters.map_size.width as i32;
        let height = map_parameters.map_size.height as i32;
        // Check if the offset coordinate is inside the map
        let [mut x, mut y] = offset_coordinate.to_array();

        if map_parameters.map_wrapping.x == WrapType::Wrap {
            x = x.rem_euclid(width);
        };
        if map_parameters.map_wrapping.y == WrapType::Wrap {
            y = y.rem_euclid(height);
        };

        if x >= 0 && x < width && y >= 0 && y < height {
            let index = (x + y * map_size.width) as usize;
            Ok(Self(index))
        } else {
            Err(String::from("Offset coordinate is outside the map!"))
        }
    }

    /// Converts a tile to the corresponding offset coordinate based on map parameters.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the dimensions of the map.
    ///
    /// # Returns
    /// Returns an `OffsetCoordinate` that corresponds to the provided tile, calculated based on the map parameters.
    /// This coordinate represents the position of the tile within the map grid.
    ///
    /// # Panics
    /// This method will panic if the tile is out of bounds for the given map size.
    pub fn to_offset_coordinate(&self, map_parameters: &MapParameters) -> OffsetCoordinate {
        let map_width = map_parameters.map_size.width;
        let map_height = map_parameters.map_size.height;

        assert!(
            self.0 < (map_width * map_height) as usize,
            "Index out of bounds"
        );

        let x = self.0 as i32 % map_width;
        let y = self.0 as i32 / map_width;

        OffsetCoordinate::new(x, y)
    }

    /// Converts the current tile to a hexagonal coordinate based on the map parameters.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the dimensions and layout of the map.
    ///
    /// # Returns
    /// Returns a `Hex` coordinate that corresponds to the provided map position, calculated based on the map parameters.
    /// This coordinate represents the position in hexagonal space within the map grid.
    ///
    /// # Panics
    /// This method will panic if the tile is out of bounds for the given map size.
    pub fn to_hex_coordinate(&self, map_parameters: &MapParameters) -> Hex {
        // We don't need to check if the index is valid here, as it has already been checked in `to_offset_coordinate`
        self.to_offset_coordinate(map_parameters)
            .to_hex(map_parameters.offset, map_parameters.hex_layout.orientation)
    }

    /// Calculates the latitude of the tile on the tile map.
    ///
    /// The latitude is defined such that:
    /// - The equator corresponds to a latitude of `0.0`.
    /// - The poles correspond to a latitude of `1.0`.
    ///
    /// As the latitude value approaches `0.0`, the tile is closer to the equator,
    /// while a value approaching `1.0` indicates proximity to the poles.
    ///
    /// # Parameters
    /// - `map_parameters`: A reference to `MapParameters`, which contains the size and dimensions of the map.
    ///
    /// # Returns
    /// A `f64` representing the latitude of the tile, with values ranging from `0.0` (equator) to `1.0` (poles).
    ///
    /// # Panics
    /// This method will panic if the tile is out of bounds for the given map size.
    pub fn latitude(&self, map_parameters: &MapParameters) -> f64 {
        // We don't need to check if the index is valid here, as it has already been checked in `to_offset_coordinate`
        let y = self.to_offset_coordinate(map_parameters).0.y;
        let half_height = map_parameters.map_size.height as f64 / 2.0;
        ((half_height - y as f64) / half_height).abs()
    }

    /// Returns the terrain type of the tile at the given index.
    #[inline]
    pub fn terrain_type(&self, tile_map: &TileMap) -> TerrainType {
        tile_map.terrain_type_query[self.0]
    }

    /// Returns the base terrain of the tile at the given index.
    #[inline]
    pub fn base_terrain(&self, tile_map: &TileMap) -> BaseTerrain {
        tile_map.base_terrain_query[self.0]
    }

    /// Returns the feature of the tile at the given index.
    #[inline]
    pub fn feature(&self, tile_map: &TileMap) -> Option<Feature> {
        tile_map.feature_query[self.0]
    }

    /// Returns the natural wonder of the tile at the given index.
    #[inline]
    pub fn natural_wonder(&self, tile_map: &TileMap) -> Option<NaturalWonder> {
        tile_map.natural_wonder_query[self.0].clone()
    }

    /// Returns the resource of the tile at the given index.
    #[inline]
    pub fn resource(&self, tile_map: &TileMap) -> Option<(Resource, u32)> {
        tile_map.resource_query[self.0].clone()
    }

    /// Returns the area id of the tile at the given index.
    #[inline]
    pub fn area_id(&self, tile_map: &TileMap) -> i32 {
        tile_map.area_id_query[self.0]
    }

    pub fn neighbor_tiles<'a>(&'a self, map_parameters: &MapParameters) -> Vec<Self> {
        self.tiles_at_distance(1, map_parameters)
    }

    /// Retrieves the neighboring tile from the current tile in the specified direction.
    ///
    /// # Parameters
    /// - `direction`: The direction to locate the neighboring tile.
    /// - `map_parameters`: A reference to the map parameters that include layout and offset information.
    ///
    /// # Returns
    /// An `Option<TileIndex>`. This is `Some` if the neighboring tile exists,
    /// or `None` if the neighboring tile is invalid.
    ///
    /// # Panics
    /// This method will panic if the current tile is out of bounds for the given map size.
    pub fn neighbor_tile<'a>(
        &'a self,
        direction: Direction,
        map_parameters: &MapParameters,
    ) -> Option<Self> {
        let orientation = map_parameters.hex_layout.orientation;
        // We don't need to check if the tile is valid here, as it has already been checked in `to_hex_coordinate`
        let neighbor_offset_coordinate = self
            .to_hex_coordinate(map_parameters)
            .neighbor(orientation, direction)
            .to_offset_coordinate(map_parameters.offset, orientation);

        Self::from_offset_coordinate(map_parameters, neighbor_offset_coordinate).ok()
    }

    /// Get the tiles at the given distance from the current tile.
    pub fn tiles_at_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<Self> {
        // We don't need to check if the index is valid here, as it has already been checked in `to_hex_coordinate`
        let hex = self.to_hex_coordinate(map_parameters);
        hex.hexes_at_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                Self::from_offset_coordinate(map_parameters, offset_coordinate).ok()
            })
            .collect()
    }

    /// Get the tiles within the given distance from the current tile, including the current tile.
    pub fn tiles_in_distance<'a>(
        &'a self,
        distance: u32,
        map_parameters: &MapParameters,
    ) -> Vec<Self> {
        // We don't need to check if the tile is valid here, as it has already been checked in `to_hex_coordinate`
        let hex = self.to_hex_coordinate(map_parameters);
        hex.hexes_in_distance(distance)
            .iter()
            .filter_map(|hex_coordinate| {
                let offset_coordinate = hex_coordinate.to_offset_coordinate(
                    map_parameters.offset,
                    map_parameters.hex_layout.orientation,
                );

                Self::from_offset_coordinate(map_parameters, offset_coordinate).ok()
            })
            .collect()
    }

    pub fn pixel_position(&self, map_parameters: &MapParameters) -> DVec2 {
        // We donn't need to check if the tile is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(map_parameters);
        map_parameters.hex_layout.hex_to_pixel(hex)
    }

    pub fn corner_position(&self, direction: Direction, map_parameters: &MapParameters) -> DVec2 {
        // We donn't need to check if the tile is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(map_parameters);
        map_parameters.hex_layout.corner(hex, direction)
    }

    /// Checks if there is a river on the current tile.
    ///
    /// # Parameters
    /// - `tile_map`: A reference to the TileMap containing river information.
    /// - `map_parameters`: A reference to the map parameters, which include hex layout settings.
    /// # Returns
    /// - `bool`: Returns true if there is a river on the current tile, false otherwise.
    pub fn has_river(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        map_parameters
            .edge_direction_array()
            .iter()
            .any(|&direction| self.has_river_in_direction(direction, tile_map, map_parameters))
    }

    /// Checks if there is a river on the current tile in the specified direction.
    ///
    /// # Parameters
    /// - `direction`: The direction to check for the river.
    /// - `tile_map`: A reference to the TileMap containing river information.
    /// - `map_parameters`: A reference to the map parameters, which include hex layout settings.
    ///
    /// # Returns
    /// - `bool`: Returns true if there is a river in the specified direction, false otherwise.
    pub fn has_river_in_direction(
        &self,
        direction: Direction,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        // Get the edge index for the specified direction.
        let edge_index = map_parameters.hex_layout.orientation.edge_index(direction);

        // Determine the tile and edge direction to check based on the edge index.
        let (check_tile, check_edge_direction) = if edge_index < 3 {
            // If the edge index is less than 3, use the current tile and the given direction.
            (*self, direction)
        } else {
            // Otherwise, check the neighboring tile and the opposite direction.
            match self.neighbor_tile(direction, map_parameters) {
                Some(neighbor_tile) => (neighbor_tile, direction.opposite_direction()),
                None => return false,
            }
        };

        tile_map.river_list.values().flatten().any(
            |&(tile, flow_direction)| {
                tile == check_tile // 1. Check whether there is a river in the current tile.
                    && check_edge_direction == edge_direction_for_flow_direction(flow_direction, map_parameters) // 2. Check whether the river edge in the direction of the current tile.
            })
    }

    pub fn is_impassable(&self, tile_map: &TileMap, ruleset: &Ruleset) -> bool {
        self.terrain_type(tile_map) == TerrainType::Mountain
            || self
                .feature(tile_map)
                .map_or(false, |feature| feature.impassable(ruleset))
            || self
                .natural_wonder(tile_map)
                .map_or(false, |natural_wonder| natural_wonder.impassable(ruleset))
    }

    /// Check if the tile is freshwater
    ///
    /// Freshwater is not water and is adjacent to lake, oasis or has a river
    pub fn is_freshwater(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        self.terrain_type(tile_map) != TerrainType::Water
            && (self.neighbor_tiles(map_parameters).iter().any(|tile| {
                tile.base_terrain(tile_map) == BaseTerrain::Lake
                    || tile.feature(tile_map) == Some(Feature::Oasis)
            }) || self.has_river(tile_map, map_parameters))
    }

    /// Check if the tile is coastal land.
    ///
    /// A tile is considered `coastal land` if it is not `Water` and has at least one neighboring tile that is `Coast`.
    /// # Notice
    /// If the tile is not `Water` and has at least one neighboring tile that is `Lake`, but it has no neighboring tile that is `Coast`, it is not `coastal land`.
    pub fn is_coastal_land(&self, tile_map: &TileMap, map_parameters: &MapParameters) -> bool {
        self.terrain_type(tile_map) != TerrainType::Water
            && self
                .neighbor_tiles(map_parameters)
                .iter()
                .any(|&tile| tile.base_terrain(tile_map) == BaseTerrain::Coast)
    }

    /// Checks if a tile can be a starting tile of civilization.
    ///
    /// A tile is initially considered a starting tile if it is either `Flatland` or `Hill`, and then it must meet one of the following conditions:
    /// 1. The tile is a coastal land.
    /// 2. If `civilization_starting_tile_must_be_coastal_land` is `false`, An inland tile (whose distance to `Coast` is greater than 2) can be a starting tile as well.
    ///
    /// # Why Tiles with Distance 2 from Coast are Excluded:
    /// Tiles with a distance of 2 from the coast are excluded because in the original game, the `Settler` unit can move 2 tiles per turn (ignoring terrain movement cost).
    /// If such a tile were considered a starting tile, a `Settler` can move to the coastal land and build a city in just one turn, which is functionally equivalent to choosing a coastal land tile as the starting tile of civilization directly.
    ///
    /// # Notice
    /// The tile with nature wonder can not be a starting tile of civilization.
    /// Doesn't like Civ6, in original Civ5, we generate the nature wonder after generating the civilization starting tile, so in this function, we don't check the nature wonder.
    /// City state starting tile is the same as well.
    pub fn can_be_civilization_starting_tile(
        &self,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        // This variable is the maximum distance a Settler can move.
        // It can be customized in the MapParameters in the future.
        const SETTLER_MOVEMENT: u32 = 2;
        matches!(
            self.terrain_type(tile_map),
            TerrainType::Flatland | TerrainType::Hill
        ) && (self.is_coastal_land(tile_map, map_parameters)
            || (!map_parameters.civilization_starting_tile_must_be_coastal_land
                && self
                    .tiles_in_distance(SETTLER_MOVEMENT, map_parameters)
                    .iter()
                    .all(|tile| tile.base_terrain(tile_map) != BaseTerrain::Coast)))
    }

    /// Checks if a tile can be a starting tile of city state.
    ///
    /// A tile is initially considered a starting tile if it is either `Flatland` or `Hill`, and then it must meet all of the following conditions:
    /// 1. The tile is not `Snow`.
    /// 2. - If `force_it` is `true`, ignores whether the tile is in the influence of other city states.
    ///    - If `false`, the tile must not be in the influence of other city states.
    /// 3. - If `ignore_collisions` is `true`, ignores whether the tile has been placed a city state, a civilization, or a natural wonder.
    ///    - If `false`, the tile must not have been placed a city state, civilization, or natural wonder.
    /// # Parameters
    /// - `tile_map`: A reference to `TileMap`, which contains the tile data.
    /// - `region`: An optional reference to `Region`, which represents the region where the city state is located.\
    /// If `None`, the function considers the tile as a candidate regardless of its region.
    /// That usually happens when we place a city state in a unhabitated area.
    /// - `force_it`: A boolean flag indicating whether to force the tile to be a candidate regardless of whether it is in the influence of another city state.
    /// If `true`, the function ignores whether the tile is in the influence of other city states.
    /// - `ignore_collisions`: A boolean flag indicating whether to ignore the tile has been placed a city state, a civilization, or a natural wonder.
    /// If `true`, the function ignores the tile has been placed a city state, civilization, or natural wonder.
    pub fn can_be_city_state_starting_tile(
        &self,
        tile_map: &TileMap,
        region: Option<&Region>,
        force_it: bool,
        ignore_collisions: bool,
    ) -> bool {
        matches!(
            self.terrain_type(tile_map),
            TerrainType::Flatland | TerrainType::Hill
        ) && region.map_or(true, |region| {
            Some(self.area_id(tile_map)) == region.landmass_id
        }) && self.base_terrain(tile_map) != BaseTerrain::Snow
            && (tile_map.layer_data[&Layer::CityState][self.index()] == 0 || force_it)
            && (tile_map.player_collision_data[self.index()] == false || ignore_collisions)
    }
}

/// Returns the edge direction that corresponds to a given flow direction in a hexagonal grid,
/// based on the specified layout orientation.
///
/// This function maps flow directions to their respective edge directions within a hexagonal
/// layout, accounting for both pointy and flat orientations.
///
/// # Parameters
/// - `flow_direction`: The direction of the river flow.
/// - `map_parameters`: A reference to `MapParameters`, which contains the hexagonal layout orientation.
///
/// # Returns
/// The corresponding edge direction refers to the direction of the river edge located on the current tile.
/// For example, when hex is `HexOrientation::Pointy`, if the river is flowing North or South, the edge direction is East.
///
/// # Panics
/// This function will panic if an invalid flow direction is provided.
fn edge_direction_for_flow_direction(
    flow_direction: Direction,
    map_parameters: &MapParameters,
) -> Direction {
    match map_parameters.hex_layout.orientation {
        HexOrientation::Pointy => match flow_direction {
            Direction::North | Direction::South => Direction::East,
            Direction::NorthEast | Direction::SouthWest => Direction::SouthEast,
            Direction::NorthWest | Direction::SouthEast => Direction::SouthWest,
            _ => panic!("Invalid flow direction"),
        },
        HexOrientation::Flat => match flow_direction {
            Direction::NorthWest | Direction::SouthEast => Direction::NorthEast,
            Direction::NorthEast | Direction::SouthWest => Direction::SouthEast,
            Direction::East | Direction::West => Direction::South,
            _ => panic!("Invalid flow direction"),
        },
    }
}
