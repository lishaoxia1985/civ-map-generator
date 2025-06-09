use glam::DVec2;
use hex::{HexLayout, Offset};

use crate::{map_parameters::WorldSize, tile::Tile};

use super::{direction::Direction, Size, WrapFlags};

pub mod hex;

#[derive(Clone, Copy)]
pub struct HexGrid {
    pub size: Size,
    pub hex_layout: HexLayout,
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

    /// Get the center of the grid in pixel coordinates.
    ///
    /// # Notice
    /// When we show the map, we need to set camera to the center of the map.
    pub fn center(&self) -> DVec2 {
        let width = self.size.width;
        let height = self.size.height;

        let (min_offset_x, min_offset_y) = [0, 1, width].into_iter().fold(
            (0.0_f64, 0.0_f64),
            |(min_offset_x, min_offset_y), index| {
                let hex = Tile::new(index as usize).to_hex_coordinate(*self);

                let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
                (min_offset_x.min(offset_x), min_offset_y.min(offset_y))
            },
        );

        let (max_offset_x, max_offset_y) = [
            width * (height - 1) - 1,
            width * height - 2,
            width * height - 1,
        ]
        .into_iter()
        .fold((0.0_f64, 0.0_f64), |(max_offset_x, max_offset_y), index| {
            let hex = Tile::new(index as usize).to_hex_coordinate(*self);

            let [offset_x, offset_y] = self.hex_layout.hex_to_pixel(hex).to_array();
            (max_offset_x.max(offset_x), max_offset_y.max(offset_y))
        });

        DVec2::new(
            (min_offset_x + max_offset_x) / 2.,
            (min_offset_y + max_offset_y) / 2.,
        )
    }

    pub const fn edge_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.edge_direction()
    }

    pub const fn corner_direction_array(&self) -> [Direction; 6] {
        self.hex_layout.orientation.corner_direction()
    }
}
