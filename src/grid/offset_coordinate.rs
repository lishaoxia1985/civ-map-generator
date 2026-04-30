//! Offset coordinate system for grid positioning.
//!
//! [`OffsetCoordinate`] describes the column and row of a tile in a grid.
//! It is used to tackle with the situation where the grid is wrapped.
//!
//! That picture below shows a unwrapped grid with offset coordinates.
//!
//! ```txt
//! Y ↑
//!   |
//!   |  (0,height-1)        (width-1,height-1)
//!   |  +-------------------+
//!   |  |                   |
//!   |  |    Grid Area      |
//!   |  |                   |
//!   |  +-------------------+
//!   |  (0,0)               (width-1,0)
//!   +--------------------------------→ X
//!   Origin (bottom-left corner)
//! ```
//!
//! The coordinate ranges depend on whether the grid wraps at boundaries:
//!
//! - **Non-wrapped grid**: `x ∈ [0, width)`, `y ∈ [0, height)`
//! - **Wrapped grid**:
//!   - Only Wrap x: x can be any value, y ∈ [0, height)
//!     - Example (x-wrapped): `(0, 0) ≡ (width, 0) ≡ (-width, 0) ≡ (2*width, 0)` is the same cell/tile
//!   - Only Wrap y: x ∈ [0, width), y can be any value
//!     - Example (y-wrapped): `(0, 0) ≡ (0, height) ≡ (0, -height) ≡ (0, 2*height)` is the same cell/tile
//!   - Wrap both x and y: x and y can be any value
//!     - Example (both x and y wrapped): `(0, 0) ≡ (width, height) ≡ (-width, -height) ≡ (2*width, 2*height)` is the same cell/tile
//!
//! In wrapped grids multiple offset coordinates can represent the same cell,
//! when we normalize an offset coordinate, i.e. wrap its x and y coordinates to the range `([0, width), [0, height))`,
//! it can be transformed into [`Cell`] uniquely. See the documentation of [`Grid::normalize_offset`](crate::grid::Grid::normalize_offset) for details on normalization.
//!

use glam::IVec2;

/// A coordinate in the offset coordinate system.
///
/// See the [module-level documentation](self) for details on coordinate ranges,
/// normalization, and relationships to other coordinate systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
