use glam::IVec2;

use super::{
    hex::{Hex, HexOrientation, Offset},
    square_grid::square::Square,
};

#[derive(Clone, Copy, PartialEq, Eq)]
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
