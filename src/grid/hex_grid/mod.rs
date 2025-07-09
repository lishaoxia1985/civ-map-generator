use glam::{IVec3, Vec2};
use hex::{Hex, HexLayout, Offset};

use crate::grid::{offset_coordinate::OffsetCoordinate, Cell, GridSize, WorldSizeType};

use super::{direction::Direction, Grid, Size, WrapFlags};

pub mod hex;

#[derive(Clone, Copy)]
pub struct HexGrid {
    pub size: Size,
    pub layout: HexLayout,
    pub offset: Offset,
    pub wrap_flags: WrapFlags,
}

impl HexGrid {
    pub fn new(size: Size, layout: HexLayout, offset: Offset, wrap_flags: WrapFlags) -> Self {
        use crate::grid::hex_grid::hex::HexOrientation;

        match layout.orientation {
            HexOrientation::Pointy => {
                if wrap_flags.contains(WrapFlags::WrapY) && size.height % 2 == 1 {
                    panic!("For pointy hexes, height must be even when wrapping on the y-axis.");
                }
            }
            HexOrientation::Flat => {
                if wrap_flags.contains(WrapFlags::WrapX) && size.width % 2 == 1 {
                    panic!("For flat hexes, width must be even when wrapping on the x-axis.");
                }
            }
        }

        Self {
            size,
            layout,
            offset,
            wrap_flags,
        }
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

    fn center(&self) -> Vec2 {
        let width = self.size.width;
        let height = self.size.height;

        let bottom_left_offset_coordinates = [
            OffsetCoordinate::new(0, 0),
            OffsetCoordinate::new(1, 0),
            OffsetCoordinate::new(0, 1),
        ];

        let (min_offset_x, min_offset_y) = bottom_left_offset_coordinates.into_iter().fold(
            (0.0_f32, 0.0_f32),
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
            (0.0_f32, 0.0_f32),
            |(max_offset_x, max_offset_y), offset_coordinate| {
                let hex = Hex::from_offset(offset_coordinate, self.layout.orientation, self.offset);

                let [offset_x, offset_y] = self.layout.hex_to_pixel(hex).to_array();
                (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
            },
        );

        Vec2::new(
            (min_offset_x + max_offset_x) / 2.,
            (min_offset_y + max_offset_y) / 2.,
        )
    }

    fn grid_coordinate_to_cell(&self, grid_coordinate: Hex) -> Option<Cell> {
        let offset_coordinate = grid_coordinate.to_offset(self.layout.orientation, self.offset);

        self.offset_to_cell(offset_coordinate).ok()
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

    fn neighbor(self, center: Cell, direction: Direction) -> Option<Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        let neighbor_offset_coordinate = center_hex
            .neighbor(self.layout.orientation, direction)
            .to_offset(self.layout.orientation, self.offset);
        self.offset_to_cell(neighbor_offset_coordinate).ok()
    }

    fn cells_at_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        center_hex
            .hexes_at_distance(distance)
            .into_iter()
            .filter_map(move |hex| self.grid_coordinate_to_cell(hex))
    }

    fn cells_within_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell> {
        let center = self.cell_to_offset(center);

        let center_hex = Hex::from_offset(center, self.layout.orientation, self.offset);
        center_hex
            .hexes_in_distance(distance)
            .into_iter()
            .filter_map(move |hex| self.grid_coordinate_to_cell(hex))
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

impl GridSize for HexGrid {
    // Define the default size for each world size type, according to CIV5 standards.
    fn default_size(world_size_type: WorldSizeType) -> Size {
        match world_size_type {
            WorldSizeType::Duel => Size {
                width: 40,
                height: 24,
            },
            WorldSizeType::Tiny => Size {
                width: 56,
                height: 36,
            },
            WorldSizeType::Small => Size {
                width: 66,
                height: 42,
            },
            WorldSizeType::Standard => Size {
                width: 80,
                height: 52,
            },
            WorldSizeType::Large => Size {
                width: 104,
                height: 64,
            },
            WorldSizeType::Huge => Size {
                width: 128,
                height: 80,
            },
        }
    }
}
