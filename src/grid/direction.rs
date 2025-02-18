#[derive(PartialEq, Clone, Copy, Debug)]
/// This enum represents all the directions of hex coordinates
///
/// Each enum member has a constant number, it means:\
/// We should create 2 direction arrays, one for the hex edge and one for the hex corner (We call them A and B).
/// - From left to right, the first digit of the number represents the index of the direction in the array A.\
///   If the digit is 9, the direction does not exist in the array A.
/// - From left to right, the second digit of the number represents the index of the direction in the array B.\
///   If the digit is 9, the direction does not exist in the array B.
pub enum Direction {
    North = 95,
    NorthEast = 50,
    East = 09,
    SouthEast = 11,
    South = 92,
    SouthWest = 23,
    West = 39,
    NorthWest = 44,
    None = 99,
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
