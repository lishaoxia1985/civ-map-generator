use direction::Direction;
use offset_coordinate::OffsetCoordinate;

use bitflags::bitflags;

pub mod direction;
pub mod hex_grid;
pub mod offset_coordinate;
pub mod square_grid;

trait Grid {
    /// The type of coordinate used in the grid.
    /// For example, for a hex grid, this would be `Hex`.
    /// For a square grid, this would be `Square`.
    type CoordinateType;

    fn edge_direction_array<const N: usize>(&self) -> [Direction; N];

    fn corner_direction_array<const N: usize>(&self) -> [Direction; N];

    /// Returns the width of the grid.
    fn width(&self) -> i32;

    /// Returns the height of the grid.
    fn height(&self) -> i32;

    /// Returns if the grid is wrapped in the X direction.
    fn wrap_x(&self) -> bool;

    /// Returns if the grid is wrapped in the Y direction.
    fn wrap_y(&self) -> bool;

    /// Converts a coordinate of the grid to an offset coordinate.
    fn to_offset_coordinate(&self, offset_coordinate: OffsetCoordinate) -> OffsetCoordinate;

    /// Converts an offset coordinate to the grid's coordinate type.
    /// If the offset coordinate is not valid, returns an error.
    fn from_offset_coordinate(
        &self,
        offset_coordinate: OffsetCoordinate,
    ) -> Result<Self::CoordinateType, String>;

    /// Computes the distance from `self` to `rhs` in the grid.
    fn distance_to(self, rhs: Self) -> i32;
}

/// Represents the size of a grid or map with a specified width and height.
#[derive(Clone, Copy)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

bitflags! {
    /// Bitflags representing how a grid/map wraps at its borders.
    ///
    /// - `WrapX`: Enable horizontal wrapping (left/right edges connect)
    /// - `WrapY`: Enable vertical wrapping (top/bottom edges connect)
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct WrapFlags: u8 {
        const WrapX = 0b0000_0001;
        const WrapY = 0b0000_0010;
    }
}
