#![allow(dead_code)]

use core::f32::consts::{FRAC_PI_3, FRAC_PI_6};
use std::{
    array,
    cmp::{max, min},
    ops::{Add, Sub},
};

use glam::{IVec2, Mat2, Vec2};

use crate::grid::{direction::Direction, offset_coordinate::OffsetCoordinate};

pub const SQRT_3: f32 = 1.732_050_8_f32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hex(IVec2);
impl Hex {
    /// Hexagon neighbor coordinates array, following [`HexOrientation::POINTY_EDGE`] or [`HexOrientation::FLAT_EDGE`] order
    pub const HEX_DIRECTIONS: [Self; 6] = [
        Self::new(1, 0),
        Self::new(1, -1),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(-1, 1),
        Self::new(0, 1),
    ];

    const HEX_DIAGONALS: [Self; 6] = [
        Self::new(2, -1),
        Self::new(1, -2),
        Self::new(-1, -1),
        Self::new(-2, 1),
        Self::new(-1, 2),
        Self::new(1, 1),
    ];

    pub const fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    /// Creates a new [`Hex`] from an [`OffsetCoordinate`].
    pub const fn from_offset(
        offset_coordinate: OffsetCoordinate,
        orientation: HexOrientation,
        offset: Offset,
    ) -> Self {
        let [x, y] = offset_coordinate.to_array();

        let (q, r) = match orientation {
            HexOrientation::Pointy => (x - (y + offset as i32 * (y & 1)) / 2, y),
            HexOrientation::Flat => (x, y - (x + offset as i32 * (x & 1)) / 2),
        };
        Hex::new(q, r)
    }

    pub const fn x(self) -> i32 {
        self.0.x
    }

    pub const fn y(self) -> i32 {
        self.0.y
    }

    pub const fn z(self) -> i32 {
        -self.0.x - self.0.y
    }

    pub const fn into_inner(self) -> IVec2 {
        self.0
    }

    pub fn to_offset(self, orientation: HexOrientation, offset: Offset) -> OffsetCoordinate {
        let (col, row) = match orientation {
            HexOrientation::Pointy => (
                self.0.x + (self.0.y + offset as i32 * (self.0.y & 1)) / 2,
                self.0.y,
            ),
            HexOrientation::Flat => (
                self.0.x,
                self.0.y + (self.0.x + offset as i32 * (self.0.x & 1)) / 2,
            ),
        };
        OffsetCoordinate::new(col, row)
    }

    pub fn to_doubled_coordinate(self, orientation: HexOrientation) -> DoubledCoordinate {
        let (col, row) = match orientation {
            HexOrientation::Pointy => (2 * self.0.x + self.0.y, self.0.y),
            HexOrientation::Flat => (self.0.x, 2 * self.0.y + self.0.x),
        };
        DoubledCoordinate::new(col, row)
    }

    /// Get the hex at the given `direction` from `self`, according to the given `orientation` is `HexOrientation::Pointy` or `HexOrientation::Flat`.
    pub fn neighbor(self, orientation: HexOrientation, direction: Direction) -> Hex {
        let edge_index = orientation.edge_index(direction);
        self + Self::HEX_DIRECTIONS[edge_index]
    }

    pub fn hex_diagonal_neighbor(self, direction: i32) -> Hex {
        self + Self::HEX_DIAGONALS[direction as usize]
    }

    #[inline]
    /// Computes coordinates length as a signed integer.
    /// The length of a [`Hex`] coordinate is equal to its distance from the origin.
    pub const fn length(self) -> i32 {
        /* let [x, y, z] = [self.x.abs(), self.y.abs(), self.z().abs()];
        if x >= y && x >= z {
            x
        } else if y >= x && y >= z {
            y
        } else {
            z
        } */

        // The following code is equivalent to the commented code above, but it is faster.
        (self.0.x.abs() + self.0.y.abs() + self.z().abs()) / 2
    }

    #[inline]
    /// Computes the distance from `self` to `rhs` in hexagonal space as a signed integer.
    pub fn distance_to(self, rhs: Self) -> i32 {
        (self - rhs).length()
    }

    /// Return a [`Vec<Hex>`] containing all [`Hex`] which are exactly at a given `distance` from `self`.
    /// If `distance` = 0 the [`Vec<Hex>`] will be empty. \
    /// The number of returned hexes is equal to `6 * distance`.
    pub fn hexes_at_distance(self, distance: u32) -> Vec<Hex> {
        // If distance is 0, return an empty vector
        if distance == 0 {
            return Vec::new();
        }

        let mut hex_list = Vec::with_capacity((6 * distance) as usize);
        let radius = distance as i32;

        /* for q in -radius..=radius {
            for r in max(-radius, -q - radius)..=min(radius, -q + radius) {
                let offset_hex = Hex::from([q, r]);
                if offset_hex.distance_to(Hex::from([0, 0])) == radius {
                    let hex = self + offset_hex;
                    hex_list.push(hex);
                }
            }
        } */

        // The following code is equivalent to the commented code above, but it is faster.
        let mut hex = Hex(self.0 + Self::HEX_DIRECTIONS[4].0 * radius);
        for hex_direction in Self::HEX_DIRECTIONS {
            for _ in 0..radius {
                hex_list.push(hex);
                hex = hex + hex_direction;
            }
        }

        hex_list
    }

    /// Return a [`Vec<Hex>`] containing all [`Hex`] around `self` in a given `distance`, including `self`. \
    /// The number of returned hexes is equal to `3 * distance * (distance + 1) + 1`.
    pub fn hexes_in_distance(self, distance: u32) -> Vec<Hex> {
        let mut hex_list = Vec::with_capacity((3 * distance * (distance + 1) + 1) as usize);
        let radius = distance as i32;
        for q in -radius..=radius {
            for r in max(-radius, -q - radius)..=min(radius, -q + radius) {
                let hex = self + Hex::new(q, r);
                hex_list.push(hex);
            }
        }
        hex_list
    }

    pub fn hex_rotate_left(self) -> Self {
        Self(-IVec2::new(self.z(), self.0.x))
    }

    pub fn hex_rotate_right(self) -> Self {
        Self(-IVec2::new(self.0.y, self.z()))
    }

    /// Rounds floating point coordinates to [`Hex`].
    pub fn round(fractional_hex: Vec2) -> Self {
        let mut rounded = fractional_hex.round();

        let diff = fractional_hex - rounded;

        if diff.x.abs() >= diff.y.abs() {
            rounded.x += 0.5_f32.mul_add(diff.y, diff.x).round();
        } else {
            rounded.y += 0.5_f32.mul_add(diff.x, diff.y).round();
        }

        Self(rounded.as_ivec2())
    }
}

impl Add for Hex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Hex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<[i32; 2]> for Hex {
    #[inline]
    fn from(a: [i32; 2]) -> Self {
        Self(a.into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DoubledCoordinate(IVec2);
impl DoubledCoordinate {
    pub fn new(x: i32, y: i32) -> Self {
        Self(IVec2::new(x, y))
    }

    pub fn to_hex(self, orientation: HexOrientation) -> Hex {
        let (q, r) = match orientation {
            HexOrientation::Pointy => ((self.0.x - self.0.y) / 2, self.0.y),
            HexOrientation::Flat => (self.0.x, (self.0.y - self.0.x) / 2),
        };
        Hex::new(q, r)
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct HexLayout {
    pub orientation: HexOrientation,
    pub size: Vec2,
    pub origin: Vec2,
}
impl HexLayout {
    pub fn new(orientation: HexOrientation, size: Vec2, origin: Vec2) -> Self {
        Self {
            orientation,
            size,
            origin,
        }
    }

    pub fn hex_to_pixel(self, hex: Hex) -> Vec2 {
        let m = self.orientation.conversion_matrix();
        let size: Vec2 = self.size;
        let origin: Vec2 = self.origin;
        let mat2 = m.forward_matrix;
        let pixel_position = mat2 * (hex.0.as_vec2()) * size;
        pixel_position + origin
    }

    pub fn pixel_to_hex(self, pixel_position: Vec2) -> Hex {
        let m = self.orientation.conversion_matrix();
        let (size, origin) = (self.size, self.origin);
        let pt = (pixel_position - origin) / size;
        let mat2 = m.inverse_matrix;
        let fractional_hex = mat2 * pt;
        Hex::round(fractional_hex)
    }

    /// Get the corner pixel coordinates of the given hexagonal coordinates according to corner direction
    pub fn corner(self, hex: Hex, direction: Direction) -> Vec2 {
        let center: Vec2 = self.hex_to_pixel(hex);
        let offset: Vec2 = self.corner_offset(direction);
        center + offset
    }

    /// Retrieves all 6 corner pixel coordinates of the given hexagonal coordinates
    pub fn all_corners(self, hex: Hex) -> [Vec2; 6] {
        let corner_array = self.orientation.corner_direction();
        array::from_fn(|i| self.corner(hex, corner_array[i]))
    }

    fn corner_offset(self, direction: Direction) -> Vec2 {
        let size: Vec2 = self.size;
        let angle: f32 = self.orientation.corner_angle(direction);
        size * Vec2::from_angle(angle)
    }
}

pub fn hex_linedraw(a: Hex, b: Hex) -> Vec<Hex> {
    let n: i32 = a.distance_to(b);
    let a_nudge = a.0.as_vec2() + Vec2::new(1e-06, 1e-06);
    let b_nudge = b.0.as_vec2() + Vec2::new(1e-06, 1e-06);
    let step: f32 = 1.0 / max(n, 1) as f32;
    (0..=n)
        .map(|i| Hex::round(a_nudge.lerp(b_nudge, step * i as f32)))
        .collect()
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Offset {
    Even = 1,
    Odd = -1,
}

#[derive(Clone, Copy, Debug)]
/// This struct stored a forward and inverse matrix, for pixel/hex conversion
pub struct ConversionMatrix {
    /// Matrix used to compute hexagonal coordinates to pixel coordinates
    pub forward_matrix: Mat2,
    /// Matrix used to compute pixel coordinates to hexagonal coordinates
    pub inverse_matrix: Mat2,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum HexOrientation {
    /// ⬢, [`Hex`] is pointy-topped
    Pointy,
    /// ⬣, [`Hex`] is flat-topped
    Flat,
}

impl HexOrientation {
    /// Pointy hex edge direction, the directions of the edges of a `Hex` relative to its center
    ///
    /// - The number in the Hex-A is the index of the direction of the Hex-A corner in the array of all the corner direction
    /// - The number outside the Hex-A is the index of the direction of the Hex-A edge in the array of all the edge direction
    ///
    /// ```txt
    ///
    ///          / \     / \
    ///         /   \   /   \
    ///        /     \ /     \
    ///       |       |       |
    ///       |   4   |   5   |
    ///       |       |       |
    ///      / \     /5\     / \
    ///     /   \   /   \   /   \
    ///    /     \ /     \ /     \
    ///   |       |4     0|       |
    ///   |   3   | Hex-A |   0   |
    ///   |       |3     1|       |
    ///    \     / \     / \     /
    ///     \   /   \   /   \   /
    ///      \ /     \2/     \ /
    ///       |       |       |
    ///       |   2   |   1   |
    ///       |       |       |
    ///        \     / \     /
    ///         \   /   \   /
    ///          \ /     \ /
    ///  ```
    ///     
    pub const POINTY_EDGE: [Direction; 6] = [
        Direction::East,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
        Direction::NorthEast,
    ];

    /// Pointy hex corner direction, the directions of the corners of a `Hex` relative to its center
    /// > See [`HexOrientation::POINTY_EDGE`] for more information
    pub const POINTY_CORNER: [Direction; 6] = [
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::NorthWest,
        Direction::North,
    ];

    /// Flat hex edge direction, the directions of the edges of a `Hex` relative to its center
    ///  
    /// - The number in the Hex-A is the index of the direction of the Hex-A corner in the array of all the corner direction
    /// - The number outside the Hex-A is the index of the direction of the Hex-A edge in the array of all the edge direction
    ///
    /// ```txt
    ///                 ________
    ///                /        \
    ///               /          \
    ///      ________/     5      \________
    ///     /        \            /        \
    ///    /          \          /          \
    ///   /     4      \________/     0      \
    ///   \            /4      5\            /
    ///    \          /          \          /
    ///     \________/3   Hex-A  0\________/
    ///     /        \            /        \
    ///    /          \          /          \
    ///   /     3      \2______1/     1      \
    ///   \            /        \            /
    ///    \          /          \          /
    ///     \________/     2      \________/
    ///              \            /
    ///               \          /
    ///                \________/
    /// ```
    ///    
    pub const FLAT_EDGE: [Direction; 6] = Self::POINTY_CORNER;

    /// Flat hex corner direction, the directions of the corners of a `Hex` relative to its center
    /// > See [`HexOrientation::FLAT_EDGE`] for more information
    pub const FLAT_CORNER: [Direction; 6] = Self::POINTY_EDGE;

    #[inline]
    /// Get the index of the direction of the [`Hex`] corner in the array of all the corner direction
    /// # Panics
    /// Panics if the direction is not a valid corner direction for the hexagon orientation
    pub fn corner_index(self, direction: Direction) -> usize {
        self.corner_direction()
            .iter()
            .position(|&x| x == direction)
            .expect("The direction is not a valid corner direction for the hexagon orientation")
    }

    #[inline]
    /// Get the index of the direction of the `Hex` edge in the array of all the edge direction
    /// # Panics
    /// Panics if the direction is not a valid edge direction for the hexagon orientation
    pub fn edge_index(self, direction: Direction) -> usize {
        self.edge_direction()
            .iter()
            .position(|&x| x == direction)
            .expect("The direction is not a valid edge direction for the hexagon orientation")
    }

    /// Returns the next corner direction in clockwise order
    pub fn corner_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 1) % 6]
    }

    /// Returns the next edge direction in clockwise order
    pub fn edge_clockwise(self, edge_direction: Direction) -> Direction {
        let edge_index = self.edge_index(edge_direction);
        self.edge_direction()[(edge_index + 1) % 6]
    }

    /// Returns the next corner direction in counter clockwise order
    pub fn corner_counter_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 5) % 6]
    }

    /// Returns the next edge direction in counter clockwise order
    pub fn edge_counter_clockwise(self, edge_direction: Direction) -> Direction {
        let edge_index = self.edge_index(edge_direction);
        self.edge_direction()[(edge_index + 5) % 6]
    }

    #[inline]
    /// Returns the angle of the `Hex` corner in radians of the given direction for the hexagons
    pub fn corner_angle(self, direction: Direction) -> f32 {
        let start_angle = match self {
            HexOrientation::Pointy => FRAC_PI_6,
            HexOrientation::Flat => 0.0,
        };
        let corner_index = self.corner_index(direction) as f32;
        //equal to `start_angle - corner_index * FRAC_PI_3`
        corner_index.mul_add(-FRAC_PI_3, start_angle)
    }

    #[inline]
    /// Returns the angle of the `Hex` edge in radians of the given direction for the hexagons
    pub fn edge_angle(self, direction: Direction) -> f32 {
        let start_angle = match self {
            HexOrientation::Pointy => 0.0,
            HexOrientation::Flat => FRAC_PI_6,
        };
        let edge_index = self.edge_index(direction) as f32;
        //equal to `start_angle - edge_index * FRAC_PI_3`
        edge_index.mul_add(-FRAC_PI_3, start_angle)
    }

    const POINTY_CONVERSION_MATRIX: ConversionMatrix = ConversionMatrix {
        forward_matrix: Mat2::from_cols_array(&[SQRT_3, 0.0, SQRT_3 / 2.0, 3.0 / 2.0]),
        inverse_matrix: Mat2::from_cols_array(&[SQRT_3 / 3.0, 0.0, -1.0 / 3.0, 2.0 / 3.0]),
    };

    const FLAT_CONVERSION_MATRIX: ConversionMatrix = ConversionMatrix {
        forward_matrix: Mat2::from_cols_array(&[3.0 / 2.0, SQRT_3 / 2.0, 0.0, SQRT_3]),
        inverse_matrix: Mat2::from_cols_array(&[2.0 / 3.0, -1.0 / 3.0, 0.0, SQRT_3 / 3.0]),
    };

    #[inline]
    /// Get `ConversionMatrix` for pixel/hex conversion
    const fn conversion_matrix(self) -> ConversionMatrix {
        match self {
            Self::Pointy => Self::POINTY_CONVERSION_MATRIX,
            Self::Flat => Self::FLAT_CONVERSION_MATRIX,
        }
    }

    #[inline]
    /// Get all the directions of the edges of a `Hex` relative to its center
    pub const fn edge_direction(&self) -> [Direction; 6] {
        match self {
            HexOrientation::Pointy => Self::POINTY_EDGE,
            HexOrientation::Flat => Self::FLAT_EDGE,
        }
    }

    #[inline]
    /// Get all the directions of the corners of a `Hex` relative to its center
    pub const fn corner_direction(&self) -> [Direction; 6] {
        match self {
            HexOrientation::Pointy => Self::POINTY_CORNER,
            HexOrientation::Flat => Self::FLAT_CORNER,
        }
    }
}

// Tests
#[cfg(test)]
mod tests {

    use glam::{IVec2, Vec2};

    use super::{
        hex_linedraw, Direction, DoubledCoordinate, Hex, HexLayout, HexOrientation, Offset,
        OffsetCoordinate,
    };

    pub fn equal_hex(name: &str, a: Hex, b: Hex) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_offset_coordinate(name: &str, a: OffsetCoordinate, b: OffsetCoordinate) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_doubled_coordinate(name: &str, a: DoubledCoordinate, b: DoubledCoordinate) {
        if a != b {
            panic!("FAIL {}", name);
        }
    }

    pub fn equal_hex_array(name: &str, a: Vec<Hex>, b: Vec<Hex>) {
        assert_eq!(a.len(), b.len(), "FAIL {}", name);
        for (x, y) in a.into_iter().zip(b.into_iter()) {
            equal_hex(name, x, y);
        }
    }

    #[test]
    pub fn test_hex_neighbor() {
        equal_hex(
            "hex_neighbor",
            Hex::new(1, -3),
            Hex::new(1, -2).neighbor(HexOrientation::Flat, Direction::South),
        );
        equal_hex(
            "hex_neighbor",
            Hex::new(1, -3),
            Hex::new(1, -2).neighbor(HexOrientation::Pointy, Direction::SouthWest),
        );
    }

    #[test]
    pub fn test_hex_diagonal() {
        equal_hex(
            "hex_diagonal",
            Hex::new(-1, -1),
            Hex::new(1, -2).hex_diagonal_neighbor(3),
        );
    }

    #[test]
    pub fn test_hex_distance() {
        assert_eq!(
            7,
            Hex::new(3, -7).distance_to(Hex(IVec2::ZERO)),
            "FAIL hex_distance"
        );
    }

    #[test]
    pub fn test_hex_rotate_right() {
        equal_hex(
            "hex_rotate_right",
            Hex::new(1, -3).hex_rotate_right(),
            Hex::new(3, -2),
        );
    }

    #[test]
    pub fn test_hex_rotate_left() {
        equal_hex(
            "hex_rotate_left",
            Hex::new(1, -3).hex_rotate_left(),
            Hex::new(-2, -1),
        );
    }

    #[test]
    pub fn test_hex_round() {
        let a = Vec2::ZERO;
        let b = Vec2::new(1.0, -1.0);
        let c = Vec2::new(0.0, -1.0);
        equal_hex(
            "hex_round 1",
            Hex::new(5, -10),
            Hex::round(Vec2::ZERO.lerp(Vec2::new(10.0, -20.0), 0.5)),
        );
        equal_hex("hex_round 2", Hex::round(a), Hex::round(a.lerp(b, 0.499)));
        equal_hex("hex_round 3", Hex::round(b), Hex::round(a.lerp(b, 0.501)));
        equal_hex(
            "hex_round 4",
            Hex::round(a),
            Hex::round(a * 0.4 + b * 0.3 + c * 0.3),
        );
        equal_hex(
            "hex_round 5",
            Hex::round(c),
            Hex::round(a * 0.3 + b * 0.3 + c * 0.4),
        );
    }

    #[test]
    pub fn test_hex_linedraw() {
        equal_hex_array(
            "hex_linedraw",
            vec![
                Hex(IVec2::ZERO),
                Hex::new(0, -1),
                Hex::new(0, -2),
                Hex::new(1, -3),
                Hex::new(1, -4),
                Hex::new(1, -5),
            ],
            hex_linedraw(Hex(IVec2::ZERO), Hex::new(1, -5)),
        );
    }

    #[test]
    pub fn test_layout() {
        let h = Hex::new(3, 4);
        let flat: HexLayout = HexLayout {
            orientation: HexOrientation::Flat,
            size: Vec2 { x: 10.0, y: 15.0 },
            origin: Vec2 { x: 35.0, y: 71.0 },
        };
        equal_hex("layout", h, flat.pixel_to_hex(flat.hex_to_pixel(h)));
        let pointy: HexLayout = HexLayout {
            orientation: HexOrientation::Pointy,
            size: Vec2 { x: 10.0, y: 15.0 },
            origin: Vec2 { x: 35.0, y: 71.0 },
        };
        equal_hex("layout", h, pointy.pixel_to_hex(pointy.hex_to_pixel(h)));
    }

    #[test]
    pub fn test_offset_roundtrip() {
        let a = Hex::new(3, 4);
        let b = OffsetCoordinate::new(1, -3);
        equal_hex(
            "conversion_roundtrip even-q",
            a,
            Hex::from_offset(
                a.to_offset(HexOrientation::Flat, Offset::Even),
                HexOrientation::Flat,
                Offset::Even,
            ),
        );
        equal_offset_coordinate(
            "conversion_roundtrip even-q",
            b,
            Hex::from_offset(b, HexOrientation::Flat, Offset::Even)
                .to_offset(HexOrientation::Flat, Offset::Even),
        );
        equal_hex(
            "conversion_roundtrip odd-q",
            a,
            Hex::from_offset(
                a.to_offset(HexOrientation::Flat, Offset::Odd),
                HexOrientation::Flat,
                Offset::Odd,
            ),
        );
        equal_offset_coordinate(
            "conversion_roundtrip odd-q",
            b,
            Hex::from_offset(b, HexOrientation::Flat, Offset::Odd)
                .to_offset(HexOrientation::Flat, Offset::Odd),
        );
        equal_hex(
            "conversion_roundtrip even-r",
            a,
            Hex::from_offset(
                a.to_offset(HexOrientation::Pointy, Offset::Even),
                HexOrientation::Pointy,
                Offset::Even,
            ),
        );
        equal_offset_coordinate(
            "conversion_roundtrip even-r",
            b,
            Hex::from_offset(b, HexOrientation::Pointy, Offset::Even)
                .to_offset(HexOrientation::Pointy, Offset::Even),
        );
        equal_hex(
            "conversion_roundtrip odd-r",
            a,
            Hex::from_offset(
                a.to_offset(HexOrientation::Pointy, Offset::Odd),
                HexOrientation::Pointy,
                Offset::Odd,
            ),
        );
        equal_offset_coordinate(
            "conversion_roundtrip odd-r",
            b,
            Hex::from_offset(b, HexOrientation::Pointy, Offset::Odd)
                .to_offset(HexOrientation::Pointy, Offset::Odd),
        );
    }

    #[test]
    pub fn test_offset_from_hex() {
        equal_offset_coordinate(
            "offset_from_hex even-q",
            OffsetCoordinate::new(1, 3),
            Hex::new(1, 2).to_offset(HexOrientation::Flat, Offset::Even),
        );
        equal_offset_coordinate(
            "offset_from_hex odd-q",
            OffsetCoordinate::new(1, 2),
            Hex::new(1, 2).to_offset(HexOrientation::Flat, Offset::Odd),
        );
    }

    #[test]
    pub fn test_offset_to_hex() {
        equal_hex(
            "offset_to_hex even-q",
            Hex::new(1, 2),
            Hex::from_offset(
                OffsetCoordinate::new(1, 3),
                HexOrientation::Flat,
                Offset::Even,
            ),
        );
        equal_hex(
            "offset_to_hex odd-q",
            Hex::new(1, 2),
            Hex::from_offset(
                OffsetCoordinate::new(1, 2),
                HexOrientation::Flat,
                Offset::Odd,
            ),
        );
    }

    #[test]
    pub fn test_doubled_roundtrip() {
        let a = Hex::new(3, 4);
        let b = DoubledCoordinate::new(1, -3);
        equal_hex(
            "conversion_roundtrip doubled-q",
            a,
            a.to_doubled_coordinate(HexOrientation::Flat)
                .to_hex(HexOrientation::Flat),
        );
        equal_doubled_coordinate(
            "conversion_roundtrip doubled-q",
            b,
            b.to_hex(HexOrientation::Flat)
                .to_doubled_coordinate(HexOrientation::Flat),
        );
        equal_hex(
            "conversion_roundtrip doubled-r",
            a,
            a.to_doubled_coordinate(HexOrientation::Pointy)
                .to_hex(HexOrientation::Pointy),
        );
        equal_doubled_coordinate(
            "conversion_roundtrip doubled-r",
            b,
            b.to_hex(HexOrientation::Pointy)
                .to_doubled_coordinate(HexOrientation::Pointy),
        );
    }

    #[test]
    pub fn test_doubled_from_hex() {
        equal_doubled_coordinate(
            "doubled_from_hex doubled-q",
            DoubledCoordinate::new(1, 5),
            Hex::new(1, 2).to_doubled_coordinate(HexOrientation::Flat),
        );
        equal_doubled_coordinate(
            "doubled_from_hex doubled-r",
            DoubledCoordinate::new(4, 2),
            Hex::new(1, 2).to_doubled_coordinate(HexOrientation::Pointy),
        );
    }

    #[test]
    pub fn test_doubled_to_hex() {
        equal_hex(
            "doubled_to_hex doubled-q",
            Hex::new(1, 2),
            DoubledCoordinate::new(1, 5).to_hex(HexOrientation::Flat),
        );
        equal_hex(
            "doubled_to_hex doubled-r",
            Hex::new(1, 2),
            DoubledCoordinate::new(4, 2).to_hex(HexOrientation::Pointy),
        );
    }
}
