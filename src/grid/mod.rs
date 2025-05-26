use direction::Direction;
use offset_coordinate::OffsetCoordinate;

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
