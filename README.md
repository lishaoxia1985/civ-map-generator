# Civ Map Generator

A Civilization series game map generator library. The algorithm is primarily based on *Civilization V* implementation with some references from *Civilization VI*.

## Adding a Map Type

[How to add a map type](./src/map_generator/How%20to%20add%20a%20map%20type.MD)

## Example

```rust
use crate::{generate_map, map_parameters::{MapParametersBuilder, WorldGrid}, ruleset::Ruleset};

fn main() {
    let world_grid = WorldGrid::default();
    let map_parameters = MapParametersBuilder::new(world_grid).build();
    let ruleset = Ruleset::default();
    let map = generate_map(&map_parameters, &ruleset);
}
```

Complete example: [Civilization-Remastered](https://github.com/lishaoxia1985/Civilization-Remastered)

## Key Innovations

* **Dual Hex Orientation Support**  
  Supports both flat and pointy hex orientations. Original Civilization implementations typically support only one orientation, but this project supports both.

## Current Limitations

* **Limited Map Generation Algorithms**  
  Only fractal and pangaea maps are currently supported. More algorithms will be added in the future.

* **Hex Grid Only**  
  Square grid support is not yet implemented.

## Future Plans

* Add more map generation algorithms
* Add square grid support
* Optimize JSON-based ruleset information (currently, only a subset of files in `src/jsons` are used as ruleset information)

## References

* [Unciv](https://github.com/yairm210/Unciv)  
* [Community Patch for Civilization V - Brave New World](https://github.com/LoneGazebo/Community-Patch-DLL)  
* [Red Blob Games](https://www.redblobgames.com/grids/hexagons/)

## License

Licensed under either of

* [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
* [MIT license](http://opensource.org/licenses/MIT)

at your option.

### Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.