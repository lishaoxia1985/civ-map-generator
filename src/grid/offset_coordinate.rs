use core::convert::From;

use glam::IVec2;

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

    pub const fn into_inner(self) -> IVec2 {
        self.0
    }

    pub const fn to_array(self) -> [i32; 2] {
        [self.0.x, self.0.y]
    }
}

impl From<[u32; 2]> for OffsetCoordinate {
    fn from(value: [u32; 2]) -> Self {
        OffsetCoordinate::new(value[0] as i32, value[1] as i32)
    }
}
