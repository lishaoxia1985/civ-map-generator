use super::Generator;
use crate::{generate_common_methods, map_parameters::MapParameters, tile_map::TileMap};

pub struct Fractal(TileMap);

impl Generator for Fractal {
    generate_common_methods!();
}
