//! This module provides functionality for generating and manipulating fractal maps which can be used in games like Civilization.

use std::{
    cmp::{max, min},
    path::Path,
};

use bitflags::bitflags;

use image::{
    imageops::{resize, FilterType},
    DynamicImage, GrayImage, ImageBuffer,
};
use rand::{rngs::StdRng, seq::SliceRandom, Rng};

use crate::grid::{
    direction::Direction, hex_grid::HexGrid, offset_coordinate::OffsetCoordinate, Cell, Grid, Size,
    WrapFlags,
};

/// A seed for the Voronoi diagram in the fractal grid.
struct VoronoiSeed {
    /// The cell of the seed in the fractal grid.
    pub cell: Cell,
    /// Implies the influence of the seed on its surrounding area.
    pub weakness: u32,
    /// Indicates the preferred direction or bias when assigning points within its influence region during the generation of the diagram.
    pub bias_direction: Direction,
    /// The strength of the bias direction.
    pub directional_bias_strength: u32,
}

impl VoronoiSeed {
    /// Generates a random seed for the fractal.
    pub fn gen_random_seed(random: &mut StdRng, fractal_grid: HexGrid) -> Self {
        let offset_coordinate = OffsetCoordinate::from([
            random.gen_range(0..fractal_grid.width()),
            random.gen_range(0..fractal_grid.height()),
        ]);

        let cell = fractal_grid.offset_to_cell(offset_coordinate).unwrap();

        let weakness = random.gen_range(0..6);

        let bias_direction = *fractal_grid
            .layout
            .orientation
            .edge_direction()
            .choose(random)
            .unwrap();

        let directional_bias_strength = random.gen_range(0..4);

        VoronoiSeed {
            cell,
            weakness,
            bias_direction,
            directional_bias_strength,
        }
    }
}

pub struct CvFractal {
    /// Store the map/world size, equal to the grid size which is used in the game. The grid is different from [`CvFractal::fractal_grid`].
    map_size: Size,
    /// The fractal grid, different from the grid used in the game, it is used to store the fractal's values.
    /// Width resolution of the fractal grid is `1 << width_exp`, is a power of 2.
    /// Height resolution of the fractal grid is `1 << height_exp`, is a power of 2.
    fractal_grid: HexGrid,
    /// The fractal's flags. It is used to control the fractal's generation process.
    flags: FractalFlags,
    /// It is an exponent related to the width of the source fractal,
    /// `width_exp = 7` means the width of the source fractal is `2^7`
    width_exp: u32,
    /// It is an exponent related to the height of the source fractal,
    /// `height_exp = 7` means the height of the source fractal is `2^7`
    height_exp: u32,
    /// Stores the 2D fractal array, the array size is `[fractal_width + 1][fractal_height + 1]`.\
    /// **NOTICE**: The last column and last row are not part of the fractal, they are only used to calculate the fractal values.
    fractal_array: Vec<Vec<u32>>,
}

bitflags! {
    /// Flags for the CvFractal. It is used to control the behavior of the fractal generation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct FractalFlags: u8 {
        /// When flag is set, The closer to the edge of the grid, the closer the value of the height to 0.
        ///
        /// # Notice
        ///
        /// When grid is wrapped in the X direction, ignored in the X direction.
        /// When grid is wrapped in the Y direction, ignored in the Y direction.
        const Polar = 0b00000001;
        /// When flag is set, the value of the height is in `0..=99`, otherwise the value is in `0..=255`
        const Percent = 0b00000010;
        /// When flag is set, draws rift in center of world
        const CenterRift = 0b00000100;
        /// When flag is set, inverts the fractal height values, e.g. the original value is `fractal_value`, the new value is `255 - fractal_value`
        const InvertHeights = 0b00001000;
    }
}

impl CvFractal {
    pub const DEFAULT_WIDTH_EXP: u32 = 7;
    pub const DEFAULT_HEIGHT_EXP: u32 = 6;

    /// Creates a new empty fractal with the given parameters.
    ///
    /// # Arguments
    ///
    /// - `grid`: The base map/world grid (distinct from [`CvFractal::fractal_grid`])
    /// - `flags`: Bit flags controlling fractal generation behavior
    /// - `width_exp`: The exponent for calculating fractal width (width = 2^width_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_WIDTH_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_WIDTH_EXP`] directly
    /// - `height_exp`: The exponent for calculating fractal height (height = 2^height_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_HEIGHT_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_HEIGHT_EXP`] directly
    fn new(grid: HexGrid, flags: FractalFlags, width_exp: u32, height_exp: u32) -> Self {
        let map_size = grid.size;

        let fractal_width = 1 << width_exp;
        let fractal_height = 1 << height_exp;

        let fractal_grid = HexGrid {
            size: Size::new(fractal_width, fractal_height),
            ..grid
        };

        let fractal_array =
            vec![vec![0; (fractal_height + 1) as usize]; (fractal_width + 1) as usize];

        Self {
            map_size,
            fractal_grid,
            fractal_array,
            flags,
            width_exp,
            height_exp,
        }
    }

    /// Creates a fractal with the given parameters.
    ///
    /// # Arguments
    ///
    /// - `random`: A mutable reference to a random number generator.
    /// - `grid`: The base map/world grid (distinct from [`CvFractal::fractal_grid`])
    /// - `grain`: Controls the level of detail or smoothness of the fractal.\
    ///     - Valid range: `[min(width_exp, height_exp).saturating_sub(7), min(width_exp, height_exp)]`
    ///     - Effect on algorithm: The value determines how many iterations the diamond-square algorithm will execute.
    ///         - Higher grain → Fewer iterations → More random noise\
    ///             When `grain = min(width_exp, height_exp)`, the fractal is completely random.
    ///         - Lower grain → More iterations → Smoother gradients\
    ///             When `grain = min(width_exp, height_exp).saturating_sub(7)`, the fractal is the smoothest.
    /// - `flags`: Bit flags controlling fractal generation behavior
    /// - `width_exp`: The exponent for calculating fractal width (width = 2^width_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_WIDTH_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_WIDTH_EXP`] directly
    /// - `height_exp`: The exponent for calculating fractal height (height = 2^height_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_HEIGHT_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_HEIGHT_EXP`] directly
    ///
    /// # Panics
    ///
    /// Panics if `grain` is invalid.
    pub fn create(
        random: &mut StdRng,
        grid: HexGrid,
        grain: u32,
        flags: FractalFlags,
        width_exp: u32,
        height_exp: u32,
    ) -> Self {
        let min_exp = min(width_exp, height_exp);
        let min_allowed = min_exp.saturating_sub(7);
        debug_assert!(
            grain >= min_allowed && grain <= min_exp,
            "grain should be in range [{}, {}] (inclusive), but got grain = {}, width_exp = {}, height_exp = {}",
            min_allowed,
            min_exp,
            grain,
            width_exp,
            height_exp
        );

        let mut fractal = Self::new(grid, flags, width_exp, height_exp);
        fractal.frac_init_internal(random, grain, None, None);
        fractal
    }

    ///  Creates a fractal with rifts.
    ///
    /// # Arguments
    ///
    /// - `random`: Random number generator
    /// - `grid`: The base map/world grid (distinct from [`CvFractal::fractal_grid`])
    /// - `grain`: Controls the level of detail or smoothness of the fractal.\
    ///     - Valid range: `[min(width_exp, height_exp).saturating_sub(7), min(width_exp, height_exp)]`
    ///     - Effect on algorithm: The value determines how many iterations the diamond-square algorithm will execute.
    ///         - Higher grain → Fewer iterations → More random noise\
    ///             When `grain = min(width_exp, height_exp)`, the fractal is completely random.
    ///         - Lower grain → More iterations → Smoother gradients\
    ///             When `grain = min(width_exp, height_exp).saturating_sub(7)`, the fractal is the smoothest.
    /// - `flags`: Bit flags controlling fractal generation behavior
    /// - `rift`: a reference to a [`CvFractal`] to control the rifts generation of the fractal
    /// - `width_exp`: The exponent for calculating fractal width (width = 2^width_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_WIDTH_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_WIDTH_EXP`] directly
    /// - `height_exp`: The exponent for calculating fractal height (height = 2^height_exp)
    ///   - Type: `u32` (unlike original CIV5 which allowed negatives)
    ///   - Original behavior: Negative values would default to [`CvFractal::DEFAULT_HEIGHT_EXP`]
    ///   - To replicate original behavior with negative values, use [`CvFractal::DEFAULT_HEIGHT_EXP`] directly
    ///
    /// # Panics
    ///
    /// Panics if `grain` is invalid.
    ///
    /// # Notice
    ///
    /// Original CIV5 only supports to create vertical rifts when the fractal is WrapX.
    /// This function support to create both vertical and horizontal rifts.
    /// But we suggest to create only one of them at a time.
    pub fn create_rifts(
        random: &mut StdRng,
        grid: HexGrid,
        grain: u32,
        flags: FractalFlags,
        rifts: &CvFractal,
        width_exp: u32,
        height_exp: u32,
    ) -> Self {
        let min_exp = min(width_exp, height_exp);
        let min_allowed = min_exp.saturating_sub(7);
        debug_assert!(
            grain >= min_allowed && grain <= min_exp,
            "grain should be in range [{}, {}] (inclusive), but got grain = {}, width_exp = {}, height_exp = {}",
            min_allowed,
            min_exp,
            grain,
            width_exp,
            height_exp
        );

        let mut fractal = Self::new(grid, flags, width_exp, height_exp);
        fractal.frac_init_internal(random, grain, None, Some(rifts));
        fractal
    }

    /// Generate a fractal with the given parameters.
    /// # Arguments
    ///
    /// - `random`: Random number generator.
    /// - `grain`: Controls the level of detail or smoothness of the fractal.\
    ///     - Valid range: `[min(width_exp, height_exp).saturating_sub(7), min(width_exp, height_exp)]`
    ///     - Effect on algorithm: The value determines how many iterations the diamond-square algorithm will execute.
    ///         - Higher grain → Fewer iterations → More random noise\
    ///             When `grain = min(width_exp, height_exp)`, the fractal is completely random.
    ///         - Lower grain → More iterations → Smoother gradients\
    ///             When `grain = min(width_exp, height_exp).saturating_sub(7)`, the fractal is the smoothest.
    /// - `img`: Optional image to use as a source for the fractal.
    ///   At first the fractal is divided into a grid of small squares,
    ///   the points where adjacent grid lines meet are called **the vertices of the grid**,
    ///   we assign an initial value to each vertex for later use in the diamond-square algorithm.
    ///     - When `img` is `None`, the initial value of each vertex is randomly generated.
    ///     - When `img` is `Some`, the initial value of each vertex is determined by `img`.
    /// - `rifts`: Optional fractal to use as a source for the fractal to add rifts. When `rifts` is `None`, no rifts are added.
    ///     - `rifts`'s width and height must be equal to the width and height of `self`.
    ///
    /// # Panics
    ///
    /// Panics if `grain` is invalid.
    fn frac_init_internal(
        &mut self,
        random: &mut StdRng,
        grain: u32,
        img: Option<&DynamicImage>,
        rifts: Option<&CvFractal>,
    ) {
        let fractal_width = self.fractal_grid.size.width;
        let fractal_height = self.fractal_grid.size.height;

        let min_exp = min(self.width_exp, self.height_exp);
        let min_allowed = min_exp.saturating_sub(7);
        debug_assert!(
            grain >= min_allowed && grain <= min_exp,
            "grain should be in range [{}, {}] (inclusive), but got grain = {}, width_exp = {}, height_exp = {}",
            min_allowed,
            min_exp,
            grain,
            self.width_exp,
            self.height_exp
        );

        // Convert to usize for the following calculations.
        let smooth = (min_exp - grain) as usize;

        // At first, We should divide the fractal into a grid of small (2^smooth) * (2^smooth) squares,
        // When the fractal is divided into a grid of small squares, the points where adjacent grid lines meet are called `the vertices of the grid`(abbreviated as `Vertices`).
        // `hint_width` is the num of `Vertices` in every row after dividing.
        // Notice: when the fractal is WrapX, we don't consider the last row,
        //      because the last row of the fractal is the same as the first row,
        //      We preprocess this case at the beginning of every iter stage in Diamond-Square algorithm.
        let hint_width = (fractal_width >> smooth)
            + if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapX) {
                0
            } else {
                1
            };
        // `hint_height` is the num of `Vertices` in every column after dividing.
        // Notice: when the fractal is WrapY, we don't consider the last column,
        //      because the last column of the fractal is the same as the first column,
        //      We preprocess this case at the beginning of every iter in Diamond-Square algorithm.
        let hint_height = (fractal_height >> smooth)
            + if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapY) {
                0
            } else {
                1
            };

        if let Some(img) = img {
            // Resize image if needed with informative message
            let hint_img = if hint_width != img.width() || hint_height != img.height() {
                eprintln!(
                    "Image size {}x{} doesn't match hint size {}x{}.
                    We will resize the image to the hint size.
                    Please check if it is the correct behavior.
                    Resizing...",
                    img.width(),
                    img.height(),
                    hint_width,
                    hint_height
                );
                img.resize_exact(hint_width, hint_height, FilterType::Triangle)
            } else {
                eprintln!(
                    "Image size matches hint size: {}x{}",
                    img.width(),
                    img.height()
                );
                img.clone()
            };

            // Convert to grayscale and create hint array
            let gray_hint_img = hint_img.to_luma8();

            // Assign an initial value to each vertex by `gray_hint_img` for later use in the diamond-square algorithm.
            for x in 0..hint_width as usize {
                for y in 0..hint_height as usize {
                    self.fractal_array[x << smooth][y << smooth] =
                        gray_hint_img.get_pixel(x as u32, y as u32)[0] as u32;
                }
            }
        } else {
            // Assign an initial value to each vertex by random number generator for later use in the diamond-square algorithm.
            for x in 0..hint_width as usize {
                for y in 0..hint_height as usize {
                    self.fractal_array[x << smooth][y << smooth] = random.gen_range(0..256);
                    // Fractal Gen 1
                }
            }
        }

        for pass in (0..smooth).rev() {
            /*********** start to preprocess fractal_array[][] at the beginning of every iter stage in Diamond-Square algorithm. ***********/

            // If wrapping in the Y direction is needed, copy the bottom row to the top
            if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapY) {
                for x in 0..=fractal_width as usize {
                    self.fractal_array[x][fractal_height as usize] = self.fractal_array[x][0];
                }
            } else if self.flags.contains(FractalFlags::Polar) {
                // Polar coordinate transformation, the top and bottom row will be set to 0
                for x in 0..=fractal_width as usize {
                    self.fractal_array[x][0] = 0;
                    self.fractal_array[x][fractal_height as usize] = 0;
                }
            }

            // If wrapping in the X direction is needed, copy the leftmost column to the rightmost
            if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapX) {
                for y in 0..=fractal_height as usize {
                    self.fractal_array[fractal_width as usize][y] = self.fractal_array[0][y];
                }
            } else if self.flags.contains(FractalFlags::Polar) {
                // Polar coordinate transformation, the rightmost and the leftmost column will be set to 0
                for y in 0..=fractal_height as usize {
                    self.fractal_array[0][y] = 0;
                    self.fractal_array[fractal_width as usize][y] = 0;
                }
            }

            // If crust construction is needed, perform the processing
            if self.flags.contains(FractalFlags::CenterRift) {
                if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapY) {
                    for x in 0..=fractal_width as usize {
                        for y in 0..=(fractal_height / 6) as usize {
                            let factor =
                                ((fractal_height / 12) as i32 - y as i32).unsigned_abs() + 1;
                            self.fractal_array[x][y] /= factor;
                            self.fractal_array[x][(fractal_height / 2) as usize + y] /= factor;
                        }
                    }
                }

                if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapX) {
                    for y in 0..=fractal_height as usize {
                        for x in 0..=(fractal_width / 6) as usize {
                            let factor =
                                ((fractal_width / 12) as i32 - x as i32).unsigned_abs() + 1;
                            self.fractal_array[x][y] /= factor;
                            self.fractal_array[(fractal_width / 2) as usize + x][y] /= factor;
                        }
                    }
                }
            }

            /********** the end of preprocess fractal_array[][] **********/

            // Use this value to exclude the vertices which have already get spots in the previous iter.
            // Generate a value with the lowest `iPass+1` bits set to 1 and the rest set to 0.
            let screen = (1 << (pass + 1)) - 1;
            // Use Diamond-Square algorithm to get spots
            // At first, We divide the fractal into a grid of smaller (2^pass) * (2^pass) squares,
            // Notice! it's different with original Diamond-Square algorithm:
            //      1. Diamond Step and Square Step are in the independent iter of each other in the original,
            //         in this code them are in the same iter.
            //      2. In the original Square Step will use the calculation result of the Diamond Step,
            //         in this code Square Step doesn't use the calculation result of the Diamond Step.
            for x in 0..((fractal_width >> pass) as usize
                + if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapX) {
                    0
                } else {
                    1
                })
            {
                for y in 0..((fractal_height >> pass) as usize
                    + if self.fractal_grid.wrap_flags.contains(WrapFlags::WrapY) {
                        0
                    } else {
                        1
                    })
                {
                    // Interpolate
                    let mut sum = 0;
                    let randness = 1 << (7 - smooth + pass) as i32;
                    // `(x << pass) & screen != 0` is equivalent to `(x << pass) % (1 << (pass + 1)) != 0`
                    // `(y << pass) & screen != 0` is equivalent to `(y << pass) % (1 << (pass +1)) != 0`
                    match ((x << pass) & screen != 0, (y << pass) & screen != 0) {
                        (true, true) => {
                            // (center)
                            sum += self.fractal_array[(x - 1) << pass][(y - 1) << pass] as i32;
                            sum += self.fractal_array[(x + 1) << pass][(y - 1) << pass] as i32;
                            sum += self.fractal_array[(x - 1) << pass][(y + 1) << pass] as i32;
                            sum += self.fractal_array[(x + 1) << pass][(y + 1) << pass] as i32;
                            sum >>= 2;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[x << pass][y << pass] = sum as u32;
                        }
                        (true, false) => {
                            // (horizontal)
                            sum += self.fractal_array[(x - 1) << pass][y << pass] as i32;
                            sum += self.fractal_array[(x + 1) << pass][y << pass] as i32;
                            sum >>= 1;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[x << pass][y << pass] = sum as u32;
                        }
                        (false, true) => {
                            // (vertical)
                            sum += self.fractal_array[x << pass][(y - 1) << pass] as i32;
                            sum += self.fractal_array[x << pass][(y + 1) << pass] as i32;
                            sum >>= 1;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[x << pass][y << pass] = sum as u32;
                        }
                        _ => {
                            // (corner) This was already set in the previous iter.
                        }
                    }
                }
            }
        }

        if let Some(rifts) = rifts {
            debug_assert!(
                rifts.width_exp == self.width_exp,
                "Rifts width_exp does not match Fractal width_exp"
            );
            debug_assert!(
                rifts.height_exp == self.height_exp,
                "Rifts height_exp does not match Fractal height_exp"
            );

            self.tectonic_action(rifts);
        }

        if self.flags.contains(FractalFlags::InvertHeights) {
            self.fractal_array
                .iter_mut()
                .flatten()
                .for_each(|val| *val = 255 - *val);
        }
    }

    pub fn get_height(&self, x: i32, y: i32) -> u32 {
        debug_assert!(
            0 <= x && x < self.map_size.width as i32,
            "'x' is out of the range of the grid width"
        );
        debug_assert!(
            0 <= y && y < self.map_size.height as i32,
            "'y' is out of the range of the grid height"
        );

        let fractal_width = self.fractal_grid.size.width;
        let fractal_height = self.fractal_grid.size.height;
        let width_ratio = fractal_width as f64 / self.map_size.width as f64;
        let height_ratio = fractal_height as f64 / self.map_size.height as f64;

        // Use bilinear interpolation to calculate the pixel value
        let src_x = (x as f64 + 0.5) * width_ratio - 0.5;
        let src_y = (y as f64 + 0.5) * height_ratio - 0.5;

        let x_diff = src_x - src_x.floor();
        let y_diff = src_y - src_y.floor();

        let src_x = min(src_x as usize, fractal_width as usize - 1);
        let src_y = min(src_y as usize, fractal_height as usize - 1);

        let value = (1.0 - x_diff) * (1.0 - y_diff) * self.fractal_array[src_x][src_y] as f64
            + x_diff * (1.0 - y_diff) * self.fractal_array[src_x + 1][src_y] as f64
            + (1.0 - x_diff) * y_diff * self.fractal_array[src_x][src_y + 1] as f64
            + x_diff * y_diff * self.fractal_array[src_x + 1][src_y + 1] as f64;

        let height = value.clamp(0.0, 255.0) as u32;

        if self.flags.contains(FractalFlags::Percent) {
            (height * 100) >> 8
        } else {
            height
        }
    }

    /// Get a vector containing the calculated height values based on the given percentages for a fractal array.
    ///
    /// It takes an array of percentages. Each percentage value is clamped between 0 and 100.
    ///
    /// The function then extracts all values from the fractal array except its last row and last column,
    /// flattens the array, sorts it in an unstable manner, and calculates target values based on the input percentages.
    ///
    /// The final output is an array containing the calculated height values corresponding to the input percentages.
    pub fn get_height_from_percents<const N: usize>(&self, percents: [u32; N]) -> [u32; N] {
        let percents = percents.map(|p| p.clamp(0, 100));

        // Get all value from the fractal array except its last row and last column
        let mut flatten: Vec<&u32> = self
            .fractal_array
            .iter()
            .take(self.fractal_array.len() - 1)
            .flat_map(|row| row.iter().take(row.len() - 1))
            .collect();
        flatten.sort_unstable();

        let len = flatten.len();
        percents.map(|percent| {
            let target_index = ((len - 1) * percent as usize) / 100;
            let target_value = flatten[target_index];
            *target_value
        })
    }

    fn tectonic_action(&mut self, rifts: &CvFractal) {
        let fractal_width = self.fractal_grid.size.width;
        let fractal_height = self.fractal_grid.size.height;

        // `deep` is the maximum depth of the rift, which is in [0..=255].
        // The deepest point is typically in the middle of the rift.
        const DEEP: u32 = 0;

        // These values when fractal is WrapX
        let rift_x = (fractal_width / 4) * 3;
        // `width` is the distance from the leftmost/rightmost to the middle of the rift.
        // The width of the rift equals [2 * width].
        const WIDTH: u32 = 16;

        // These values when fractal is WrapY
        let rift_y = (fractal_height / 4) * 3;
        // `height` is the distance from the topmost/bottommost to the middle of the rift.
        // The height of the rift equals [2 * height].
        const HEIGHT: u32 = 16;

        if self.fractal_grid.wrap_x() {
            for y in 0..=fractal_height {
                // `rift_value` is the horizontal center x-coordinate of the rift.
                // It's in [-fractal_width / 8, fractal_width / 8 - fractal_width / 1024] range.
                let rift_value = (rifts.fractal_array[rift_x as usize][y as usize] as i32 - 128)
                    * fractal_width as i32
                    / 128
                    / 8;
                for x in 0..WIDTH {
                    //  Rift along edge of map.
                    let right = self
                        .fractal_grid
                        .normalize_offset(OffsetCoordinate::new(rift_value + x as i32, y as i32))
                        .unwrap();
                    let left = self
                        .fractal_grid
                        .normalize_offset(OffsetCoordinate::new(rift_value - x as i32, y as i32))
                        .unwrap();

                    self.fractal_array[right.0.x as usize][y as usize] =
                        (self.fractal_array[right.0.x as usize][y as usize] * x
                            + DEEP * (WIDTH - x))
                            / WIDTH;
                    self.fractal_array[left.0.x as usize][y as usize] =
                        (self.fractal_array[left.0.x as usize][y as usize] * x
                            + DEEP * (WIDTH - x))
                            / WIDTH;
                }
            }

            for y in 0..=fractal_height as usize {
                self.fractal_array[fractal_width as usize][y] = self.fractal_array[0][y];
            }
        }

        if self.fractal_grid.wrap_y() {
            for x in 0..=fractal_width {
                // `rift_value` is the height of the rift at the given y coordinate
                // It's in [-fractal_height / 8, fractal_height / 8 - fractal_height / 1024] range.
                let rift_value = (rifts.fractal_array[x as usize][rift_y as usize] as i32 - 128)
                    * fractal_height as i32
                    / 128
                    / 8;
                for y in 0..HEIGHT {
                    let top = self
                        .fractal_grid
                        .normalize_offset(OffsetCoordinate::new(x as i32, rift_value + y as i32))
                        .unwrap();
                    let bottom = self
                        .fractal_grid
                        .normalize_offset(OffsetCoordinate::new(x as i32, rift_value - y as i32))
                        .unwrap();

                    self.fractal_array[x as usize][top.0.y as usize] =
                        (self.fractal_array[x as usize][top.0.y as usize] * y
                            + DEEP * (HEIGHT - y))
                            / HEIGHT;
                    self.fractal_array[x as usize][bottom.0.y as usize] =
                        (self.fractal_array[x as usize][top.0.y as usize] * y
                            + DEEP * (HEIGHT - y))
                            / HEIGHT;
                }
            }

            for x in 0..=fractal_width as usize {
                self.fractal_array[x][fractal_height as usize] = self.fractal_array[x][0];
            }
        }
    }

    pub fn ridge_builder(
        &mut self,
        random: &mut StdRng,
        num_voronoi_seeds: u32,
        ridge_flags: FractalFlags,
        blend_ridge: u32,
        blend_fract: u32,
    ) {
        let fractal_width = self.fractal_grid.size.width;
        let fractal_height = self.fractal_grid.size.height;
        // this will use a modified Voronoi system to give the appearance of mountain ranges

        let num_voronoi_seeds = max(num_voronoi_seeds, 3); // make sure that we have at least 3

        let mut voronoi_seeds: Vec<VoronoiSeed> = Vec::with_capacity(num_voronoi_seeds as usize);

        for _ in 0..num_voronoi_seeds {
            let mut voronoi_seed;

            loop {
                voronoi_seed = VoronoiSeed::gen_random_seed(random, self.fractal_grid);

                // Check if the new random seed is too close to an existing seed
                let is_too_close = voronoi_seeds.iter().any(|existing_seed| {
                    let distance_between_voronoi_seeds = self
                        .fractal_grid
                        .distance_to(voronoi_seed.cell, existing_seed.cell);
                    distance_between_voronoi_seeds < 7
                });

                // If it's not too close, break the loop
                if !is_too_close {
                    break;
                }
            }

            voronoi_seeds.push(voronoi_seed);
        }

        for x in 0..fractal_width as usize {
            for y in 0..fractal_height as usize {
                // get the hex coordinate for this position
                let current_offset_coordinate = OffsetCoordinate::new(x as i32, y as i32);
                let current_cell = self
                    .fractal_grid
                    .offset_to_cell(current_offset_coordinate)
                    .unwrap();

                // find the distance to each of the seeds (with modifiers for strength of the seed, directional bias, and random factors)
                // closest seed distance is the distance to the seed with the lowest distance
                let mut closest_seed_distance = i32::MAX;
                // next closest seed distance is the distance to the seed with the second lowest distance
                let mut next_closest_seed_distance = i32::MAX;
                for current_voronoi_seed in &voronoi_seeds {
                    let mut modified_distance = self
                        .fractal_grid
                        .distance_to(current_cell, current_voronoi_seed.cell);
                    // Checking if ridge_flags is not empty
                    // If it is empty, we don't need to modify the distance
                    // If it is not empty, we need to modify the distance
                    // The distance is modified by the weakness of the seed, the bias direction, and the directional bias strength
                    // The weakness of the seed is used to make the influence of the seed on its surrounding area more random
                    // The bias direction is used to make the influence of the seed more directional
                    // The directional bias strength is used to make the influence of the seed more directional
                    if !ridge_flags.is_empty() {
                        // make the influence of the seed on its surrounding area more random
                        modified_distance += current_voronoi_seed.weakness as i32;

                        let relative_direction = self
                            .fractal_grid
                            .estimate_direction(current_cell, current_voronoi_seed.cell);

                        // make the influence of the seed more directional
                        if relative_direction == Some(current_voronoi_seed.bias_direction) {
                            modified_distance -=
                                current_voronoi_seed.directional_bias_strength as i32;
                        } else if relative_direction
                            == Some(current_voronoi_seed.bias_direction.opposite())
                        {
                            modified_distance +=
                                current_voronoi_seed.directional_bias_strength as i32;
                        }

                        modified_distance = max(1, modified_distance);
                    }

                    if modified_distance < closest_seed_distance {
                        next_closest_seed_distance = closest_seed_distance;
                        closest_seed_distance = modified_distance;
                    } else if modified_distance < next_closest_seed_distance {
                        next_closest_seed_distance = modified_distance;
                    }
                }

                // use the modified distance between the two closest seeds to determine the ridge height
                let ridge_height =
                    (255 * closest_seed_distance as u32) / next_closest_seed_distance as u32;

                // blend the new ridge height with the previous fractal height
                self.fractal_array[x][y] = (ridge_height * blend_ridge
                    + self.fractal_array[x][y] * blend_fract)
                    / max(blend_ridge + blend_fract, 1);
            }
        }
    }

    /// Get the noise map of the 2d Array which is used in the civ map. The map is saved as a gray image.
    pub fn write_to_file(&self, path: &Path) {
        let width = self.map_size.width;
        let height = self.map_size.height;
        let pixels: Vec<u8> = (0..height)
            .flat_map(|y| (0..width).map(move |x| self.get_height(x as i32, y as i32) as u8))
            .collect();

        let _ = image::save_buffer(
            Path::new(&path),
            &pixels,
            width,
            height,
            image::ColorType::L8,
        );
    }

    /// Get the noise map of the 2d Array which is used in the civ map. The map is saved as a gray image.
    ///
    /// The function is same as [`CvFractal::write_to_file`], but it uses the image crate to resize the image.
    pub fn write_to_file_by_image(&self, path: &Path) {
        let map_width = self.map_size.width;
        let map_height = self.map_size.height;
        // get gray_image from `self.fractal_array`
        let fractal_width = self.fractal_grid.size.width;
        let fractal_height = self.fractal_grid.size.height;

        // NOTICE: `pixels` doesn't contain the last row and column of the `fractal_array`
        let pixels: Vec<u8> = (0..fractal_height as usize)
            .flat_map(|y| (0..fractal_width as usize).map(move |x| self.fractal_array[x][y] as u8))
            .collect();

        let image: GrayImage =
            ImageBuffer::from_raw(fractal_width, fractal_height, pixels).unwrap();

        // get resized_image
        let resized_image = resize(&image, map_width, map_height, FilterType::Triangle);
        resized_image.save(path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use glam::Vec2;
    use rand::{rngs::StdRng, SeedableRng};

    use crate::grid::{
        hex_grid::{
            hex::{HexLayout, HexOrientation, Offset},
            HexGrid,
        },
        Size, WrapFlags,
    };

    use super::{CvFractal, FractalFlags};

    #[test]
    fn create_fractal_image() {
        let filename = "fractal.png";

        let path = Path::new(filename);

        let mut random = StdRng::seed_from_u64(77777777);

        let map_size = Size::new(1024, 512);
        let grid_size = map_size;

        let grid = HexGrid {
            size: grid_size,
            layout: HexLayout {
                orientation: HexOrientation::Flat,
                size: Vec2::new(8., 8.),
                origin: Vec2::new(0., 0.),
            },
            wrap_flags: WrapFlags::WrapX,
            offset: Offset::Odd,
        };

        let flags = FractalFlags::Polar;
        let fractal = CvFractal::create(&mut random, grid, 2, flags, 8, 7);

        fractal.write_to_file(path);
    }
}
