# Civ Map Generator

This is a civilization map generator. This algorithm is primarily based on the implementation in *Civilization V*, with some references from *Civilization VI*.

## How to add a map type

[How to add a map type](./src/map_generator/How%20to%20add%20a%20map%20type.MD)

## Example

[Civilization-Remastered](https://github.com/lishaoxia1985/Civilization-Remastered)

## Innovation Highlights

This project introduces several key innovations:

1. **Support both flat and pointy hex**  
   Original civilization implementation only supports pointy hex, Unciv implementation only supports flat hex, but this project supports both flat and pointy hex.

## Miss Features

1. **Only support to generate fractal and pangaea map**  
   This project only supports to generate fractal and pangaea map. we will add more map generation algorithm in the future.
2. **No support to square grid**  
   This project only supports hex grid. We will add support to square grid in the future.

## Future Plans

1. **Add more map generation algorithm**  
   We will add more map generation algorithm in the future.
2. **support to square grid**  
   We will add support to square grid in the future.
3. **Optimize the JSON file as ruleset information**  
   We will optimize the JSON file as ruleset information in the future. In folder `src/jsons`, only a litter files are used as ruleset information to generate map. And Some map parameters are hard-coded in the code. We will optimize it in the future.

## Reference project

 * [Unciv](https://github.com/yairm210/Unciv)  
 * [Community Patch for Civilization V - Brave New World](https://github.com/LoneGazebo/Community-Patch-DLL)  
 * [Red Blob Games](https://www.redblobgames.com/grids/hexagons/)

## License

Licensed under either of

 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
 * [MIT license](http://opensource.org/licenses/MIT)

at your option.

**Contributions**

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.