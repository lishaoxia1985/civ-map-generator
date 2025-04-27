use super::Generator;
use crate::map_parameters::MapParameters;
use crate::{generate_common_methods, tile_map::TileMap};

pub struct Fractal(TileMap);

impl Generator for Fractal {
    generate_common_methods!();
}
