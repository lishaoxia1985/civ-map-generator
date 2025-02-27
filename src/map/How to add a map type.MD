# How to add a map type

If you want to add a new map type to the map, you need to do the following:

1. Add a new file to the `src/map` directory with the name of the map type. For example, if you want to add a new map type called `InlandSea`, you should create a file named `inland_sea.rs`. 
2. In the new file, define a struct named `InlandSea`. For example:
```rust
pub struct InlandSea(TileMap);

impl InlandSea {
    pub fn new(map_parameters: &MapParameters) -> Self {
        Self(TileMap::new(map_parameters))
    }
}
```
3. Implement the `Generator` trait for the new map type. This trait defines the behavior of the map, such as how to generate the map, how to place the player, ..., For example:
```rust
impl Generator for PangaeaMap {
    // This must be implemented
    fn into_inner(self) -> TileMap {
        self.0
    }

    // This must be implemented
    fn tile_map_mut(&mut self) -> &mut TileMap {
        &mut self.0
    }

    // If you don't want to use default implementation, you can implement this for custom map type
    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        let tile_map = self.tile_map_mut();
        // Perform necessary processing on the `tile_map` variable to generate terrain types for the custom map type
        // ...
    }

    // If you don't want to use default implementation, you can implement more methods for custom map type
    // ...
}
```
4. Add the new map type to the `MapType` enum in the `src/tile_map/map_parameters.rs` file. For example:
```rust
pub enum MapType {
    Fractal,
    Pangaea,
    InlandSea, // Add this line
}
```
5. Add code to the `generate_map` function in the `src/lib.rs` file to handle the new map type. For example:
```rust
pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_type {
       MapType::Fractal => generate_map_from_type!(map_parameters, ruleset, Fractal),
        MapType::Pangaea => generate_map_from_type!(map_parameters, ruleset, Pangaea),
        MapType::InlandSea => generate_map_from_type!(map_parameters, ruleset, InlandSea), // Add this line
    }
}
```
