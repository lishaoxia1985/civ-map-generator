use crate::{
    tile::Tile,
    tile_component::{BaseTerrain, Feature, Resource, TerrainType},
    tile_map::{Layer, TileMap},
};

mod place_bonus_resources;
mod place_luxury_resources;
mod place_strategic_resources;

pub(crate) use place_bonus_resources::*;
pub(crate) use place_luxury_resources::*;
pub(crate) use place_strategic_resources::*;
use rand::{
    Rng, RngExt,
    distr::{Distribution, weighted::WeightedIndex},
};

impl TileMap {
    // function AssignStartingPlots:ProcessResourceList
    /// Placing bonus or strategic resources on the map based on the given parameters.
    ///
    /// It iterates through the list of tiles and places resources on eligible tiles based on the
    /// resource type, quantity, and radius.\
    /// Before using this function, make sure `tile_list` has been shuffled.
    ///
    /// # Arguments
    ///
    /// - `frequency`: The frequency of resource placement.\
    ///   It determines resource placement such that one resource is placed per every 'frequency' tiles, with at least one resource guaranteed even if there are fewer than 'frequency' tiles.
    ///   For example, a frequency of 3 means that one resource is placed every 3 tiles, with at least one resource guaranteed.
    /// - `layer`: The layer on which the resource will be placed.
    /// - `tile_list`: A vector of tiles representing the tiles where resources can be placed. Before using this argument, make sure the vector has been shuffled.
    /// - `resource_list_to_place`: A vector of resource to place, which contains the resource type,
    ///   quantity, minimum radius, and maximum radius for each resource.
    ///
    /// # Panics
    ///
    /// This function will panic if the layer is not [`Layer::Bonus`] or [`Layer::Strategic`]. That means if you use this function to place luxury resources, it will panic.
    ///
    /// # Notes
    ///
    /// Although in the original CIV5, this function has some code about placing luxury resources, but in fact, it is never used to place luxury resources. So, we forbid placing luxury resources in this function.
    /// If you want to place luxury resources, please use [`TileMap::place_specific_number_of_resources`].
    fn process_resource_list(
        &mut self,
        frequency: u32,
        layer: Layer,
        tile_list: &[Tile],
        resource_list_to_place: &[ResourceToPlace],
    ) {
        debug_assert!(layer == Layer::Bonus || layer == Layer::Strategic,
            "`process_resource_list` can only be used to place bonus or strategic resources, not luxury resources.
            If you want to place luxury resources, please use `place_specific_number_of_resources` instead."
        );

        if tile_list.is_empty() {
            return;
        }

        let resource_weight = resource_list_to_place
            .iter()
            .map(|resource| resource.weight)
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(resource_weight).unwrap();

        let num_resources_to_place = (tile_list.len() as u32).div_ceil(frequency);

        let mut tile_list_iter = tile_list.iter();

        // Main loop
        for _ in 0..num_resources_to_place {
            let current_resource_to_place =
                &resource_list_to_place[dist.sample(&mut self.random_number_generator)];
            let resource = current_resource_to_place.resource;
            let quantity = current_resource_to_place.quantity;
            let min_radius = current_resource_to_place.min_radius;
            let max_radius = current_resource_to_place.max_radius;
            let radius = self
                .random_number_generator
                .random_range(min_radius..=max_radius);

            // First pass: Seek the first eligible 0 value on impact matrix
            if let Some(&tile) = tile_list_iter.find(|tile| {
                self.layer_data[layer][tile.index()] == 0 && tile.resource(self).is_none()
            }) {
                tile.set_resource(self, resource, quantity);
                self.place_impact_and_ripples(tile, layer, radius);
                continue;
            }

            // Completed first pass of tile_list, now change to seeking lowest value instead of zero value.
            // If no eligible 0 value is found, second pass: Seek the lowest value (value < 98) on the impact matrix
            if let Some(&tile) = tile_list
                .iter()
                .filter(|tile| {
                    self.layer_data[layer][tile.index()] < 98 && tile.resource(self).is_none()
                })
                .min_by_key(|tile| self.layer_data[layer][tile.index()])
            {
                tile.set_resource(self, resource, quantity);
                self.place_impact_and_ripples(tile, layer, radius);
            }
        }
    }

    // AssignStartingPlots:GenerateLuxuryPlotListsAtCitySite
    /// Generate the candidate tile lists for placing luxury or strategic resources within the specified radius around a city site, excluding the city site itself.
    ///
    /// # Arguments
    ///
    /// - `city_site`: The tile representing the city site. This is the center of the radius.
    /// - `radius`: The radius within which to generate candidate tiles.
    ///   For example, if `radius` is 2, the function will consider tiles within a distance of 2 tiles from the city site, excluding the city site itself.
    ///   In original CIV5 code, the max radius which city site can extend is 5. So `radius` should be in `[1, 5]`.
    ///
    /// # Returns
    ///
    /// - `[Vec<Tile>; 15]`: An array of vectors of tiles, where each inner vector represents a list of candidate tiles matching a specific criteria.
    ///
    /// # Notes
    ///
    /// In the original code, `clear ice near city site` and `generate luxury or strategic tile lists at city site` are combined in one method.
    /// We have extracted the `clear ice near city site` into a separate method.
    /// If you want to clear ice near city site, you should use [`TileMap::clear_ice_near_city_site`].\
    pub fn generate_luxury_or_strategic_tile_lists_at_city_site(
        &self,
        city_site: Tile,
        radius: u32,
    ) -> [Vec<Tile>; 15] {
        let grid = self.world_grid.grid;

        let mut region_coast_tile_list = Vec::new();
        let mut region_hill_open_tile_list = Vec::new();
        let mut region_hill_jungle_tile_list = Vec::new();
        let mut region_hill_forest_tile_list = Vec::new();
        let mut region_hill_covered_tile_list = Vec::new();
        let mut region_tundra_flat_including_forest_tile_list = Vec::new();
        let mut region_forest_flat_but_not_tundra_tile_list = Vec::new();
        let mut region_desert_flat_no_feature_tile_list = Vec::new();
        let mut region_plain_flat_no_feature_tile_list = Vec::new();
        let mut region_fresh_water_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_dry_grass_flat_no_feature_tile_list = Vec::new();
        let mut region_forest_flat_tile_list = Vec::new();
        let mut region_marsh_tile_list = Vec::new();
        let mut region_flood_plain_tile_list = Vec::new();
        let mut region_jungle_flat_tile_list = Vec::new();

        // In original CIV5 code, the max radius which city site can extend is 5.
        // So we only consider the tiles within the radius of 5 from the city site.
        if radius > 0 && radius < 6 {
            for ripple_radius in 1..=radius {
                city_site
                    .tiles_at_distance(ripple_radius, grid)
                    .for_each(|tile_at_distance| {
                        let terrain_type = tile_at_distance.terrain_type(self);
                        let base_terrain = tile_at_distance.base_terrain(self);
                        let feature = tile_at_distance.feature(self);

                        match terrain_type {
                            TerrainType::Water => {
                                if base_terrain == BaseTerrain::Coast
                                    && feature != Some(Feature::Ice)
                                    && feature != Some(Feature::Atoll)
                                {
                                    region_coast_tile_list.push(tile_at_distance);
                                }
                            }
                            TerrainType::Flatland => {
                                if let Some(feature) = feature {
                                    match feature {
                                        Feature::Forest => {
                                            region_forest_flat_tile_list.push(tile_at_distance);
                                            if base_terrain == BaseTerrain::Tundra {
                                                region_tundra_flat_including_forest_tile_list
                                                    .push(tile_at_distance);
                                            } else {
                                                region_forest_flat_but_not_tundra_tile_list
                                                    .push(tile_at_distance);
                                            }
                                        }
                                        Feature::Jungle => {
                                            region_jungle_flat_tile_list.push(tile_at_distance);
                                        }
                                        Feature::Marsh => {
                                            region_marsh_tile_list.push(tile_at_distance);
                                        }
                                        Feature::Floodplain => {
                                            region_flood_plain_tile_list.push(tile_at_distance);
                                        }
                                        _ => {}
                                    }
                                } else {
                                    match base_terrain {
                                        BaseTerrain::Grassland => {
                                            if tile_at_distance.is_freshwater(self) {
                                                region_fresh_water_grass_flat_no_feature_tile_list
                                                    .push(tile_at_distance);
                                            } else {
                                                region_dry_grass_flat_no_feature_tile_list
                                                    .push(tile_at_distance);
                                            }
                                        }
                                        BaseTerrain::Desert => {
                                            region_desert_flat_no_feature_tile_list
                                                .push(tile_at_distance);
                                        }
                                        BaseTerrain::Plain => {
                                            region_plain_flat_no_feature_tile_list
                                                .push(tile_at_distance);
                                        }
                                        BaseTerrain::Tundra => {
                                            region_tundra_flat_including_forest_tile_list
                                                .push(tile_at_distance);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            TerrainType::Mountain => {}
                            TerrainType::Hill => {
                                if base_terrain != BaseTerrain::Snow {
                                    if feature.is_none() {
                                        region_hill_open_tile_list.push(tile_at_distance);
                                    } else if feature == Some(Feature::Forest) {
                                        region_hill_forest_tile_list.push(tile_at_distance);
                                        region_hill_covered_tile_list.push(tile_at_distance);
                                    } else if feature == Some(Feature::Jungle) {
                                        region_hill_jungle_tile_list.push(tile_at_distance);
                                        region_hill_covered_tile_list.push(tile_at_distance);
                                    }
                                }
                            }
                        }
                    });
            }
        }

        [
            region_coast_tile_list,
            region_marsh_tile_list,
            region_flood_plain_tile_list,
            region_hill_open_tile_list,
            region_hill_covered_tile_list,
            region_hill_jungle_tile_list,
            region_hill_forest_tile_list,
            region_jungle_flat_tile_list,
            region_forest_flat_tile_list,
            region_desert_flat_no_feature_tile_list,
            region_plain_flat_no_feature_tile_list,
            region_dry_grass_flat_no_feature_tile_list,
            region_fresh_water_grass_flat_no_feature_tile_list,
            region_tundra_flat_including_forest_tile_list,
            region_forest_flat_but_not_tundra_tile_list,
        ]
    }
}

struct ResourceToPlace {
    /// The resource will be placed on the tile.
    pub resource: Resource,
    /// The number of the resource will be placed on one tile.
    pub quantity: u32,
    /// Determine the probability of placing the resource on a tile.
    pub weight: u32,
    /// Related to `resource_impact` when we place resources on tiles.
    pub min_radius: u32,
    /// Related to `resource_impact` when we place resources on tiles.
    pub max_radius: u32,
}
