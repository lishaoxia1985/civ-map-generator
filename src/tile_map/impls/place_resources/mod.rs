use crate::{
    tile::Tile,
    tile_component::Resource,
    tile_map::{Layer, TileMap},
};

mod place_bonus_resources;
mod place_luxury_resources;
mod place_strategic_resources;

pub(crate) use place_bonus_resources::*;
pub(crate) use place_luxury_resources::*;
pub(crate) use place_strategic_resources::*;
use rand::{
    Rng,
    distr::{Distribution, weighted::WeightedIndex},
};

impl TileMap {
    // function AssignStartingPlots:ProcessResourceList
    /// Placing bonus or strategic resources on the map based on the given parameters.
    ///
    /// It iterates through the list of plots and places resources on eligible plots based on the
    /// resource type, quantity, and radius.\
    /// Before using this function, make sure `tile_list` has been shuffled.
    ///
    /// # Arguments
    ///
    /// - `frequency`: The frequency of resource placement.\
    ///   It determines resource placement such that one resource is placed per every 'frequency' tiles, with at least one resource guaranteed even if there are fewer than 'frequency' tiles.
    ///   For example, a frequency of 3 means that one resource is placed every 3 tiles, with at least one resource guaranteed.
    /// - `layer`: The layer on which the resource will be placed.
    /// - `tile_list`: A vector of tiles representing the plots where resources can be placed. Before using this argument, make sure the vector has been shuffled.
    /// - `resource_list_to_place`: A vector of resource to place, which contains the resource type,
    ///   quantity, minimum radius, and maximum radius for each resource.
    ///
    /// # Panics
    ///
    /// This function will panic if the layer is not [`Layer::Bonus`] or [`Layer::Strategic`]. That means if you use this function to place luxury resources, it will panic.
    ///
    /// # Notice
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
