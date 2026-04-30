//! Hexagonal grid coordinate system implementation.
//!

#![allow(dead_code)]

use core::f32::consts::{FRAC_PI_3, FRAC_PI_6};
use std::{
    cmp::{max, min},
    ops::{Add, Sub},
};

use glam::{IVec2, Mat2, Vec2};

use crate::grid::{direction::Direction, offset_coordinate::OffsetCoordinate};

pub const SQRT_3: f32 = 1.732_050_8_f32;

/// Hexagonal grid coordinate in axial (cube) coordinate system.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hex(IVec2);
impl Hex {
    /// Hexagon neighbor coordinates array, following [`HexOrientation::POINTY_EDGE`] or [`HexOrientation::FLAT_EDGE`] order.
    ///
    /// These 6 direction vectors represent the offset from a hex to each of its neighbors.
    /// The order corresponds to clockwise directions starting from East (for pointy-top)
    /// or NorthEast (for flat-top).
    ///
    /// # Direction Mapping
    ///
    /// | Index | Pointy-Top | Flat-Top | Vector (Hex)  |
    /// | :---: | :--------- | :------- | :------------ |
    /// | 0     | East       | NE       | (1, 0)        |
    /// | 1     | SE         | SE       | (1, -1)       |
    /// | 2     | SW         | South    | (0, -1)       |
    /// | 3     | West       | SW       | (-1, 0)       |
    /// | 4     | NW         | NW       | (-1, 1)       |
    /// | 5     | NE         | North    | (0, 1)        |
    pub const HEX_DIRECTIONS: [Self; 6] = [
        Self::new(1, 0),
        Self::new(1, -1),
        Self::new(0, -1),
        Self::new(-1, 0),
        Self::new(-1, 1),
        Self::new(0, 1),
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

    /// Converts a hex coordinate to an offset coordinate.
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

    /// Get the hex at the given `direction` from `self`, according to the given `orientation` is `HexOrientation::Pointy` or `HexOrientation::Flat`.
    pub fn neighbor(self, orientation: HexOrientation, direction: Direction) -> Hex {
        let edge_index = orientation.edge_index(direction);
        self + Self::HEX_DIRECTIONS[edge_index]
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

    /// Rounds floating point coordinates to [`Hex`].
    #[inline(always)]
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

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct HexLayout {
    /// The orientation of the hexagonal layout (pointy or flat top).
    pub orientation: HexOrientation,
    /// The size of each hex in pixels: [width, height].
    pub size: [f32; 2],
    /// The pixel position of hex (0, 0): [x, y].
    pub origin: [f32; 2],
}

impl HexLayout {
    pub fn new(orientation: HexOrientation, size: [f32; 2], origin: [f32; 2]) -> Self {
        Self {
            orientation,
            size,
            origin,
        }
    }

    /// Returns the pixel coordinates of the center of the given hexagonal coordinates.
    pub fn hex_to_pixel(self, hex: Hex) -> Vec2 {
        let m = self.orientation.conversion_matrix();
        let size = Vec2::from(self.size);
        let origin = Vec2::from(self.origin);
        let mat2 = m.forward_matrix;
        let pixel = mat2 * (hex.0.as_vec2()) * size;
        pixel + origin
    }

    /// Returns the hexagonal coordinates that contains the given pixel position.
    pub fn pixel_to_hex(self, pixel: [f32; 2]) -> Hex {
        let m = self.orientation.conversion_matrix();
        let size = Vec2::from(self.size);
        let origin = Vec2::from(self.origin);
        let pt = (Vec2::from(pixel) - origin) / size;
        let mat2 = m.inverse_matrix;
        let fractional_hex = mat2 * pt;
        Hex::round(fractional_hex)
    }

    /// Returns the corner pixel coordinates of the given hexagonal coordinates according to corner direction.
    pub fn corner(self, hex: Hex, direction: Direction) -> [f32; 2] {
        let center: Vec2 = self.hex_to_pixel(hex);
        let offset: Vec2 = self.corner_offset(direction);
        (center + offset).to_array()
    }

    /// Retrieves all 6 corner pixel coordinates of the given hexagonal coordinates.
    ///
    /// The returned array is ordered and usually used to draw a hexagon.
    pub fn all_corners(self, hex: Hex) -> [[f32; 2]; 6] {
        self.orientation
            .corner_direction()
            .map(|direction| self.corner(hex, direction))
    }

    #[inline(always)]
    fn corner_offset(self, direction: Direction) -> Vec2 {
        let size: Vec2 = Vec2::from(self.size);
        let angle: f32 = self.orientation.corner_angle(direction);
        size * Vec2::from_angle(angle)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Offset {
    /// Even offset variant (value = +1)
    Even = 1,
    /// Odd offset variant (value = -1)
    Odd = -1,
}

/// Conversion matrices for transforming between hex and pixel coordinates.
///
/// Contains precomputed forward and inverse matrices for efficient coordinate transformations.
/// These matrices are orientation-specific (pointy vs flat) and account for hex geometry.
#[derive(Clone, Copy, Debug)]
pub struct ConversionMatrix {
    /// Matrix used to compute hexagonal coordinates to pixel coordinates.
    pub forward_matrix: Mat2,
    /// Matrix used to compute pixel coordinates to hexagonal coordinates.
    pub inverse_matrix: Mat2,
}

/// Hexagonal grid orientation (pointy-top vs flat-top).
///
/// Determines the visual orientation of hexagons and affects coordinate conversions,
/// neighbor directions, and pixel layout calculations.
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum HexOrientation {
    /// ⬢ Pointy-top orientation: hexagon has pointed top/bottom
    Pointy,
    /// ⬣ Flat-top orientation: hexagon has flat top/bottom
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
    pub const fn corner_index(self, direction: Direction) -> usize {
        match (self, direction) {
            (HexOrientation::Pointy, Direction::NorthEast) => 0,
            (HexOrientation::Pointy, Direction::SouthEast) => 1,
            (HexOrientation::Pointy, Direction::South) => 2,
            (HexOrientation::Pointy, Direction::SouthWest) => 3,
            (HexOrientation::Pointy, Direction::NorthWest) => 4,
            (HexOrientation::Pointy, Direction::North) => 5,
            (HexOrientation::Pointy, Direction::East | Direction::West) => {
                panic!("The direction is not a valid corner direction for the hexagon orientation")
            }
            (HexOrientation::Flat, Direction::East) => 0,
            (HexOrientation::Flat, Direction::SouthEast) => 1,
            (HexOrientation::Flat, Direction::SouthWest) => 2,
            (HexOrientation::Flat, Direction::West) => 3,
            (HexOrientation::Flat, Direction::NorthWest) => 4,
            (HexOrientation::Flat, Direction::NorthEast) => 5,
            (HexOrientation::Flat, Direction::North | Direction::South) => {
                panic!("The direction is not a valid corner direction for the hexagon orientation")
            }
        }
    }

    #[inline]
    /// Get the index of the direction of the `Hex` edge in the array of all the edge direction
    /// # Panics
    /// Panics if the direction is not a valid edge direction for the hexagon orientation
    pub const fn edge_index(self, direction: Direction) -> usize {
        match (self, direction) {
            (HexOrientation::Pointy, Direction::East) => 0,
            (HexOrientation::Pointy, Direction::SouthEast) => 1,
            (HexOrientation::Pointy, Direction::SouthWest) => 2,
            (HexOrientation::Pointy, Direction::West) => 3,
            (HexOrientation::Pointy, Direction::NorthWest) => 4,
            (HexOrientation::Pointy, Direction::NorthEast) => 5,
            (HexOrientation::Pointy, Direction::North | Direction::South) => {
                panic!("The direction is not a valid edge direction for the hexagon orientation")
            }
            (HexOrientation::Flat, Direction::NorthEast) => 0,
            (HexOrientation::Flat, Direction::SouthEast) => 1,
            (HexOrientation::Flat, Direction::South) => 2,
            (HexOrientation::Flat, Direction::SouthWest) => 3,
            (HexOrientation::Flat, Direction::NorthWest) => 4,
            (HexOrientation::Flat, Direction::North) => 5,
            (HexOrientation::Flat, Direction::East | Direction::West) => {
                panic!("The direction is not a valid edge direction for the hexagon orientation")
            }
        }
    }

    /// Returns the next corner direction in clockwise order
    pub const fn corner_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 1) % 6]
    }

    /// Returns the next edge direction in clockwise order
    pub const fn edge_clockwise(self, edge_direction: Direction) -> Direction {
        let edge_index = self.edge_index(edge_direction);
        self.edge_direction()[(edge_index + 1) % 6]
    }

    /// Returns the next corner direction in counter clockwise order
    pub const fn corner_counter_clockwise(self, corner_direction: Direction) -> Direction {
        let corner_index = self.corner_index(corner_direction);
        self.corner_direction()[(corner_index + 5) % 6]
    }

    /// Returns the next edge direction in counter clockwise order
    pub const fn edge_counter_clockwise(self, edge_direction: Direction) -> Direction {
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
    use glam::Vec2;

    use super::{Direction, Hex, HexLayout, HexOrientation, Offset, OffsetCoordinate};

    /// Helper function to assert hex equality with descriptive error message
    fn assert_hex_eq(actual: Hex, expected: Hex, msg: &str) {
        assert_eq!(
            actual, expected,
            "{}: expected {:?}, got {:?}",
            msg, expected, actual
        );
    }

    /// Helper function to assert offset coordinate equality
    fn assert_offset_eq(actual: OffsetCoordinate, expected: OffsetCoordinate, msg: &str) {
        assert_eq!(
            actual, expected,
            "{}: expected {:?}, got {:?}",
            msg, expected, actual
        );
    }

    #[test]
    fn test_hex_neighbor_flat_orientation() {
        // Test flat-top orientation: South neighbor
        let center = Hex::new(1, -2);
        let expected = Hex::new(1, -3);
        let actual = center.neighbor(HexOrientation::Flat, Direction::South);
        assert_hex_eq(actual, expected, "Flat orientation South neighbor");
    }

    #[test]
    fn test_hex_neighbor_pointy_orientation() {
        // Test pointy-top orientation: SouthWest neighbor
        let center = Hex::new(1, -2);
        let expected = Hex::new(1, -3);
        let actual = center.neighbor(HexOrientation::Pointy, Direction::SouthWest);
        assert_hex_eq(actual, expected, "Pointy orientation SouthWest neighbor");
    }

    #[test]
    fn test_hex_distance_from_origin() {
        let hex = Hex::new(3, -7);
        let origin = Hex::new(0, 0);
        let distance = hex.distance_to(origin);
        assert_eq!(distance, 7, "Distance from (3,-7) to origin should be 7");
    }

    #[test]
    fn test_hex_distance_symmetric() {
        let a = Hex::new(2, -3);
        let b = Hex::new(-1, 4);
        let dist_ab = a.distance_to(b);
        let dist_ba = b.distance_to(a);
        assert_eq!(dist_ab, dist_ba, "Distance should be symmetric");
    }

    #[test]
    fn test_hex_round_interpolation() {
        // Test rounding at interpolation midpoint
        let start = Vec2::ZERO;
        let end = Vec2::new(10.0, -20.0);
        let midpoint = start.lerp(end, 0.5);
        let rounded = Hex::round(midpoint);
        assert_hex_eq(rounded, Hex::new(5, -10), "Rounding at 0.5 interpolation");
    }

    #[test]
    fn test_hex_round_bias_towards_start() {
        // Values < 0.5 should round towards start
        let a = Vec2::ZERO;
        let b = Vec2::new(1.0, -1.0);
        let biased = a.lerp(b, 0.499);
        let rounded = Hex::round(biased);
        assert_hex_eq(rounded, Hex::round(a), "Should bias towards start at 0.499");
    }

    #[test]
    fn test_hex_round_bias_towards_end() {
        // Values > 0.5 should round towards end
        let a = Vec2::ZERO;
        let b = Vec2::new(1.0, -1.0);
        let biased = a.lerp(b, 0.501);
        let rounded = Hex::round(biased);
        assert_hex_eq(rounded, Hex::round(b), "Should bias towards end at 0.501");
    }

    #[test]
    fn test_hex_round_weighted_average() {
        // Test rounding with weighted combination
        let a = Vec2::ZERO;
        let b = Vec2::new(1.0, -1.0);
        let c = Vec2::new(0.0, -1.0);

        // More weight on 'a' should round to 'a'
        let weighted_a = a * 0.4 + b * 0.3 + c * 0.3;
        assert_hex_eq(
            Hex::round(weighted_a),
            Hex::round(a),
            "Weighted average biased towards a",
        );

        // More weight on 'c' should round to 'c'
        let weighted_c = a * 0.3 + b * 0.3 + c * 0.4;
        assert_hex_eq(
            Hex::round(weighted_c),
            Hex::round(c),
            "Weighted average biased towards c",
        );
    }

    #[test]
    fn test_layout_flat_orientation_roundtrip() {
        let hex = Hex::new(3, 4);
        let layout = HexLayout {
            orientation: HexOrientation::Flat,
            size: [10.0, 15.0],
            origin: [35.0, 71.0],
        };

        // Convert hex → pixel → hex should return original
        let pixel = layout.hex_to_pixel(hex);
        let recovered = layout.pixel_to_hex(pixel.to_array());
        assert_hex_eq(recovered, hex, "Flat layout roundtrip conversion");
    }

    #[test]
    fn test_layout_pointy_orientation_roundtrip() {
        let hex = Hex::new(3, 4);
        let layout = HexLayout {
            orientation: HexOrientation::Pointy,
            size: [10.0, 15.0],
            origin: [35.0, 71.0],
        };

        // Convert hex → pixel → hex should return original
        let pixel = layout.hex_to_pixel(hex);
        let recovered = layout.pixel_to_hex(pixel.to_array());
        assert_hex_eq(recovered, hex, "Pointy layout roundtrip conversion");
    }

    #[test]
    fn test_offset_conversion_flat_even_roundtrip() {
        let hex = Hex::new(3, 4);
        let offset = hex.to_offset(HexOrientation::Flat, Offset::Even);
        let recovered = Hex::from_offset(offset, HexOrientation::Flat, Offset::Even);
        assert_hex_eq(recovered, hex, "Flat even-offset roundtrip");
    }

    #[test]
    fn test_offset_conversion_flat_odd_roundtrip() {
        let hex = Hex::new(3, 4);
        let offset = hex.to_offset(HexOrientation::Flat, Offset::Odd);
        let recovered = Hex::from_offset(offset, HexOrientation::Flat, Offset::Odd);
        assert_hex_eq(recovered, hex, "Flat odd-offset roundtrip");
    }

    #[test]
    fn test_offset_conversion_pointy_even_roundtrip() {
        let hex = Hex::new(3, 4);
        let offset = hex.to_offset(HexOrientation::Pointy, Offset::Even);
        let recovered = Hex::from_offset(offset, HexOrientation::Pointy, Offset::Even);
        assert_hex_eq(recovered, hex, "Pointy even-offset roundtrip");
    }

    #[test]
    fn test_offset_conversion_pointy_odd_roundtrip() {
        let hex = Hex::new(3, 4);
        let offset = hex.to_offset(HexOrientation::Pointy, Offset::Odd);
        let recovered = Hex::from_offset(offset, HexOrientation::Pointy, Offset::Odd);
        assert_hex_eq(recovered, hex, "Pointy odd-offset roundtrip");
    }

    #[test]
    fn test_offset_coordinate_to_hex_flat_even() {
        let offset = OffsetCoordinate::new(1, -3);
        let hex = Hex::from_offset(offset, HexOrientation::Flat, Offset::Even);
        let recovered = hex.to_offset(HexOrientation::Flat, Offset::Even);
        assert_offset_eq(recovered, offset, "Offset to hex conversion (flat, even)");
    }

    #[test]
    fn test_offset_coordinate_to_hex_flat_odd() {
        let offset = OffsetCoordinate::new(1, -3);
        let hex = Hex::from_offset(offset, HexOrientation::Flat, Offset::Odd);
        let recovered = hex.to_offset(HexOrientation::Flat, Offset::Odd);
        assert_offset_eq(recovered, offset, "Offset to hex conversion (flat, odd)");
    }

    #[test]
    fn test_offset_coordinate_to_hex_pointy_even() {
        let offset = OffsetCoordinate::new(1, -3);
        let hex = Hex::from_offset(offset, HexOrientation::Pointy, Offset::Even);
        let recovered = hex.to_offset(HexOrientation::Pointy, Offset::Even);
        assert_offset_eq(recovered, offset, "Offset to hex conversion (pointy, even)");
    }

    #[test]
    fn test_offset_coordinate_to_hex_pointy_odd() {
        let offset = OffsetCoordinate::new(1, -3);
        let hex = Hex::from_offset(offset, HexOrientation::Pointy, Offset::Odd);
        let recovered = hex.to_offset(HexOrientation::Pointy, Offset::Odd);
        assert_offset_eq(recovered, offset, "Offset to hex conversion (pointy, odd)");
    }

    #[test]
    fn test_hex_to_offset_flat_even() {
        let hex = Hex::new(1, 2);
        let expected = OffsetCoordinate::new(1, 3);
        let actual = hex.to_offset(HexOrientation::Flat, Offset::Even);
        assert_offset_eq(actual, expected, "Hex to offset (flat, even)");
    }

    #[test]
    fn test_hex_to_offset_flat_odd() {
        let hex = Hex::new(1, 2);
        let expected = OffsetCoordinate::new(1, 2);
        let actual = hex.to_offset(HexOrientation::Flat, Offset::Odd);
        assert_offset_eq(actual, expected, "Hex to offset (flat, odd)");
    }

    #[test]
    fn test_offset_to_hex_flat_even() {
        let offset = OffsetCoordinate::new(1, 3);
        let expected = Hex::new(1, 2);
        let actual = Hex::from_offset(offset, HexOrientation::Flat, Offset::Even);
        assert_hex_eq(actual, expected, "Offset to hex (flat, even)");
    }

    #[test]
    fn test_offset_to_hex_flat_odd() {
        let offset = OffsetCoordinate::new(1, 2);
        let expected = Hex::new(1, 2);
        let actual = Hex::from_offset(offset, HexOrientation::Flat, Offset::Odd);
        assert_hex_eq(actual, expected, "Offset to hex (flat, odd)");
    }

    #[test]
    fn test_hex_coordinates_accessors() {
        let hex = Hex::new(5, -3);
        assert_eq!(hex.x(), 5, "X coordinate accessor");
        assert_eq!(hex.y(), -3, "Y coordinate accessor");
        assert_eq!(hex.z(), -2, "Z coordinate (should be -x-y)");
    }

    #[test]
    fn test_hex_addition() {
        let a = Hex::new(2, -1);
        let b = Hex::new(-1, 3);
        let sum = a + b;
        assert_hex_eq(sum, Hex::new(1, 2), "Hex addition");
    }

    #[test]
    fn test_hex_subtraction() {
        let a = Hex::new(5, -2);
        let b = Hex::new(2, 1);
        let diff = a - b;
        assert_hex_eq(diff, Hex::new(3, -3), "Hex subtraction");
    }

    #[test]
    fn test_hex_length() {
        assert_eq!(Hex::new(0, 0).length(), 0, "Origin length");
        assert_eq!(Hex::new(1, 0).length(), 1, "Unit X length");
        assert_eq!(Hex::new(0, 1).length(), 1, "Unit Y length");
        assert_eq!(Hex::new(3, -4).length(), 4, "Longer distance");
    }

    #[test]
    fn test_hex_equality() {
        let a = Hex::new(2, -3);
        let b = Hex::new(2, -3);
        let c = Hex::new(2, -2);
        assert_eq!(a, b, "Equal hexes");
        assert_ne!(a, c, "Different hexes");
    }
}
