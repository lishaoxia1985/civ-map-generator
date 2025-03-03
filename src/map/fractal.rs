use crate::{
    generate_common_methods,
    tile_map::{MapParameters, TileMap},
};

use super::Generator;

pub struct Fractal(TileMap);

impl Generator for Fractal {
    generate_common_methods!();
}
