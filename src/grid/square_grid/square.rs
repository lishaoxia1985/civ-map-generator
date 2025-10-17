use std::ops::{Add, Sub};

use glam::{IVec2, Vec2};

use crate::grid::{direction::Direction, offset_coordinate::OffsetCoordinate};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Square(IVec2);

impl Square {
    /// Square neighbor coordinates array, following [`SquareOrientation::ORTHOGONAL_EDGE`] order.
    pub const SQUARE_DIRECTIONS: [Self; 4] = [
        Self::new(1, 0),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(0, 1),
    ];

    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    /// Create a new [`Square`] from an [`OffsetCoordinate`].
    pub const fn from_offset(offset_coordinate: OffsetCoordinate) -> Self {
        Self(offset_coordinate.into_inner())
    }

    pub const fn x(&self) -> i32 {
        self.0.x
    }

    pub const fn y(&self) -> i32 {
        self.0.y
    }

    pub const fn into_inner(self) -> IVec2 {
        self.0
    }

    /// Create a new [`Square`] from an [`OffsetCoordinate`].
    pub fn to_offset(self) -> OffsetCoordinate {
        OffsetCoordinate::new(self.x(), self.y())
    }

    /// Get [`Square`] at the given `direction` from `self`.
    pub fn neighbor(self, orientation: SquareOrientation, direction: Direction) -> Self {
        let edge_index = orientation.edge_index(direction);
        self + Self::SQUARE_DIRECTIONS[edge_index]
    }

    #[inline]
    /// Computes coordinates length as a signed integer.
    /// The length of a [`Square`] coordinate is equal to its distance from the origin.
    pub const fn length(self) -> i32 {
        self.0.x.abs() + self.0.y.abs()
    }

    #[inline]
    /// Computes the distance from `self` to `rhs` in square coordinates as a signed integer.
    pub fn distance_to(self, rhs: Self) -> i32 {
        (self - rhs).length()
    }

    /// Return a [`Vec<Square>`] containing all [`Square`] which are exactly at a given `distance` from `self`.
    /// If `distance` = 0 the [`Vec<Square>`] will be empty. \
    /// The number of returned squares is equal to `4 * distance`.
    pub fn squares_at_distance(self, distance: u32) -> Vec<Self> {
        // If distance is 0, return an empty vector
        if distance == 0 {
            return Vec::new();
        }

        let mut square_list = Vec::with_capacity((4 * distance) as usize);
        let radius = distance as i32;

        /* for x in -radius..=radius {
            for y in -radius..=radius {
                let offset_square = Square::from([x, y]);
                if offset_square.distance_to(Square::from([0, 0])) == radius {
                    square_list.push(self + Self::new(x, y));
                }
            }
        } */

        // The following code is equivalent to the commented code above, but it is faster.
        for x in -radius..=radius {
            let y1 = radius - x.abs();
            let y2 = -y1;
            square_list.push(self + Self::new(x, y1));
            if y1 != y2 {
                square_list.push(self + Self::new(x, y2));
            }
        }

        square_list
    }

    /// Return a [`Vec<Square>`] containing all [`Square`] around `self` in a given `distance`, including `self`. \
    /// The number of returned squares is equal to `2 * distance * (distance + 1) + 1`.
    pub fn squares_in_distance(self, distance: u32) -> Vec<Self> {
        let mut square_list = Vec::with_capacity((2 * distance * (distance + 1) + 1) as usize);
        let radius = distance as i32;

        /* for x in -radius..=radius {
            for y in -radius..=radius {
                let offset_square = Square::new(x, y);
                if offset_square.distance_to(Square::new(0, 0)) <= radius {
                    square_list.push(self + offset_square);
                }
            }
        } */

        // The following code is equivalent to the commented code above, but it is faster.
        for x in -radius..=radius {
            let y_max = radius - x.abs();
            for y in -y_max..=y_max {
                square_list.push(self + Self::new(x, y));
            }
        }

        square_list
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

#[derive(Clone, Copy, Debug)]
pub struct SquareLayout {
    /// The orientation of the square layout, it can be only `orthogonal` currently.
    pub orientation: SquareOrientation,
    /// The size of the square layout. Its first element is the width and the second is the height.
    pub size: [f32; 2],
    /// The origin of the square layout. Its first element is the x-coordinate and the second is the y-coordinate.
    pub origin: [f32; 2],
}

impl SquareLayout {
    pub fn new(orientation: SquareOrientation, size: [f32; 2], origin: [f32; 2]) -> Self {
        Self {
            orientation,
            size,
            origin,
        }
    }

    /// Returns the pixel coordinates of the center of the given square coordinates.
    pub fn square_to_pixel(self, square: Square) -> Vec2 {
        match self.orientation {
            SquareOrientation::Orthogonal => {
                Vec2::from(self.origin) + square.0.as_vec2() * Vec2::from(self.size)
            }
        }
    }

    /// Returns the square coordinates that contains the given pixel position.
    pub fn pixel_to_square(self, pixel: [f32; 2]) -> Square {
        let pt: Vec2 = (Vec2::from(pixel) - Vec2::from(self.origin)) / Vec2::from(self.size);
        match self.orientation {
            SquareOrientation::Orthogonal => Square((pt + Vec2::new(0.5, 0.5)).floor().as_ivec2()),
        }
    }

    /// Returns the corner pixel coordinates of the given square coordinates according to corner direction.
    pub fn corner(self, square: Square, direction: Direction) -> [f32; 2] {
        let center = self.square_to_pixel(square);
        (center + self.corner_offset(direction)).to_array()
    }

    /// Retrieves all 4 corner pixel coordinates of the given square coordinates.
    ///
    /// The returned array is ordered and usually used to draw a square.
    pub fn all_corners(self, square: Square) -> [[f32; 2]; 4] {
        self.orientation
            .corner_direction()
            .map(|direction| self.corner(square, direction))
    }

    #[inline(always)]
    fn corner_offset(self, direction: Direction) -> Vec2 {
        let offset_value = match self.orientation {
            SquareOrientation::Orthogonal => match direction {
                Direction::NorthEast => Vec2::new(1., 1.),
                Direction::SouthEast => Vec2::new(1., -1.),
                Direction::SouthWest => Vec2::new(-1., -1.),
                Direction::NorthWest => Vec2::new(-1., 1.),
                _ => panic!("Invalid direction"),
            },
        };
        offset_value * Vec2::from(self.size) / 2.
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum SquareOrientation {
    /// 🔳
    Orthogonal,
}

impl SquareOrientation {
    /// Orthogonal Square edge directions, the directions of the edges of a `Square` relative to its center.
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
    pub const ORTHOGONAL_EDGE: [Direction; 4] = [
        Direction::East,
        Direction::South,
        Direction::West,
        Direction::North,
    ];

    /// Orthogonal Square corner directions, the directions of the corners of a `Square` relative to its center.
    /// > See [`Self::ORTHOGONAL_EDGE`] for more information
    pub const ORTHOGONAL_CORNER: [Direction; 4] = [
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthWest,
    ];

    #[inline]
    /// Get the index of the direction of the [`Square`] corner in the array of all the corner direction
    /// # Panics
    /// Panics if the direction is not a valid corner direction for the square orientation
    pub const fn corner_index(self, direction: Direction) -> usize {
        match (self, direction) {
            (SquareOrientation::Orthogonal, Direction::NorthEast) => 0,
            (SquareOrientation::Orthogonal, Direction::SouthEast) => 1,
            (SquareOrientation::Orthogonal, Direction::SouthWest) => 2,
            (SquareOrientation::Orthogonal, Direction::NorthWest) => 3,
            _ => panic!("The direction is not a valid corner direction for the square orientation"),
        }
    }

    #[inline]
    /// Get the index of the direction of the `Square` edge in the array of all the edge direction
    /// # Panics
    /// Panics if the direction is not a valid edge direction for the square orientation
    pub const fn edge_index(self, direction: Direction) -> usize {
        match (self, direction) {
            (SquareOrientation::Orthogonal, Direction::East) => 0,
            (SquareOrientation::Orthogonal, Direction::South) => 1,
            (SquareOrientation::Orthogonal, Direction::West) => 2,
            (SquareOrientation::Orthogonal, Direction::North) => 3,
            _ => panic!("The direction is not a valid edge direction for the square orientation"),
        }
    }

    /// Returns the next corner direction in clockwise order
    pub const fn corner_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 1) % 4]
    }

    /// Returns the next edge direction in clockwise order
    pub const fn edge_clockwise(self, edge_direction: Direction) -> Direction {
        let edge_index = self.edge_index(edge_direction);
        self.edge_direction()[(edge_index + 1) % 4]
    }

    /// Returns the next corner direction in counter clockwise order
    pub const fn corner_counter_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 3) % 4]
    }

    /// Returns the next edge direction in counter clockwise order
    pub const fn edge_counter_clockwise(self, edge_direction: Direction) -> Direction {
        let edge_index = self.edge_index(edge_direction);
        self.edge_direction()[(edge_index + 3) % 4]
    }

    #[inline]
    /// Get all the directions of the edges of a `Square` relative to its center
    pub const fn edge_direction(&self) -> [Direction; 4] {
        match self {
            SquareOrientation::Orthogonal => Self::ORTHOGONAL_EDGE,
        }
    }

    #[inline]
    /// Get all the directions of the corners of a `Square` relative to its center
    pub const fn corner_direction(&self) -> [Direction; 4] {
        match self {
            SquareOrientation::Orthogonal => Self::ORTHOGONAL_CORNER,
        }
    }
}
