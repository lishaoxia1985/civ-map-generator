use crate::tile_map::{MapParameters, TileMap};

use super::Generator;

pub struct Fractal(TileMap);

impl Fractal {
    pub fn new(map_parameters: &MapParameters) -> Self {
        Self(TileMap::new(map_parameters))
    }
}

impl Generator for Fractal {
    fn into_inner(self) -> TileMap {
        self.0
    }
    fn tile_map_mut(&mut self) -> &mut TileMap {
        &mut self.0
    }
}
