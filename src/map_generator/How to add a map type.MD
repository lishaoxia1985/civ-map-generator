# Adding a New Map Type

To add a new map type, follow these steps:

## 1. Create Map Type File

Create a new file in `src/map_generator/` with the map type name. For example, for an `InlandSea` map type, create `inland_sea.rs`.

## 2. Define the Struct

Define a struct for the new map type:

```rust
pub struct InlandSea(TileMap);
```

## 3. Implement the Generator Trait

Implement the `Generator` trait for the new map type:

```rust
impl Generator for InlandSea {
    // Required: Use macros to generate common methods
    generate_common_methods!();

    // Optional: Override default implementation for custom behavior
    fn generate_terrain_types(&mut self, map_parameters: &MapParameters) {
        let tile_map = self.tile_map_mut();
        // Custom terrain generation logic
        // ...
    }

    // Optional: Override additional methods as needed
    // ...
}
```

## 4. Update MapType Enum

Add the new map type to the `MapType` enum in `src/map_parameters.rs`:

```rust
pub enum MapType {
    Fractal,
    Pangaea,
    InlandSea, // Add this line
}
```

## 5. Register in generate_map Function

Add handling for the new map type in the `generate_map` function in `src/lib.rs`:

```rust
pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters, ruleset),
        MapType::Pangaea => Pangaea::generate(map_parameters, ruleset),
        MapType::InlandSea => InlandSea::generate(map_parameters, ruleset), // Add this line
    }
}
```