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

    /// Converts a coordinate of the grid to an offset coordinate.
    fn to_offset_coordinate(&self, offset_coordinate: OffsetCoordinate) -> OffsetCoordinate;

    /// Converts an offset coordinate to the grid's coordinate type.
    fn from_offset_coordinate(&self, offset_coordinate: OffsetCoordinate) -> Self::CoordinateType;
}
