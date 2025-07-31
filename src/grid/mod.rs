//! This module provides various grid implementations and utilities.
//! It includes support for different grid types such as hexagonal and square grids,
//! calculating distances, neighbors, and converting between grid coordinates and offset coordinates.

use direction::Direction;
use glam::Vec2;
use offset_coordinate::OffsetCoordinate;

use bitflags::bitflags;

pub mod direction;
pub mod hex_grid;
pub mod offset_coordinate;
pub mod square_grid;

/// Grid trait defines the interface for a grid structure.
///
/// Grid uses [`Cell`] representing each unique position in the grid.
/// Grids implement this trait with their specific coordinate types,
/// such as [`Hex`](hex_grid::hex::Hex) for hexagonal grids or [`Square`](square_grid::square::Square) for square grids.
/// their specific coordinate types is used to calculate the neighbors of a given coordinate,
/// distance between coordinates, and other grid-related operations.
pub trait Grid {
    /// The type of coordinate used in the grid.
    /// This type is used to calculate neighbors, distances, and other grid-related operations.
    /// For a hex grid, this would be [`Hex`](hex_grid::hex::Hex).
    /// For a square grid, this would be [`Square`](square_grid::square::Square).
    type GridCoordinateType;

    /// The type of the edge direction and corner array.
    /// It should be `[Direction; N]` where N is the number of edges or corners.
    /// For a hex grid, this would be `[Direction; 6]`.
    /// For a square grid, this would be `[Direction; 4]`.
    type DirectionArrayType;

    /// Returns the array of directions for the edges of the grid.
    fn edge_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the array of directions for the corners of the grid.
    fn corner_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the size of the grid as a [`Size`] struct, which contains the width and height of the grid.
    fn size(&self) -> Size;

    /// Returns the width of the grid.
    fn width(&self) -> u32 {
        self.size().width
    }

    /// Returns the height of the grid.
    fn height(&self) -> u32 {
        self.size().height
    }

    /// Returns the flags that indicate how a grid/map wraps at its borders.
    fn wrap_flags(&self) -> WrapFlags;

    /// Returns if the grid is wrapped in the X direction.
    fn wrap_x(&self) -> bool {
        self.wrap_flags().contains(WrapFlags::WrapX)
    }

    /// Returns if the grid is wrapped in the Y direction.
    fn wrap_y(&self) -> bool {
        self.wrap_flags().contains(WrapFlags::WrapY)
    }

    /// Get the center of the grid in pixel coordinates.
    ///
    /// # Notice
    /// When we show the map, we need to set camera to the center of the map.
    fn center(&self) -> Vec2;

    /// Converts a `Cell` to an `OffsetCoordinate`.
    ///
    /// `OffsetCoordinate` is a normalized coordinate that fits within the grid's bounds.
    ///
    /// # Panics
    ///
    /// If the cell is out of bounds, it will panic in debug mode.
    fn cell_to_offset(&self, cell: Cell) -> OffsetCoordinate {
        let width = self.size().width;
        let height = self.size().height;

        debug_assert!(
            cell.0 < (width * height) as usize,
            "Tile is out of bounds! Tile index: {}, Map size: {}x{}",
            cell.0,
            width,
            height
        );

        let x = cell.0 as u32 % width;
        let y = cell.0 as u32 / width;

        OffsetCoordinate::from([x, y])
    }

    /// Converts an `OffsetCoordinate`  to a `Cell`. If the coordinate is out of bounds, an error is returned.
    ///
    /// # Arguments
    ///
    /// - `offset_coordinate` - The offset coordinate to convert.
    ///
    /// # Returns
    ///
    /// - `Result<Cell, String>`: The cell if the coordinate is valid, otherwise an error message.
    fn offset_to_cell(&self, offset_coordinate: OffsetCoordinate) -> Result<Cell, String> {
        self.normalize_offset(offset_coordinate)
            .map(|normalized_coordinate| {
                let [x, y] = normalized_coordinate.to_array();
                Cell((x + y * self.size().width as i32) as usize)
            })
    }

    /// Normalizes an offset coordinate to fit within the grid's bounds. If the coordinate is out of bounds, an error is returned.
    ///
    /// # Returns
    ///
    /// Returns a normalized `OffsetCoordinate` that fits within the grid's bounds or an error message.
    /// The normalized `OffsetCoordinate` should meet the conditions:
    /// - x ∈ [0, width)
    /// - y ∈ [0, height)
    ///
    /// If the coordinate is out of bounds, an error is returned.
    /// - If the grid is not wrapped in the X direction, and `offset_coordinate`'s `x` is out of bounds, i.e., not in the range `[0, width)`, the function will return an error.
    /// - If the grid is not wrapped in the Y direction, and `offset_coordinate`'s `y` is out of bounds, i.e., not in the range `[0, height)`, the function will return an error.
    ///
    fn normalize_offset(
        &self,
        offset_coordinate: OffsetCoordinate,
    ) -> Result<OffsetCoordinate, String> {
        let mut x = offset_coordinate.0.x;
        let mut y = offset_coordinate.0.y;

        if self.wrap_x() {
            x = x.rem_euclid(self.width() as i32);
        }
        if self.wrap_y() {
            y = y.rem_euclid(self.height() as i32);
        }

        let offset_coordinate = OffsetCoordinate::new(x, y);

        if self.within_grid_bounds(offset_coordinate) {
            Ok(offset_coordinate)
        } else {
            Err(format!(
                "Offset coordinate out of bounds: {:?}",
                offset_coordinate
            ))
        }
    }

    /// Checks if the given `OffsetCoordinate` is within the grid's bounds.
    fn within_grid_bounds(&self, offset_coordinate: OffsetCoordinate) -> bool {
        offset_coordinate.0.x >= 0
            && offset_coordinate.0.x < self.width() as i32
            && offset_coordinate.0.y >= 0
            && offset_coordinate.0.y < self.height() as i32
    }

    /// Convert `GridCoordinateType` to a [`Cell`] in the grid.
    ///
    /// If the grid coordinate is out of bounds, it will return `None`.
    fn grid_coordinate_to_cell(&self, grid_coordinate: Self::GridCoordinateType) -> Option<Cell>;

    /// Computes the distance from `start` to `dest` in the grid.
    fn distance_to(&self, start: Cell, dest: Cell) -> i32;

    /// Returns the neighbor of `center` in the given `direction`.
    fn neighbor(self, center: Cell, direction: Direction) -> Option<Cell>;

    /// Returns an iterator over all grid cells that are at a distance of `distance` from `center`.
    ///
    /// # Arguments
    ///
    /// * `center` - The center cell.
    /// * `distance` - The distance from the center cell.
    ///
    /// # Notice
    ///
    /// Before calling:
    /// - For WarpX grids: `distance` should be ≤ `self.width() / 2`.
    /// - For WarpY grids: `distance` should be ≤ `self.height() / 2`.
    ///
    /// If you need distances beyond these limits (not recommended), filter results like this:
    /// ```rust, ignore
    /// let cells = grid.cells_at_distance(center, distance)
    /// .filter(|cell| {
    ///     grid.distance_to(center, *cell) == distance as i32
    /// }).collect::<Vec<_>>();
    /// ```
    ///
    fn cells_at_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Returns an iterator over all grid cells that are within a distance of `distance` from `center`.
    /// This includes the center cell itself.
    ///
    /// # Arguments
    ///
    /// - `center`: The center cell.
    /// - `distance`: The distance from the center cell.
    ///
    /// # Notice
    ///
    /// Before calling:
    /// - For WarpX grids: `distance` should be ≤ `self.width() / 2`.
    /// - For WarpY grids: `distance` should be ≤ `self.height() / 2`.
    ///
    /// If you need distances beyond these limits (not recommended), remove duplicate results like this:
    /// ```rust, ignore
    /// let cells = grid.cells_within_distance(center, distance)
    /// .collect::<HashSet<_>>();
    /// ```
    ///
    fn cells_within_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Determine the direction of `dest` relative to `start`.
    ///
    /// If `dest` is located to the north of `start`, the function returns `Some(Direction::North)`.
    /// If `dest` is equal to `start`, the function returns [`None`].
    fn estimate_direction(&self, start: Cell, dest: Cell) -> Option<Direction>;
}

/// Represents the size of a grid or map with a specified width and height.
#[derive(Clone, Copy)]
pub struct Size {
    /// The width of the grid or map.
    pub width: u32,
    /// The height of the grid or map.
    pub height: u32,
}

impl Size {
    /// Create a new `Size` with the specified width and height.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns how many cells are in the grid.
    ///
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

bitflags! {
    /// Bitflags representing how a grid/map wraps at its borders.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct WrapFlags: u8 {
        ///Enable horizontal wrapping (left/right edges connect)
        const WrapX = 0b0000_0001;
        /// Enable vertical wrapping (top/bottom edges connect)
        const WrapY = 0b0000_0010;
    }
}

/// Representing a cell or tile in a grid, which is identified by a unique index.
///
/// It is a wrapper around a `usize` index, which uniquely identifies the cell within the grid.
/// The index is used to access the cell in a flat representation of the grid, such as a 1D array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell(usize);

impl Cell {
    /// Creates a new `Cell` with the given index.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the index of the cell in the grid.
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Trait for grids that can determine their world size type based on their size and provide a default size based on [WorldSizeType].
pub trait GridSize: Grid {
    /// Get world size type of the grid based on its size.
    ///
    fn world_size_type(&self) -> WorldSizeType {
        let width = self.width();
        let height = self.height();
        let area = width * height;

        // Get the threshold areas for each world size from default_size
        let duel_area = Self::default_size(WorldSizeType::Duel).area();
        let tiny_area = Self::default_size(WorldSizeType::Tiny).area();
        let small_area = Self::default_size(WorldSizeType::Small).area();
        let standard_area = Self::default_size(WorldSizeType::Standard).area();
        let large_area = Self::default_size(WorldSizeType::Large).area();
        let huge_area = Self::default_size(WorldSizeType::Huge).area();

        match area {
            // When area < duel_area, show warning
            area if area < duel_area => {
                eprintln!(
                "The map size is too small. The provided dimensions are {}x{}, which gives an area of {}. The minimum area is {} in the original CIV5 game.",
                width, height, area, duel_area
            );
                WorldSizeType::Duel
            }
            // Compare with each threshold
            area if area < tiny_area => WorldSizeType::Duel,
            area if area < small_area => WorldSizeType::Tiny,
            area if area < standard_area => WorldSizeType::Small,
            area if area < large_area => WorldSizeType::Standard,
            area if area < huge_area => WorldSizeType::Large,
            _ => WorldSizeType::Huge,
        }
    }

    /// Get the default size of the grid based on its world size type.
    /// This is used to initialize the grid with a default size.
    fn default_size(world_size_type: WorldSizeType) -> Size;
}

/// Defines standard world size type presets for game maps or environments.
///
/// Variants represent different scale levels from smallest to largest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorldSizeType {
    Duel,
    Tiny,
    Small,
    Standard,
    Large,
    Huge,
}
