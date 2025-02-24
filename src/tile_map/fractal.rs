use std::{
    array,
    cmp::{max, min},
    path::Path,
};

use glam::DVec2;
use image::{imageops::resize, GrayImage, ImageBuffer};
use rand::{rngs::StdRng, seq::SliceRandom, Rng};

use crate::grid::{
    hex::{Hex, HexLayout, HexOrientation, Offset, SQRT_3},
    Direction, OffsetCoordinate,
};

use super::{MapWrapping, WrapType};

struct VoronoiSeed {
    /// The hex coordinate of the seed
    pub hex_coordinate: Hex,
    /// The weakness value implies the influence of the seed on its surrounding area
    pub weakness: i32,
    /// The bias direction indicates the preferred direction or bias when assigning points within its influence region during the generation of the diagram.
    pub bias_direction: Direction,
    /// The strength of the bias direction.
    pub directional_bias_strength: i32,
}

impl VoronoiSeed {
    /// Generates a random seed for the fractal
    pub fn gen_random_seed(
        random: &mut StdRng,
        fractal_width: i32,
        fractal_height: i32,
        offset: Offset,
        orientation: HexOrientation,
    ) -> Self {
        let offset_coordinate = OffsetCoordinate::new(
            random.gen_range(0..fractal_width),
            random.gen_range(0..fractal_height),
        );
        let hex_coordinate = offset_coordinate.to_hex(offset, orientation);

        let weakness = random.gen_range(0..6);

        let hex_edge_direction = orientation.edge_direction();
        let bias_direction = *hex_edge_direction.choose(random).unwrap();

        let directional_bias_strength = random.gen_range(0..4);

        VoronoiSeed {
            hex_coordinate,
            weakness,
            bias_direction,
            directional_bias_strength,
        }
    }
}

pub struct CvFractal {
    /// The width of the 2D map
    map_width: i32,
    /// The height of the 2D map
    map_height: i32,
    /// It determines the type of the fractal, for example: wrap x, wrap y...
    flags: Flags,
    /// It is an exponent related to the width of the source fractal,
    /// `width_exp = 7` means the width of the source fractal is `2^7`
    width_exp: i32,
    /// It is an exponent related to the height of the source fractal,
    /// `height_exp = 7` means the height of the source fractal is `2^7`
    height_exp: i32,
    /// Stores the 2D fractal array, the array size is `[fractal_width + 1][fractal_height + 1]`
    /// # Notice
    /// The last column and last row are not part of the fractal, they are used to calculate the fractal values.
    fractal_array: Vec<Vec<i32>>,
    /// Width resolution of the fractal, is a power of 2. It equals `1 << width_exp`
    fractal_width: i32,
    /// Height resolution of the fractal, is a power of 2. It equals `1 << height_exp`
    fractal_height: i32,
    /// It represents the ratio between the width of the fractal (`fractal_width`) and the width of the 2D array (`map_width`).\
    /// It is used to calculate the source x position based on the given `x` coordinate.
    width_ratio: f64,
    /// It represents the ratio between the height of the fractal (`fractal_height`) and the height of the 2D array (`map_height`).\
    /// Similar to width_ratio, it is used to calculate the source y position based on the given `y` coordinate.
    height_ratio: f64,
}

#[derive(PartialEq, Eq)]
pub struct Flags {
    /// Determines the type of the wrapping, for example: wrap x, wrap y...
    pub map_wrapping: MapWrapping,
    /// When it's `false` the value of the height is in `0..=255`, otherwise the value is in `0..=99`
    pub percent: bool,
    /// Draws rift in center of world    
    pub center_rift: bool,
    /// Draws inverts the heights, the value of the invert height equals to `255 - the value of the original height`
    pub invert_heights: bool,
}

impl Default for Flags {
    /// Default values of ridge_flags are false
    fn default() -> Self {
        Self {
            map_wrapping: MapWrapping {
                x: WrapType::None,
                y: WrapType::None,
            },
            percent: false,
            center_rift: false,
            invert_heights: false,
        }
    }
}

impl CvFractal {
    const DEFAULT_WIDTH_EXP: i32 = 7;
    const DEFAULT_HEIGHT_EXP: i32 = 6;

    /// Creates a new empty fractal with the given parameters.
    fn new(map_width: i32, map_height: i32, flags: Flags, width_exp: i32, height_exp: i32) -> Self {
        let width_exp = if width_exp < 0 {
            Self::DEFAULT_WIDTH_EXP
        } else {
            width_exp
        };
        let height_exp = if height_exp < 0 {
            Self::DEFAULT_HEIGHT_EXP
        } else {
            height_exp
        };

        let fractal_width = 1 << width_exp;
        let fractal_height = 1 << height_exp;

        let fractal_array =
            vec![vec![0; (fractal_height + 1) as usize]; (fractal_width + 1) as usize];

        let width_ratio = fractal_width as f64 / map_width as f64;
        let height_ratio = fractal_height as f64 / map_height as f64;

        Self {
            fractal_array,
            map_width,
            map_height,
            flags,
            width_exp,
            height_exp,
            fractal_width,
            fractal_height,
            width_ratio,
            height_ratio,
        }
    }

    /// # Arguments
    ///
    /// * `random` - A mutable reference to a random number generator.
    /// * `grain` - Controls the level of detail or smoothness of the fractal. A higher value leads to more rougher,
    ///   while a lower value produces more smoother.
    ///         - When `grain = min(width_exp, height_exp)`, the fractal is completely random.
    ///         - When `grain = 0`, the fractal is the smoothest.
    ///
    /// # Panics
    ///
    /// Panics when `min(width_exp, height_exp) - grain >= 8`
    ///
    pub fn create(
        random: &mut StdRng,
        map_width: i32,
        map_height: i32,
        grain: i32,
        flags: Flags,
        width_exp: i32,
        height_exp: i32,
    ) -> Self {
        let mut fractal = Self::new(map_width, map_height, flags, width_exp, height_exp);
        fractal.frac_init_internal(grain, random, None, None);
        fractal
    }

    pub fn create_rifts(
        random: &mut StdRng,
        map_width: i32,
        map_height: i32,
        grain: i32,
        flags: Flags,
        rifts: &CvFractal,
        width_exp: i32,
        height_exp: i32,
    ) -> Self {
        let mut fractal = Self::new(map_width, map_height, flags, width_exp, height_exp);
        fractal.frac_init_internal(grain, random, None, Some(rifts));
        fractal
    }

    /// # Arguments
    ///
    /// * `grain` - Controls the level of detail or smoothness of the fractal. A higher value leads to more rougher,
    ///   while a lower value produces more smoother.
    /// * hint_array - At first the fractal is divided into a grid of small squares,
    ///   the points where adjacent grid lines meet are called **the vertices of the grid**,
    ///   we assign an initial value to each vertex by `hint_array` for later use in the diamond-square algorithm.
    ///   hint_array is usually generated by grayscale image.
    ///
    /// # Panics
    ///
    /// Panics when `min(`[CvFractal::width_exp]`, `[CvFractal::height_exp]`) - grain >= 8`
    ///
    fn frac_init_internal(
        &mut self,
        grain: i32,
        random: &mut StdRng,
        hint_array: Option<Vec<Vec<i32>>>,
        rifts: Option<&CvFractal>,
    ) {
        let min_exp = min(self.width_exp, self.height_exp);
        let smooth = (min_exp - grain).clamp(0, min_exp);

        assert!(
            smooth < 8,
            "'min(self.width_exp, self.height_exp) - grain < 8' should be true!"
        );
        // At first, We should divide the fractal into a grid of small (2^smooth) * (2^smooth) squares,
        // When the fractal is divided into a grid of small squares, the points where adjacent grid lines meet are called `the vertices of the grid`(abbreviated as `Vertices`).
        // `hint_width` is the num of `Vertices` in every row after dividing.
        // Notice: when the fractal is WrapX, we don't consider the last row (row index: `self.fractal_width`),
        //      because the last row of the fractal is the same as the first row,
        //      We preprocess this case at the beginning of every iter stage in Diamond-Square algorithm.
        let hint_width = (self.fractal_width >> smooth)
            + if self.flags.map_wrapping.x == WrapType::Wrap {
                0
            } else {
                1
            };
        // `hint_height` is the num of `Vertices` in every column after dividing.
        // Notice: when the fractal is WrapY, we don't consider the last column (column index: `self.fractal_height`),
        //      because the last column of the fractal is the same as the first column,
        //      We preprocess this case at the beginning of every iter in Diamond-Square algorithm.
        let hint_height = (self.fractal_height >> smooth)
            + if self.flags.map_wrapping.y == WrapType::Wrap {
                0
            } else {
                1
            };

        // The `hint_array` is the array of the vertices of the grid, every element is in [0..=255],
        // The array size is `[hint_width][hint_width]`
        if let Some(hint_array) = hint_array {
            assert!(
                hint_array.len() == hint_width as usize
                    && hint_array[0].len() == hint_height as usize,
                "Invalid hints array size."
            );
            // Assign an initial value to each vertex by `hint_array` for later use in the diamond-square algorithm.
            for x in 0..hint_width {
                for y in 0..hint_height {
                    self.fractal_array[(x << smooth) as usize][(y << smooth) as usize] =
                        hint_array[x as usize][y as usize];
                }
            }
        } else {
            // Assign an initial value to each vertex by random number generator for later use in the diamond-square algorithm.
            for x in 0..hint_width {
                for y in 0..hint_height {
                    self.fractal_array[(x << smooth) as usize][(y << smooth) as usize] =
                        random.gen_range(0..256); // Fractal Gen 1
                }
            }
        }

        for pass in (0..smooth).rev() {
            /*********** start to preprocess fractal_array[][] at the beginning of every iter stage in Diamond-Square algorithm. ***********/

            // If wrapping in the Y direction is needed, copy the bottom row to the top
            match self.flags.map_wrapping.y {
                WrapType::Wrap => {
                    for x in 0..=self.fractal_width {
                        self.fractal_array[x as usize][self.fractal_height as usize] =
                            self.fractal_array[x as usize][0];
                    }
                }
                WrapType::Polar => {
                    // Polar coordinate transformation, the top and bottom row will be set to 0
                    for x in 0..=self.fractal_width {
                        self.fractal_array[x as usize][0] = 0;
                        self.fractal_array[x as usize][self.fractal_height as usize] = 0;
                    }
                }
                WrapType::None => {}
            }

            // If wrapping in the X direction is needed, copy the leftmost column to the rightmost
            match self.flags.map_wrapping.x {
                WrapType::Wrap => {
                    for y in 0..=self.fractal_height {
                        self.fractal_array[self.fractal_width as usize][y as usize] =
                            self.fractal_array[0][y as usize];
                    }
                }
                WrapType::Polar => {
                    // Polar coordinate transformation, the rightmost and the leftmost column will be set to 0
                    for y in 0..=self.fractal_height {
                        self.fractal_array[0][y as usize] = 0;
                        self.fractal_array[self.fractal_width as usize][y as usize] = 0;
                    }
                }
                WrapType::None => {}
            }

            // If crust construction is needed, perform the processing
            if self.flags.center_rift {
                if self.flags.map_wrapping.y == WrapType::Wrap {
                    for x in 0..=self.fractal_width {
                        for y in 0..=(self.fractal_height / 6) {
                            let factor = ((self.fractal_height / 12) - y).abs() + 1;
                            self.fractal_array[x as usize][y as usize] /= factor;
                            self.fractal_array[x as usize]
                                [((self.fractal_height / 2) + y) as usize] /= factor;
                        }
                    }
                }

                if self.flags.map_wrapping.x == WrapType::Wrap {
                    for y in 0..=self.fractal_height {
                        for x in 0..=(self.fractal_width / 6) {
                            let factor = ((self.fractal_width / 12) - x).abs() + 1;
                            self.fractal_array[x as usize][y as usize] /= factor;
                            self.fractal_array[((self.fractal_width / 2) + x) as usize]
                                [y as usize] /= factor;
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
            for x in 0..((self.fractal_width >> pass)
                + if self.flags.map_wrapping.x == WrapType::Wrap {
                    0
                } else {
                    1
                })
            {
                for y in 0..((self.fractal_height >> pass)
                    + if self.flags.map_wrapping.y == WrapType::Wrap {
                        0
                    } else {
                        1
                    })
                {
                    // Interpolate
                    let mut sum = 0;
                    let randness = 1 << (7 - smooth + pass);
                    // `(x << pass) & screen != 0` is equivalent to `(x << pass) % (1 << (pass + 1)) != 0`
                    // `(y << pass) & screen != 0` is equivalent to `(y << pass) % (1 << (pass +1)) != 0`
                    match ((x << pass) & screen != 0, (y << pass) & screen != 0) {
                        (true, true) => {
                            // (center)
                            sum += self.fractal_array[((x - 1) << pass) as usize]
                                [((y - 1) << pass) as usize];
                            sum += self.fractal_array[((x + 1) << pass) as usize]
                                [((y - 1) << pass) as usize];
                            sum += self.fractal_array[((x - 1) << pass) as usize]
                                [((y + 1) << pass) as usize];
                            sum += self.fractal_array[((x + 1) << pass) as usize]
                                [((y + 1) << pass) as usize];
                            sum >>= 2;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[(x << pass) as usize][(y << pass) as usize] = sum;
                        }
                        (true, false) => {
                            // (horizontal)
                            sum += self.fractal_array[((x - 1) << pass) as usize]
                                [(y << pass) as usize];
                            sum += self.fractal_array[((x + 1) << pass) as usize]
                                [(y << pass) as usize];
                            sum >>= 1;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[(x << pass) as usize][(y << pass) as usize] = sum;
                        }
                        (false, true) => {
                            // (vertical)
                            sum += self.fractal_array[(x << pass) as usize]
                                [((y - 1) << pass) as usize];
                            sum += self.fractal_array[(x << pass) as usize]
                                [((y + 1) << pass) as usize];
                            sum >>= 1;
                            sum += random.gen_range(-randness..randness);
                            sum = sum.clamp(0, 255);
                            self.fractal_array[(x << pass) as usize][(y << pass) as usize] = sum;
                        }
                        _ => {
                            // (corner) This was already set in the previous iter.
                        }
                    }
                }
            }
        }

        if let Some(rifts) = rifts {
            self.tectonic_action(rifts); //  Assumes FRAC_WRAP_X is on.
        }

        if self.flags.invert_heights {
            self.fractal_array
                .iter_mut()
                .flatten()
                .for_each(|val| *val = 255 - *val);
        }
    }

    pub fn get_height(&self, x: i32, y: i32) -> i32 {
        assert!(0 <= x && x < self.map_width, "'x' is out of range");
        assert!(0 <= y && y < self.map_height, "'y' is out of range");

        // Use bilinear interpolation to calculate the pixel value
        let src_x = (x as f64 + 0.5) * self.width_ratio - 0.5;
        let src_y = (y as f64 + 0.5) * self.height_ratio - 0.5;

        let x_diff = src_x - src_x.floor();
        let y_diff = src_y - src_y.floor();

        let src_x = min(src_x as usize, self.fractal_width as usize - 1);
        let src_y = min(src_y as usize, self.fractal_height as usize - 1);

        let value = (1.0 - x_diff) * (1.0 - y_diff) * self.fractal_array[src_x][src_y] as f64
            + x_diff * (1.0 - y_diff) * self.fractal_array[src_x + 1][src_y] as f64
            + (1.0 - x_diff) * y_diff * self.fractal_array[src_x][src_y + 1] as f64
            + x_diff * y_diff * self.fractal_array[src_x + 1][src_y + 1] as f64;

        let height = value.clamp(0.0, 255.0) as i32;

        if self.flags.percent {
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
    /// The final output is a vector containing the calculated height values corresponding to the input percentages.
    pub fn get_height_from_percents(&self, percents: &[i32]) -> Vec<i32> {
        let percents: Vec<i32> = percents
            .iter()
            .map(|&percent| percent.clamp(0, 100))
            .collect();
        // Get all value from the fractal array except its last row and last column
        let mut flatten: Vec<&i32> = self
            .fractal_array
            .iter()
            .take(self.fractal_array.len() - 1)
            .flat_map(|row| row.iter().take(row.len() - 1))
            .collect();
        flatten.sort_unstable();

        let len = flatten.len();
        percents
            .iter()
            .map(|&percent| {
                let target_index = ((len - 1) * percent as usize) / 100;
                let target_value = flatten[target_index];

                *target_value
            })
            .collect()
    }

    fn tectonic_action(&mut self, rifts: &CvFractal) {
        //  Assumes FRAC_WRAP_X is on.
        let rift_x = (self.fractal_width / 4) * 3;
        // `width` is the distance from the leftmost/rightmost to the middle of the rift.
        // The width of the rift equals [2 * width].
        let width = 16;
        // `deep` is the maximum depth of the rift, which is in [0..=255].
        // The deepest point is typically in the middle of the rift.
        let deep = 0;

        for y in 0..=self.fractal_height {
            let rift_value = (rifts.fractal_array[rift_x as usize][y as usize] - 128)
                * self.fractal_width
                / 128
                / 8;
            for x in 0..width {
                //  Rift along edge of map.
                let right_x = self.yield_x(rift_value, x);
                let left_x = self.yield_x(rift_value, -x);

                self.fractal_array[right_x as usize][y as usize] =
                    (self.fractal_array[right_x as usize][y as usize] * x + deep * (width - x))
                        / width;
                self.fractal_array[left_x as usize][y as usize] =
                    (self.fractal_array[left_x as usize][y as usize] * x + deep * (width - x))
                        / width;
            }
        }

        for y in 0..=self.fractal_height {
            self.fractal_array[self.fractal_width as usize][y as usize] =
                self.fractal_array[0][y as usize];
        }
    }

    /// In a Wrap X map, given the coordinates of a point and an offset representing the direction of movement, calculate the new coordinates of the point after it moves accordingly.
    fn yield_x(&self, x: i32, offset_x: i32) -> i32 {
        let width = self.fractal_width;
        // Calculate the new coordinates without wrapping
        let nx = x + offset_x;
        // Wrap the coordinates and return the wrapped coordinates
        nx.rem_euclid(width)
    }

    pub fn ridge_builder(
        &mut self,
        random: &mut StdRng,
        num_voronoi_seeds: i32,
        ridge_flags: &Flags,
        blend_ridge: i32,
        blend_fract: i32,
        orientation: HexOrientation,
        offset: Offset,
    ) {
        // this will use a modified Voronoi system to give the appearance of mountain ranges

        let num_voronoi_seeds = max(num_voronoi_seeds, 3); // make sure that we have at least 3

        let mut voronoi_seeds: Vec<VoronoiSeed> = Vec::with_capacity(num_voronoi_seeds as usize);

        for _ in 0..num_voronoi_seeds {
            let mut voronoi_seed = VoronoiSeed::gen_random_seed(
                random,
                self.fractal_width,
                self.fractal_height,
                offset,
                orientation,
            );

            // Check if the new random seed is too close to an existing seed
            // If it is, generate a new random seed until it is not too close
            while voronoi_seeds.iter().any(|existing_seed| {
                let distance_between_voronoi_seeds =
                    Hex::hex_distance(voronoi_seed.hex_coordinate, existing_seed.hex_coordinate);
                distance_between_voronoi_seeds < 7
            }) {
                let offset_coordinate = OffsetCoordinate::new(
                    random.gen_range(0..self.fractal_width),
                    random.gen_range(0..self.fractal_height),
                );
                let hex_coordinate = offset_coordinate.to_hex(offset, orientation);
                voronoi_seed.hex_coordinate = hex_coordinate;
            }

            voronoi_seeds.push(voronoi_seed);
        }

        for x in 0..self.fractal_width {
            for y in 0..self.fractal_height {
                // get the hex coordinate for this position
                let offset_coordinate = OffsetCoordinate::new(x, y);
                let current_hex = offset_coordinate.to_hex(offset, orientation);

                // find the distance to each of the seeds (with modifiers for strength of the seed, directional bias, and random factors)
                // closest seed distance is the distance to the seed with the lowest distance
                let mut closest_seed_distance = i32::MAX;
                // next closest seed distance is the distance to the seed with the second lowest distance
                let mut next_closest_seed_distance = i32::MAX;
                for current_voronoi_seed in &voronoi_seeds {
                    let mut modified_hex_distance =
                        Hex::hex_distance(current_hex, current_voronoi_seed.hex_coordinate);
                    // Checking if all values of ridge_flags are false by comparing it to the default value of Flags.
                    if ridge_flags != &Flags::default() {
                        // make the influence of the seed on its surrounding area more random
                        modified_hex_distance += current_voronoi_seed.weakness;

                        let relative_direction = self.estimate_direction(
                            current_hex,
                            current_voronoi_seed.hex_coordinate,
                            orientation,
                        );

                        // make the influence of the seed more directional
                        if relative_direction == current_voronoi_seed.bias_direction {
                            modified_hex_distance -= current_voronoi_seed.directional_bias_strength;
                        } else if relative_direction
                            == current_voronoi_seed.bias_direction.opposite_direction()
                        {
                            modified_hex_distance += current_voronoi_seed.directional_bias_strength;
                        }

                        modified_hex_distance = max(1, modified_hex_distance);
                    }

                    if modified_hex_distance < closest_seed_distance {
                        next_closest_seed_distance = closest_seed_distance;
                        closest_seed_distance = modified_hex_distance;
                    } else if modified_hex_distance < next_closest_seed_distance {
                        next_closest_seed_distance = modified_hex_distance;
                    }
                }

                // use the modified distance between the two closest seeds to determine the ridge height
                let ridge_height = (255 * closest_seed_distance) / next_closest_seed_distance;

                // blend the new ridge height with the previous fractal height
                self.fractal_array[x as usize][y as usize] = (ridge_height * blend_ridge
                    + self.fractal_array[x as usize][y as usize] * blend_fract)
                    / max(blend_ridge + blend_fract, 1);
            }
        }
    }

    /// Determine the direction of dest relative to start.
    ///
    /// If dest is located to the north of start, the function returns [Direction::North].
    fn estimate_direction(&self, start: Hex, dest: Hex, orientation: HexOrientation) -> Direction {
        // If the start and dest are the same, return Direction::None
        if start == dest {
            return Direction::None;
        }

        // Define hex_layout and set the size to 1/sqrt(3) to make sure we can get the unit vectors
        // A unit vector refers to a vector whose norm is 1.
        let hex_layout = HexLayout {
            orientation,
            size: DVec2::new(1. / SQRT_3, 1. / SQRT_3),
            origin: DVec2::new(0., 0.),
        };

        // `direction_vectors` contains all the unit vectors that indicate direction.
        let direction_vectors: [DVec2; 6] =
            array::from_fn(|i| hex_layout.hex_to_pixel(Hex::HEX_DIRECTIONS[i]));

        let mut estimate_vector = (dest - start).into_inner();

        // If the map is wrapping, adjust the estimate vector accordingly.
        // The distance from the dest to the start's left may be shorter than the distance from the dest to the start's right.
        // So we make sure the distance from the dest to the start is always shortest.
        if self.flags.map_wrapping.x == WrapType::Wrap {
            if estimate_vector.x > self.fractal_width / 2 {
                estimate_vector.x -= self.fractal_width;
            } else if estimate_vector.x < -self.fractal_width / 2 {
                estimate_vector.x += self.fractal_width;
            }
        }

        // The distance from the dest to the start's top may be shorter than the distance from the dest to the start's bottom.
        // So we make sure the distance from the dest to the start is always shortest.
        if self.flags.map_wrapping.y == WrapType::Wrap {
            if estimate_vector.y > self.fractal_height / 2 {
                estimate_vector.y -= self.fractal_height;
            } else if estimate_vector.y < -self.fractal_height / 2 {
                estimate_vector.y += self.fractal_height;
            }
        }

        let estimate_vector = estimate_vector.as_dvec2();

        // Find the index of the direction vector with the largest dot product with the estimate vector.
        let max_index = direction_vectors
            .into_iter()
            .enumerate()
            .map(|(index, direction_vector)| (index, estimate_vector.dot(direction_vector)))
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b))
            .map(|(index, _)| index)
            .unwrap();

        orientation.edge_direction()[max_index]
    }

    /// Get the noise map of the 2d Array which is used in the civ map. The map is saved as a gray image.
    pub fn write_to_file(&self, path: &Path) {
        let width = self.map_width;
        let height = self.map_height;
        let mut pixels = vec![];
        for y in 0..height {
            for x in 0..width {
                pixels.push(self.get_height(x, y) as u8);
            }
        }

        let _ = image::save_buffer(
            Path::new(&path),
            &pixels,
            width as u32,
            height as u32,
            image::ColorType::L8,
        );
    }

    /// Get the noise map of the 2d Array which is used in the civ map. The map is saved as a gray image.
    ///
    /// The function is same as [`CvFractal::write_to_file`], but it uses the crate function of the image crate.
    pub fn write_to_file_by_image(&self, path: &Path) {
        // get gray_image from `self.fractal_array`
        let width = self.fractal_width as usize;
        let height = self.fractal_height as usize;
        let mut pixels = vec![0; width * height];
        for y in 0..height {
            for x in 0..width {
                pixels[y * width + x] = self.fractal_array[x][y] as u8;
            }
        }
        let image: GrayImage = ImageBuffer::from_raw(width as u32, height as u32, pixels).unwrap();

        // get resized_image
        let resized_image = resize(
            &image,
            self.map_width as u32,
            self.map_height as u32,
            image::imageops::FilterType::Triangle,
        );
        resized_image.save(path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rand::{rngs::StdRng, SeedableRng};

    use crate::tile_map::{fractal::Flags, MapWrapping, WrapType};

    use super::CvFractal;

    #[test]
    fn create_fractal_image() {
        let filename = "fractal.png";

        let path = Path::new(filename);

        let mut random = StdRng::seed_from_u64(77777777);
        let flags = Flags {
            map_wrapping: MapWrapping {
                x: WrapType::Wrap,
                y: WrapType::Polar,
            },
            ..Default::default()
        };
        let fractal = CvFractal::create(&mut random, 1024, 512, 2, flags, 8, 7);

        fractal.write_to_file(path);
    }
}
