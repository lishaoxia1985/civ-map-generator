use glam::IVec2;

use super::{
    hex_grid::hex::{Hex, HexOrientation, Offset},
    square_grid::square::Square,
};

/// A coordinate in the offset coordinate system.
///
/// Offset coordinates represent positions relative to a reference point (typically the origin at (0, 0)
/// in a 2D coordinate system, with the grid's left-bottom corner as origin). These coordinates indicate
/// displacement rather than absolute positions.
///
/// # Coordinate Ranges
///
/// `width` and `height` are the dimensions of the grid, and they define the valid ranges for the x and y coordinates:
///
/// - **Non-wrapped grid**:  
///   `x ∈ [0, width)`, `y ∈ [0, height)`
///
/// - **Wrapped grid**:
///   - When x-wrapped: `x` can be any integer (multiple representations exist)
///   - When y-wrapped: `y` can be any integer (multiple representations exist)
///
/// # Multiple Representations
///
/// In wrapped grids, a single coordinate may have multiple representations. For example:
///
/// - If x-wrapped: (0, 0) ≡ (width, 0) ≡ (-width, 0) ≡ (2*width, 0), etc.
///
/// By convention, we typically store coordinates normalized to `x ∈ [0, width)` and `y ∈ [0, height)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetCoordinate(pub IVec2);

impl OffsetCoordinate {
    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub fn to_hex(self, offset: Offset, orientation: HexOrientation) -> Hex {
        let (q, r) = match orientation {
            HexOrientation::Pointy => (
                self.0.x - (self.0.y + offset.value() * (self.0.y & 1)) / 2,
                self.0.y,
            ),
            HexOrientation::Flat => (
                self.0.x,
                self.0.y - (self.0.x + offset.value() * (self.0.x & 1)) / 2,
            ),
        };
        Hex::new(q, r)
    }

    pub const fn to_square(self) -> Square {
        Square::new(self.0.x, self.0.y)
    }

    pub const fn to_array(self) -> [i32; 2] {
        [self.0.x, self.0.y]
    }
}
