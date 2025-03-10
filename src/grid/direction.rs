#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    None,
}

impl Direction {
    /// Returns the opposite direction of the current direction
    ///
    /// # Panics
    ///
    /// Panics if the current direction is `Direction::None`
    pub const fn opposite_direction(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::NorthEast => Direction::SouthWest,
            Direction::East => Direction::West,
            Direction::SouthEast => Direction::NorthWest,
            Direction::South => Direction::North,
            Direction::SouthWest => Direction::NorthEast,
            Direction::West => Direction::East,
            Direction::NorthWest => Direction::SouthEast,
            Direction::None => panic!("This direction has no opposite direction."),
        }
    }
}
