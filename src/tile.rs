use glam::Vec2;

use crate::{
    component::map_component::{
        base_terrain::BaseTerrain, feature::Feature, natural_wonder::NaturalWonder,
        resource::Resource, terrain_type::TerrainType,
    },
    grid::{
        direction::Direction,
        hex_grid::{
            hex::{Hex, HexOrientation},
            HexGrid,
        },
        offset_coordinate::OffsetCoordinate,
        Cell, Grid,
    },
    map_parameters::MapParameters,
    ruleset::Ruleset,
    tile_map::{impls::generate_regions::Region, Layer, TileMap},
};

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

    /// Creates a `Tile` from an `OffsetCoordinate` according to the specified `HexGrid`.
    ///
    pub fn from_offset(offset_coordinate: OffsetCoordinate, grid: HexGrid) -> Self {
        let cell = grid
            .offset_to_cell(offset_coordinate)
            .expect("Offset coordinate is out of bounds for the grid size");
        Self::from_cell(cell)
    }

    /// Creates a `Tile` from a `Cell`.
    ///
    #[inline(always)]
    pub fn from_cell(cell: Cell) -> Self {
        Self(cell.index())
    }

    #[inline(always)]
    pub fn to_cell(&self) -> Cell {
        Cell::new(self.0)
    }

    /// Get the index of the tile.
    ///
    /// The index indicates the tile's position on the map, typically used to access or reference specific tiles.
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.0
    }

    /// Converts a tile to the corresponding offset coordinate based on grid parameters.
    ///
    /// # Arguments
    ///
    /// - `grid`: A `HexGrid` that contains the map size information.
    ///
    /// # Returns
    /// Returns an `OffsetCoordinate` that corresponds to the provided tile, calculated based on the grid parameters.
    /// This coordinate represents the position of the tile within the map grid.
    ///
    pub fn to_offset(&self, grid: HexGrid) -> OffsetCoordinate {
        grid.cell_to_offset(self.to_cell())
    }

    /// Converts the current tile to a hexagonal coordinate based on the map parameters.
    ///
    /// # Returns
    /// Returns a `Hex` coordinate that corresponds to the provided map position, calculated based on the map grid parameters.
    /// This coordinate represents the position in hexagonal space within the map grid.
    ///
    /// # Panics
    /// This method will panic if the tile is out of bounds for the given map size.
    pub fn to_hex_coordinate(&self, grid: HexGrid) -> Hex {
        let offset_coordinate = self.to_offset(grid);
        Hex::from_offset(offset_coordinate, grid.layout.orientation, grid.offset)
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
    /// # Arguments
    ///
    /// - `grid`: A `HexGrid` that contains the map size information.
    ///
    /// # Returns
    ///
    /// A `f64` representing the latitude of the tile, with values ranging from `0.0` (equator) to `1.0` (poles).
    ///
    /// # Panics
    ///
    /// This method will panic if the tile is out of bounds for the given map size.
    pub fn latitude(&self, grid: HexGrid) -> f64 {
        // We don't need to check if the index is valid here, as it has already been checked in `to_offset_coordinate`
        let y = self.to_offset(grid).0.y;
        let half_height = grid.height() as f64 / 2.0;
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

    /// Returns the area ID of the tile at the given index.
    #[inline]
    pub fn area_id(&self, tile_map: &TileMap) -> usize {
        tile_map.area_id_query[self.0]
    }

    /// Returns the landmass ID of the tile at the given index.
    #[inline]
    pub fn landmass_id(&self, tile_map: &TileMap) -> usize {
        tile_map.landmass_id_query[self.0]
    }

    /// Returns an iterator over the neighboring tiles of the current tile.
    ///
    pub fn neighbor_tiles(&self, grid: HexGrid) -> impl Iterator<Item = Self> {
        self.tiles_at_distance(1, grid)
    }

    /// Retrieves the neighboring tile from the current tile in the specified direction.
    ///
    /// # Arguments
    ///
    /// - `direction`: The direction to locate the neighboring tile.
    /// - `grid`: The grid parameters that include layout and offset information.
    ///
    /// # Returns
    ///
    /// An `Option<Tile>`. This is `Some` if the neighboring tile exists,
    /// or `None` if the neighboring tile is invalid.
    ///
    /// # Panics
    ///
    /// This method will panic if the current tile is out of bounds for the given map size.
    pub fn neighbor_tile(&self, direction: Direction, grid: HexGrid) -> Option<Self> {
        grid.neighbor(self.to_cell(), direction)
            .map(Self::from_cell)
    }

    /// Returns an iterator over the tiles at the given distance from the current tile.
    ///
    pub fn tiles_at_distance(&self, distance: u32, grid: HexGrid) -> impl Iterator<Item = Self> {
        grid.cells_at_distance(self.to_cell(), distance)
            .map(Self::from_cell)
    }

    /// Returns an iterator over the tiles within the given distance from the current tile, including the current tile.
    ///
    pub fn tiles_in_distance(&self, distance: u32, grid: HexGrid) -> impl Iterator<Item = Self> {
        grid.cells_within_distance(self.to_cell(), distance)
            .map(Self::from_cell)
    }

    pub fn pixel_position(&self, grid: HexGrid) -> Vec2 {
        // We donn't need to check if the tile is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(grid);
        grid.layout.hex_to_pixel(hex)
    }

    pub fn corner_position(&self, direction: Direction, grid: HexGrid) -> Vec2 {
        // We donn't need to check if the tile is valid here, because the caller should have done that.
        let hex = self.to_hex_coordinate(grid);
        grid.layout.corner(hex, direction)
    }

    /// Checks if there is a river on the current tile.
    ///
    /// # Arguments
    ///
    /// - `tile_map`: A reference to the [`TileMap`] containing river information.
    ///
    /// # Returns
    ///
    /// - `bool`: Returns true if there is a river on the current tile, false otherwise.
    pub fn has_river(&self, tile_map: &TileMap) -> bool {
        let grid = tile_map.world_grid.grid;
        grid.edge_direction_array()
            .iter()
            .any(|&direction| self.has_river_in_direction(direction, tile_map))
    }

    /// Checks if there is a river on the current tile in the specified direction.
    ///
    /// # Arguments
    ///
    /// - `direction`: The direction to check for the river.
    /// - `tile_map`: A reference to the [`TileMap`] containing river information.
    ///
    /// # Returns
    ///
    /// - `bool`: Returns true if there is a river in the specified direction, false otherwise.
    pub fn has_river_in_direction(&self, direction: Direction, tile_map: &TileMap) -> bool {
        let grid = tile_map.world_grid.grid;
        // Get the edge index for the specified direction.
        let edge_index = grid.layout.orientation.edge_index(direction);

        // Determine the tile and edge direction to check based on the edge index.
        let (check_tile, check_edge_direction) = if edge_index < 3 {
            // If the edge index is less than 3, use the current tile and the given direction.
            (*self, direction)
        } else {
            // Otherwise, check the neighboring tile and the opposite direction.
            match self.neighbor_tile(direction, grid) {
                Some(neighbor_tile) => (neighbor_tile, direction.opposite()),
                None => return false,
            }
        };

        tile_map.river_list.iter().flatten().any(
            |&(tile, flow_direction)| {
                tile == check_tile // 1. Check whether there is a river in the current tile.
                    && check_edge_direction == edge_direction_for_flow_direction(flow_direction, grid) // 2. Check whether the river edge in the direction of the current tile.
            })
    }

    /// Checks if the tile is water.
    ///
    /// When tile's terrain type is [`TerrainType::Water`], it is considered water.
    /// Otherwise, it is not water.
    pub fn is_water(&self, tile_map: &TileMap) -> bool {
        self.terrain_type(tile_map) == TerrainType::Water
    }

    /// Checks if the tile is impassable.
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
    pub fn is_freshwater(&self, tile_map: &TileMap) -> bool {
        let grid = tile_map.world_grid.grid;
        self.terrain_type(tile_map) != TerrainType::Water
            && (self.neighbor_tiles(grid).any(|tile| {
                tile.base_terrain(tile_map) == BaseTerrain::Lake
                    || tile.feature(tile_map) == Some(Feature::Oasis)
            }) || self.has_river(tile_map))
    }

    /// Check if the tile is coastal land.
    ///
    /// A tile is considered `coastal land` if it is not `Water` and has at least one neighboring tile that is `Coast`.
    ///
    /// # Notice
    ///
    /// If the tile is not `Water` and has at least one neighboring tile that is `Lake`, but it has no neighboring tile that is `Coast`, it is not `coastal land`.
    pub fn is_coastal_land(&self, tile_map: &TileMap) -> bool {
        let grid = tile_map.world_grid.grid;
        self.terrain_type(tile_map) != TerrainType::Water
            && self
                .neighbor_tiles(grid)
                .any(|tile| tile.base_terrain(tile_map) == BaseTerrain::Coast)
    }

    /// Checks if a tile can be a starting tile of civilization.
    ///
    /// A tile is considered a starting tile if it is either `Flatland` or `Hill`, and then it must meet one of the following conditions:
    /// 1. The tile is a coastal land.
    /// 2. If `civ_require_coastal_land_start` is `false`, An inland tile (whose distance to `Coast` is greater than 2) can be a starting tile as well.
    ///
    /// **Why Inland Tiles with Distance 2 from Coast are Excluded**
    ///
    /// Because in the original game, the `Settler` unit can move 2 tiles per turn (ignoring terrain movement cost).
    /// If such a tile were considered a starting tile, a `Settler` can move to the coastal land and build a city in just one turn, which is functionally equivalent to choosing a coastal land tile as the starting tile of civilization directly.
    ///
    /// # Notice
    ///
    /// The tile with nature wonder can not be a starting tile of civilization.
    /// But we don't check the nature wonder in this function, because we generate the nature wonder after generating the civilization starting tile.
    /// That's like in original CIV5.
    /// City state starting tile is the same as well.
    /// In CIV6, we should check the nature wonder in this function.
    pub fn can_be_civilization_starting_tile(
        &self,
        tile_map: &TileMap,
        map_parameters: &MapParameters,
    ) -> bool {
        // This variable is the maximum distance a Settler can move.
        // TODO: It can be customized in the MapParameters in the future.
        const SETTLER_MOVEMENT: u32 = 2;
        matches!(
            self.terrain_type(tile_map),
            TerrainType::Flatland | TerrainType::Hill
        ) && (self.is_coastal_land(tile_map)
            || (!map_parameters.civ_require_coastal_land_start
                && self
                    .tiles_in_distance(SETTLER_MOVEMENT, tile_map.world_grid.grid)
                    .all(|tile| tile.base_terrain(tile_map) != BaseTerrain::Coast)))
    }

    /// Checks if a tile can be a starting tile of city state.
    ///
    /// A tile is considered a starting tile, it must meet all of the following conditions:
    /// 1. It is either `Flatland` or `Hill`.
    /// 2. It is not `Snow`.
    ///
    /// # Arguments
    ///
    /// - `tile_map`: A reference to `TileMap`, which contains the tile data.
    /// - `region`: An optional reference to `Region`, which represents the region where the city state is located.\
    ///   If `None`, the function considers the tile as a candidate regardless of its region.
    ///   That usually happens when we place a city state in a unhabitated area.
    pub fn can_be_city_state_starting_tile(
        &self,
        tile_map: &TileMap,
        region: Option<&Region>,
    ) -> bool {
        matches!(
            self.terrain_type(tile_map),
            TerrainType::Flatland | TerrainType::Hill
        ) && region.map_or(true, |region| {
            Some(self.area_id(tile_map)) == region.area_id
        }) && self.base_terrain(tile_map) != BaseTerrain::Snow
            && (tile_map.layer_data[Layer::CityState][self.index()] == 0)
            && (!tile_map.player_collision_data[self.index()])
    }
}

/// Returns the edge direction that corresponds to a given flow direction in the grid.
///
/// # Arguments
///
/// - `flow_direction`: The direction of the river flow.
/// - `grid`: The `HexGrid` that contains the layout and orientation information.
///
/// # Returns
///
/// The corresponding edge direction refers to the direction of the river edge located on the current tile.
/// For example, when hex is `HexOrientation::Pointy`, if the river is flowing North or South, the edge direction is East.
///
/// # Panics
///
/// This function will panic if an invalid flow direction is provided.
fn edge_direction_for_flow_direction(flow_direction: Direction, grid: HexGrid) -> Direction {
    match grid.layout.orientation {
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
