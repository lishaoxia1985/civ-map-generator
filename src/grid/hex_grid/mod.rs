use glam::{DVec2, IVec3};
use hex::{Hex, HexLayout, Offset};

use crate::{
    grid::{offset_coordinate::OffsetCoordinate, Cell},
    map_parameters::WorldSize,
};

use super::{direction::Direction, Grid, Size, WrapFlags};

pub mod hex;

#[derive(Clone, Copy)]
pub struct HexGrid {
    pub size: Size,
    pub layout: HexLayout,
    pub wrap_flags: WrapFlags,
    pub offset: Offset,
}

impl HexGrid {
    /// Get the world size of the grid based on its dimensions.
    ///
    /// Maybe be as one function of trait in the future?
    pub fn get_world_size(&self) -> WorldSize {
        let width = self.size.width;
        let height = self.size.height;
        let area = width * height;
        match area {
            // When area <= 40 * 24, set the WorldSize to "Duel" and give a warning message
            area if area < 960 => {
                eprintln!(
                    "The map size is too small. The provided dimensions are {}x{}, which gives an area of {}. The minimum area is 40 * 24 = 960 in the original CIV5 game.",
                    width, height, area
                );
                WorldSize::Duel
            }
            // For "Duel" size: area <= 56 * 36
            area if area < 2016 => WorldSize::Duel,
            // For "Tiny" size: area <= 66 * 42
            area if area < 2772 => WorldSize::Tiny,
            // For "Small" size: area <= 80 * 52
            area if area < 4160 => WorldSize::Small,
            // For "Standard" size: area <= 104 * 64
            area if area < 6656 => WorldSize::Standard,
            // For "Large" size: area <= 128 * 80
            area if area < 10240 => WorldSize::Large,
            // For "Huge" size: area >= 128 * 80
            _ => WorldSize::Huge,
        }
    }

    /// Set the default size of the grid based on the provided `WorldSize`.
    ///
    /// Maybe be as one function of trait in the future?
    pub fn set_default_size(&mut self, world_size: WorldSize) {
        let (width, height) = match world_size {
            WorldSize::Duel => (40, 24),
            WorldSize::Tiny => (56, 36),
            WorldSize::Small => (66, 42),
            WorldSize::Standard => (80, 52),
            WorldSize::Large => (104, 64),
            WorldSize::Huge => (128, 80),
        };
        let size = Size { width, height };
        self.size = size;
    }
}

impl Grid for HexGrid {
    type GridCoordinateType = Hex;

    type DirectionArrayType = [Direction; 6];

    fn edge_direction_array(&self) -> [Direction; 6] {
        self.layout.orientation.edge_direction()
    }

    fn corner_direction_array(&self) -> [Direction; 6] {
        self.layout.orientation.corner_direction()
    }

    fn size(&self) -> Size {
        self.size
    }

    fn wrap_flags(&self) -> WrapFlags {
        self.wrap_flags
    }

    fn center(&self) -> DVec2 {
        let width = self.size.width;
        let height = self.size.height;

        let bottom_left_offset_coordinates = [
            OffsetCoordinate::new(0, 0),
            OffsetCoordinate::new(1, 0),
            OffsetCoordinate::new(0, 1),
        ];

        let (min_offset_x, min_offset_y) = bottom_left_offset_coordinates.into_iter().fold(
            (0.0_f64, 0.0_f64),
            |(min_offset_x, min_offset_y), offset_coordinate| {
                let hex = Hex::from_offset(offset_coordinate, self.layout.orientation, self.offset);

                let [offset_x, offset_y] = self.layout.hex_to_pixel(hex).to_array();
                (min_offset_x.min(offset_x), min_offset_y.min(offset_y))
            },
        );

        let top_right_offset_coordinates = [
            OffsetCoordinate::from([width - 1, height - 1]),
            OffsetCoordinate::from([width - 2, height - 1]),
            OffsetCoordinate::from([width - 1, height - 2]),
        ];

        let (max_offset_x, max_offset_y) = top_right_offset_coordinates.into_iter().fold(
            (0.0_f64, 0.0_f64),
            |(max_offset_x, max_offset_y), offset_coordinate| {
                let hex = Hex::from_offset(offset_coordinate, self.layout.orientation, self.offset);

                let [offset_x, offset_y] = self.layout.hex_to_pixel(hex).to_array();
                (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
            },
        );

        DVec2::new(
            (min_offset_x + max_offset_x) / 2.,
            (min_offset_y + max_offset_y) / 2.,
        )
    }

    // Convert the hex coordinate to an offset coordinate
    fn grid_coordinate_to_offset(&self, grid_coordinate: Hex) -> OffsetCoordinate {
        let offset_coordinate = grid_coordinate.to_offset(self.layout.orientation, self.offset);

        self.normalize_offset(offset_coordinate).expect(&format!(
            "Offset coordinate out of bounds: ({}, {})",
            offset_coordinate.0.x, offset_coordinate.0.y
        ))
    }

    fn distance_to(&self, start: Cell, dest: Cell) -> i32 {
        let start = self.cell_to_offset(start);
        let dest = self.cell_to_offset(dest);

        let [mut dest_x, mut dest_y] = dest.to_array();

        let [x, y] = (dest.0 - start.0).to_array();
        if self.wrap_x() {
            if x > self.width() as i32 / 2 {
                // Wrap around the x-axis
                dest_x -= self.width() as i32;
            }
            if x < -(self.width() as i32) / 2 {
                // Wrap around the x-axis
                dest_x += self.width() as i32;
            }
        }

        if self.wrap_y() {
            if y > self.height() as i32 / 2 {
                // Wrap around the y-axis
                dest_y -= self.height() as i32;
            }
            if y < -(self.height() as i32) / 2 {
                // Wrap around the y-axis
                dest_y += self.height() as i32;
            }
        }

        let dest = OffsetCoordinate::new(dest_x, dest_y);

        let dest_hex = Hex::from_offset(dest, self.layout.orientation, self.offset);
        let start_hex = Hex::from_offset(start, self.layout.orientation, self.offset);

        start_hex.distance_to(dest_hex)
    }

    fn neighbor(&self, center: Cell, direction: Direction) -> Option<Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        let neighbor_offset_coordinate = center_hex
            .neighbor(self.layout.orientation, direction)
            .to_offset(self.layout.orientation, self.offset);
        self.offset_to_cell(neighbor_offset_coordinate).ok()
    }

    fn cells_at_distance(&self, center: Cell, distance: u32) -> Vec<Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        center_hex
            .hexes_at_distance(distance)
            .iter()
            .filter_map(|&hex_coordinate| {
                let offset_coordinate = self.grid_coordinate_to_offset(hex_coordinate);

                self.offset_to_cell(offset_coordinate).ok()
            })
            .collect()
    }

    fn cells_within_distance(&self, center: Cell, distance: u32) -> Vec<Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        center_hex
            .hexes_in_distance(distance)
            .iter()
            .filter_map(|&hex_coordinate| {
                let offset_coordinate = self.grid_coordinate_to_offset(hex_coordinate);

                self.offset_to_cell(offset_coordinate).ok()
            })
            .collect()
    }

    fn estimate_direction(&self, start: Cell, dest: Cell) -> Option<Direction> {
        let start = self.cell_to_offset(start);
        let dest = self.cell_to_offset(dest);

        // If the start and dest are the same, return `None`.
        if start == dest {
            return None;
        }

        let [mut dest_x, mut dest_y] = dest.to_array();

        let [x, y] = (dest.0 - start.0).to_array();

        // If the map is wrapping, adjust the dest's offset coordinate accordingly.
        // The distance from the dest to the start's left may be shorter than the distance from the dest to the start's right.
        // So we make sure the distance from the dest to the start is always shortest.
        if self.wrap_x() {
            if x > self.width() as i32 / 2 {
                // Wrap around the x-axis
                dest_x -= self.width() as i32;
            }
            if x < -(self.width() as i32) / 2 {
                // Wrap around the x-axis
                dest_x += self.width() as i32;
            }
        }

        // The distance from the dest to the start's top may be shorter than the distance from the dest to the start's bottom.
        // So we make sure the distance from the dest to the start is always shortest.
        if self.wrap_y() {
            if y > self.height() as i32 / 2 {
                // Wrap around the y-axis
                dest_y -= self.height() as i32;
            }
            if y < -(self.height() as i32) / 2 {
                // Wrap around the y-axis
                dest_y += self.height() as i32;
            }
        }

        let dest = OffsetCoordinate::new(dest_x, dest_y);

        let dest_hex = Hex::from_offset(dest, self.layout.orientation, self.offset);
        let start_hex = Hex::from_offset(start, self.layout.orientation, self.offset);

        let estimate_vector = dest_hex - start_hex;

        let estimate_cube_vector = IVec3::new(
            estimate_vector.x(),
            estimate_vector.y(),
            estimate_vector.z(),
        );

        let edge_direction_array = self.edge_direction_array();

        let origin = Hex::new(0, 0);

        let max_direction = edge_direction_array.into_iter().max_by_key(|&direction| {
            let unit_direction_vector = origin.neighbor(self.layout.orientation, direction);
            let unit_direction_cube_vector = IVec3::new(
                unit_direction_vector.x(),
                unit_direction_vector.y(),
                unit_direction_vector.z(),
            );
            estimate_cube_vector.dot(unit_direction_cube_vector)
        });

        max_direction
    }
}
