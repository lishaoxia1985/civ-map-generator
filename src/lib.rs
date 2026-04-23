//! # Civilization Map Generator
//!
//! A map generation library for civilization-style strategy games.
//! The algorithm is primarily based on *Civilization V* with references from *Civilization VI*.
//!
//! ## Features
//!
//! - **Dual Hex Orientation**: Supports both flat and pointy hex orientations
//! - **Multiple Map Types**: Fractal and Pangaea generation algorithms
//! - **Complete Game Elements**: Terrain, resources, rivers, natural wonders, civilizations, city-states
//! - **Data-Driven Configuration**: JSON-based ruleset system
//!
//! ## Quick Start
//!
//! ```rust
//! use civ_map_generator::{generate_map, map_parameters::{MapParametersBuilder, WorldGrid}, ruleset::Ruleset};
//!
//! // Create default world grid
//! let world_grid = WorldGrid::default();
//!
//! // Build map parameters with custom settings
//! let map_parameters = MapParametersBuilder::new(world_grid)
//!     .seed(42)  // Optional: set seed for reproducible maps
//!     .build();
//!
//! // Load game rules
//! let ruleset = Ruleset::default();
//!
//! // Generate the map
//! let map = generate_map(&map_parameters, &ruleset);
//! ```
//!
//! ## Adding Custom Map Types
//!
//! See [How to add a map type](./src/map_generator/How%20to%20add%20a%20map%20type.MD) for implementation guide.
//!
//! ## Complete Example
//!
//! For a full-featured example, see [Civilization-Remastered](https://github.com/lishaoxia1985/Civilization-Remastered).
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - **`grid`**: Hexagonal and square grid systems with coordinate transformations
//! - **`map_generator`**: Map generation algorithms (Fractal, Pangaea)
//! - **`ruleset`**: Game rule definitions loaded from JSON files
//! - **`tile_map`**: Map data structure and generation pipeline
//! - **`tile_component`**: Tile components (terrain, features, resources, etc.)
//!
//! ## Current Limitations
//!
//! - Only fractal and pangaea map algorithms are implemented
//! - Square grid is not yet supported
//! - Some map parameters are hardcoded; JSON ruleset integration is partial
//!
//! ## References
//!
//! - [Unciv](https://github.com/yairm210/Unciv)
//! - [Community Patch for Civilization V](https://github.com/LoneGazebo/Community-Patch-DLL)
//! - [Red Blob Games - Hexagonal Grids](https://www.redblobgames.com/grids/hexagons/)

////////////////////////////////////////////////////////////////////////////////

pub mod fractal;
pub mod grid;
pub mod map_generator;
pub mod map_parameters;
pub mod nation;
pub mod ruleset;
pub mod tile;
pub mod tile_component;
pub mod tile_map;

// Re-export commonly used items for convenience
pub use map_generator::Generator;
pub use map_parameters::{MapParameters, MapParametersBuilder};
pub use ruleset::Ruleset;
pub use tile_map::TileMap;

use map_generator::{fractal::Fractal, pangaea::Pangaea};
use map_parameters::MapType;

/// Generates a map based on the provided parameters and ruleset.
///
/// # Arguments
///
/// * `map_parameters` - Configuration parameters for map generation
/// * `ruleset` - Game rules and definitions
///
/// # Returns
///
/// A fully generated [`TileMap`] with terrain, resources, civilizations, and other game elements.
///
/// # Examples
///
/// ```
/// use civ_map_generator::{generate_map, map_parameters::{MapParametersBuilder, WorldGrid}, ruleset::Ruleset};
///
/// let world_grid = WorldGrid::default();
/// let map_parameters = MapParametersBuilder::new(world_grid).build();
/// let ruleset = Ruleset::default();
/// let map = generate_map(&map_parameters, &ruleset);
/// ```
pub fn generate_map(map_parameters: &MapParameters, ruleset: &Ruleset) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters, ruleset),
        MapType::Pangaea => Pangaea::generate(map_parameters, ruleset),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generate_map,
        map_parameters::{MapParametersBuilder, WorldGrid},
        ruleset::Ruleset,
    };

    /// Tests for consistent map generation output when provided with the same random seed.
    #[test]
    fn test_generate_map_deterministic() {
        let world_grid = WorldGrid::default();
        let map_parameters = MapParametersBuilder::new(world_grid).seed(12345).build();
        let ruleset = Ruleset::default();

        for _ in 0..15 {
            let map_a = generate_map(&map_parameters, &ruleset);
            let map_b = generate_map(&map_parameters, &ruleset);
            assert_eq!(map_a, map_b, "Maps should be identical with same seed");
        }
    }

    /// Tests that different seeds produce different maps.
    #[test]
    fn test_different_seeds_produce_different_maps() {
        let world_grid = WorldGrid::default();
        let ruleset = Ruleset::default();

        let map_a = generate_map(
            &MapParametersBuilder::new(world_grid).seed(111).build(),
            &ruleset,
        );
        let map_b = generate_map(
            &MapParametersBuilder::new(world_grid).seed(222).build(),
            &ruleset,
        );

        assert_ne!(
            map_a, map_b,
            "Different seeds should produce different maps"
        );
    }
}
