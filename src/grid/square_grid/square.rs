use std::ops::{Add, Sub};

use glam::IVec2;

use crate::grid::{Direction, OffsetCoordinate};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Square(IVec2);

impl Square {
    /// Square neighbor coordinates array, following [`Self::EDGE`] order.
    pub const SQUARE_DIRECTIONS: [Self; 4] = [
        Self::new(1, 0),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(0, 1),
    ];

    /// Square edge directions, the directions of the edges of a `Square` relative to its center.
    ///
    /// - The number in the Square-A is the index of the direction of the Square-A corner in the array of all the corner direction
    /// - The number outside the Square-A is the index of the direction of the Square-A edge in the array of all the edge direction
    ///
    /// ```txt
    ///  ____________ ____________ ____________
    /// |            |            |            |
    /// |            |            |            |
    /// |            |     3      |            |
    /// |            |            |            |
    /// |____________|____________|____________|
    /// |            |3          0|            |
    /// |            |            |            |
    /// |     2      |  Square-A  |     0      |
    /// |            |            |            |
    /// |____________|2__________1|____________|
    /// |            |            |            |
    /// |            |            |            |
    /// |            |     1      |            |
    /// |            |            |            |
    /// |____________|____________|____________|
    /// ```
    ///
    const EDGE: [Direction; 4] = [
        Direction::East,
        Direction::South,
        Direction::West,
        Direction::North,
    ];

    /// Square corner directions, the directions of the corners of a `Square` relative to its center.
    /// > See [`EDGE`] for more information
    const CORNER: [Direction; 4] = [
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthWest,
    ];

    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub const fn x(&self) -> i32 {
        self.0.x
    }

    pub const fn y(&self) -> i32 {
        self.0.y
    }

    pub fn to_offset_coordinate(self) -> OffsetCoordinate {
        OffsetCoordinate::new(self.x(), self.y())
    }

    /// Get the square at the given `direction` from `self`.
    pub fn neighbor(self, direction: Direction) -> Self {
        let edge_index = Self::EDGE.iter().position(|&x| x == direction).unwrap();
        Self(self.0 + Self::SQUARE_DIRECTIONS[edge_index].0)
    }
}

impl Add for Square {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Square {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<[i32; 2]> for Square {
    #[inline]
    fn from(a: [i32; 2]) -> Self {
        Self(a.into())
    }
}
