use crate::grid::{
    Cell, Grid, GridSize, WorldSizeType, direction::Direction, offset_coordinate::OffsetCoordinate,
};

use super::{Size, WrapFlags};

mod square;

pub use square::*;

#[derive(Clone, Copy)]
pub struct SquareGrid {
    pub size: Size,
    pub layout: SquareLayout,
    pub wrap_flags: WrapFlags,
}

impl SquareGrid {
    pub fn new(size: Size, layout: SquareLayout, wrap_flags: WrapFlags) -> Self {
        Self {
            size,
            layout,
            wrap_flags,
        }
    }
}

impl Grid for SquareGrid {
    type GridCoordinateType = Square;

    type DirectionArrayType = [Direction; 4];

    fn edge_direction_array(&self) -> Self::DirectionArrayType {
        self.layout.orientation.edge_direction()
    }

    fn corner_direction_array(&self) -> Self::DirectionArrayType {
        self.layout.orientation.corner_direction()
    }

    fn size(&self) -> Size {
        self.size
    }

    fn wrap_flags(&self) -> WrapFlags {
        self.wrap_flags
    }

    fn center(&self) -> [f32; 2] {
        let width = self.size.width;
        let height = self.size.height;

        let [min_offset_x, min_offset_y] =
            self.layout.square_to_pixel(Square::new(0, 0)).to_array();

        let [max_offset_x, max_offset_y] = self
            .layout
            .square_to_pixel(Square::new(width as i32 - 1, height as i32 - 1))
            .to_array();

        [
            (min_offset_x + max_offset_x) / 2.,
            (min_offset_y + max_offset_y) / 2.,
        ]
    }

    fn grid_coordinate_to_cell(&self, grid_coordinate: Self::GridCoordinateType) -> Option<Cell> {
        // Convert the square coordinate to an offset coordinate
        let offset_coordinate = grid_coordinate.to_offset();

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

        let dest_square = Square::from_offset(dest);
        let start_square = Square::from_offset(start);

        start_square.distance_to(dest_square)
    }

    fn neighbor(self, center: Cell, direction: Direction) -> Option<Cell> {
        let center = self.cell_to_offset(center);

        let center_square = Square::from_offset(center);
        let neighbor_offset_coordinate = center_square
            .neighbor(self.layout.orientation, direction)
            .to_offset();
        self.offset_to_cell(neighbor_offset_coordinate).ok()
    }

    fn cells_at_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell> {
        let center = self.cell_to_offset(center);

        let center_square = Square::from_offset(center);
        center_square
            .squares_at_distance(distance)
            .into_iter()
            .filter_map(move |square| self.grid_coordinate_to_cell(square))
    }

    fn cells_within_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell> {
        let center = self.cell_to_offset(center);

        let center_square = Square::from_offset(center);
        center_square
            .squares_in_distance(distance)
            .into_iter()
            .filter_map(move |square| self.grid_coordinate_to_cell(square))
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

        let dest_square = Square::from_offset(dest);
        let start_square = Square::from_offset(start);

        let estimate_vector = (dest_square - start_square).into_inner();

        let edge_direction_array = self.edge_direction_array();

        let origin = Square::new(0, 0);

        edge_direction_array.into_iter().max_by_key(|&direction| {
            let unit_direction_vector = origin
                .neighbor(self.layout.orientation, direction)
                .into_inner();
            estimate_vector.dot(unit_direction_vector)
        })
    }
}

impl GridSize for SquareGrid {
    fn default_size(world_size_type: WorldSizeType) -> Size {
        // Define the default size for each world size type, according to CIV4 standards.
        match world_size_type {
            WorldSizeType::Duel => Size {
                width: 48,
                height: 32,
            },
            WorldSizeType::Tiny => Size {
                width: 60,
                height: 40,
            },
            WorldSizeType::Small => Size {
                width: 72,
                height: 48,
            },
            WorldSizeType::Standard => Size {
                width: 96,
                height: 64,
            },
            WorldSizeType::Large => Size {
                width: 120,
                height: 80,
            },
            WorldSizeType::Huge => Size {
                width: 144,
                height: 96,
            },
        }
    }
}
