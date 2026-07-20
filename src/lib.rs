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
//! ```rust,ignore
//! use civ_map_generator::{generate_map, map_parameters::{MapParametersBuilder, WorldGrid}};
//!
//! // Create default world grid
//! let world_grid = WorldGrid::default();
//!
//! // Build map parameters with custom settings
//! let map_parameters = MapParametersBuilder::new(world_grid)
//!     .seed(42)  // Optional: set seed for reproducible maps
//!     .build();
//!
//! // Generate the map
//! let map = generate_map(&map_parameters);
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
use crate::{map_generator::Generator, map_parameters::MapParameters, tile_map::TileMap};
use map_generator::{fractal::Fractal, pangaea::Pangaea};
use map_parameters::MapType;

pub mod fractal;
pub mod grid;
pub mod map_generator;
pub mod map_parameters;
pub mod ruleset;
pub mod tile;
pub mod tile_map;

/// Generates a map based on the provided parameters and ruleset.
///
/// # Arguments
///
/// * `map_parameters` - Configuration parameters for map generation.
///
/// # Returns
///
/// A fully generated [`TileMap`] with terrain, resources, civilizations, and other game elements.
///
/// # Examples
///
/// ```rust,ignore
/// use civ_map_generator::{generate_map, map_parameters::{MapParametersBuilder, WorldGrid}};
///
/// let world_grid = WorldGrid::default();
/// let map_parameters = MapParametersBuilder::new(world_grid).build();
/// let map = generate_map(&map_parameters);
/// ```
pub fn generate_map(map_parameters: &MapParameters) -> TileMap {
    match map_parameters.map_type {
        MapType::Fractal => Fractal::generate(map_parameters),
        MapType::Pangaea => Pangaea::generate(map_parameters),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generate_map,
        map_parameters::{MapParametersBuilder, WorldGrid},
    };

    /// Tests for consistent map generation output when provided with the same random seed.
    #[test]
    fn test_generate_map_deterministic() {
        let world_grid = WorldGrid::default();
        let map_parameters = MapParametersBuilder::new(world_grid).seed(12345).build();

        for _ in 0..10 {
            let map_a = generate_map(&map_parameters);
            let map_b = generate_map(&map_parameters);
            assert_eq!(map_a, map_b, "Maps should be identical with same seed");
        }
    }
}
