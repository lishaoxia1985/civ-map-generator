//! Direction enum for hexagonal and square grids.
//!
//! Represents the 8 cardinal and intercardinal directions used in grid navigation.
//!
//! # Direction Layout
//!
//! ```txt
//!         North
//!           ↑
//!     NW ←  |  → NE
//!           |
//! West ←----+----→ East
//!           |
//!     SW ←  |  → SE
//!           ↓
//!         South
//! ```
//!

#[repr(u8)]
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    /// Returns the opposite direction of the current direction.
    pub const fn opposite(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::NorthEast => Direction::SouthWest,
            Direction::East => Direction::West,
            Direction::SouthEast => Direction::NorthWest,
            Direction::South => Direction::North,
            Direction::SouthWest => Direction::NorthEast,
            Direction::West => Direction::East,
            Direction::NorthWest => Direction::SouthEast,
        }
    }
}
