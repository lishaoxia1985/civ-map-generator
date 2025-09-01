use rand::{seq::SliceRandom, Rng};

use crate::{
    map_parameters::{MapParameters, ResourceSetting},
    tile::Tile,
    tile_component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    tile_map::{Layer, TileMap},
};

use super::assign_starting_tile::{get_major_strategic_resource_quantity_values, ResourceToPlace};

impl TileMap {
    pub fn place_strategic_resources(&mut self, map_parameters: &MapParameters) {
        // Adjust amounts, if applicable, based on Resource Setting.
        let (uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt) =
            get_major_strategic_resource_quantity_values(map_parameters.resource_setting);

        // Adjust appearance rate per Resource Setting chosen by user.
        let bonus_multiplier = match map_parameters.resource_setting {
            // Sparse, so increase the number of tiles per bonus.
            ResourceSetting::Sparse => 1.5,
            // Abundant, so reduce the number of tiles per bonus.
            ResourceSetting::Abundant => 2.0 / 3.0,
            _ => 1.0,
        };

        let [coast_list, flatland_list, jungle_flat_list, forest_flat_list, marsh_list, snow_flat_list, dry_grass_flat_no_feature, plains_flat_no_feature, tundra_flat_no_feature, desert_flat_no_feature, hills_list] =
            self.generate_strategic_resource_tile_lists_in_map();

        // Place Strategic resources.
        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Oil,
                quantity: oil_amt,
                weight: 65,
                min_radius: 1,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: Resource::Uranium,
                quantity: uran_amt,
                weight: 35,
                min_radius: 0,
                max_radius: 1,
            },
        ];
        self.process_resource_list(9., Layer::Strategic, &marsh_list, &resources_to_place);

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Oil,
                quantity: oil_amt,
                weight: 40,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: Resource::Aluminum,
                quantity: alum_amt,
                weight: 15,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 45,
                min_radius: 1,
                max_radius: 2,
            },
        ];
        self.process_resource_list(
            16.,
            Layer::Strategic,
            &tundra_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Oil,
                quantity: oil_amt,
                weight: 60,
                min_radius: 1,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: Resource::Aluminum,
                quantity: alum_amt,
                weight: 15,
                min_radius: 2,
                max_radius: 3,
            },
            ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 25,
                min_radius: 2,
                max_radius: 3,
            },
        ];
        self.process_resource_list(17., Layer::Strategic, &snow_flat_list, &resources_to_place);

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Oil,
                quantity: oil_amt,
                weight: 65,
                min_radius: 0,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 35,
                min_radius: 1,
                max_radius: 1,
            },
        ];
        self.process_resource_list(
            13.,
            Layer::Strategic,
            &desert_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 26,
                min_radius: 0,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: Resource::Coal,
                quantity: coal_amt,
                weight: 35,
                min_radius: 1,
                max_radius: 3,
            },
            ResourceToPlace {
                resource: Resource::Aluminum,
                quantity: alum_amt,
                weight: 39,
                min_radius: 2,
                max_radius: 3,
            },
        ];
        self.process_resource_list(22., Layer::Strategic, &hills_list, &resources_to_place);

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Coal,
                quantity: coal_amt,
                weight: 30,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: Resource::Uranium,
                quantity: uran_amt,
                weight: 70,
                min_radius: 1,
                max_radius: 2,
            },
        ];
        self.process_resource_list(
            33.,
            Layer::Strategic,
            &jungle_flat_list,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: Resource::Coal,
                quantity: coal_amt,
                weight: 30,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: Resource::Uranium,
                quantity: uran_amt,
                weight: 70,
                min_radius: 1,
                max_radius: 1,
            },
        ];
        self.process_resource_list(
            39.,
            Layer::Strategic,
            &forest_flat_list,
            &resources_to_place,
        );

        let resources_to_place = [ResourceToPlace {
            resource: Resource::Horses,
            quantity: horse_amt,
            weight: 100,
            min_radius: 2,
            max_radius: 5,
        }];
        self.process_resource_list(
            33.,
            Layer::Strategic,
            &dry_grass_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [ResourceToPlace {
            resource: Resource::Horses,
            quantity: horse_amt,
            weight: 100,
            min_radius: 1,
            max_radius: 4,
        }];
        self.process_resource_list(
            33.,
            Layer::Strategic,
            &plains_flat_no_feature,
            &resources_to_place,
        );

        self.add_modern_minor_strategics_to_city_states(map_parameters);

        self.place_small_quantities_of_strategics(
            map_parameters,
            23. * bonus_multiplier,
            &flatland_list,
        );

        self.place_oil_in_the_sea(map_parameters, &coast_list);

        // Check for low or missing Strategic resources.
        // If there are very few resources, add one more.
        if self.placed_resource_count(Resource::Iron) < 8 {
            let resources_to_place = [ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Iron) < 4 * map_parameters.num_civilization {
            // print("Map has very low iron, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Iron,
                quantity: iron_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Horses) < 4 * map_parameters.num_civilization {
            // print("Map has very low horse, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Horses,
                quantity: horse_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &plains_flat_no_feature,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Horses) < 4 * map_parameters.num_civilization {
            // print("Map has very low horse, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Horses,
                quantity: horse_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &dry_grass_flat_no_feature,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Coal) < 8 {
            // print("Map has very low coal, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Coal,
                quantity: coal_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Coal) < 4 * map_parameters.num_civilization {
            // print("Map has very low coal, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Coal,
                quantity: coal_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Oil) < 4 * map_parameters.num_civilization {
            // print("Map has very low oil, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Oil,
                quantity: oil_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Aluminum) < 4 * map_parameters.num_civilization {
            // print("Map has very low aluminum, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Aluminum,
                quantity: alum_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count(Resource::Uranium) < 2 * map_parameters.num_civilization {
            // print("Map has very low uranium, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: Resource::Uranium,
                quantity: uran_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }
    }

    // function AssignStartingPlots:PlaceOilInTheSea
    /// Places oil sources in [`BaseTerrain::Coast`], with the amount being half of what is on land.
    /// If the map has too little ocean, it will place as much as can fit.
    /// Before calling this function, make sure `coast_list` is shuffled.
    ///
    /// # Notice
    ///
    /// This operation will invalidate the Strategic Resource Impact Table for future operations,
    /// so it should always be called last, even after minor resource placements.
    fn place_oil_in_the_sea(&mut self, map_parameters: &MapParameters, coast_list: &[Tile]) {
        // `resource_setting` is Abundant, increase amount.
        let sea_oil_amt = if let ResourceSetting::Abundant = map_parameters.resource_setting {
            6
        } else {
            4
        };
        let num_land_oil = self.placed_resource_count(Resource::Oil);

        let num_to_place = ((num_land_oil as f64 / 2.) / sea_oil_amt as f64) as u32;
        self.place_specific_number_of_resources(
            Resource::Oil,
            sea_oil_amt,
            num_to_place,
            0.2,
            Some(Layer::Strategic),
            4,
            7,
            coast_list,
        );
    }

    // function AssignStartingPlots:PlaceSmallQuantitiesOfStrategics
    /// Distributes small quantities of strategic resources.
    /// Before calling this function, make sure `tile_list` is shuffled.
    fn place_small_quantities_of_strategics(
        &mut self,
        map_parameters: &MapParameters,
        frequency: f64,
        tile_list: &[Tile],
    ) {
        if tile_list.is_empty() {
            return;
        }

        let [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt] =
            get_small_strategic_resource_quantity_values(map_parameters.resource_setting);

        let num_to_place = (tile_list.len() as f64 / frequency).ceil() as u32;

        let mut num_left_to_place = num_to_place;

        for &tile in tile_list.iter() {
            if num_left_to_place == 0 {
                break;
            }
            let terrain_type = tile.terrain_type(self);
            let base_terrain = tile.base_terrain(self);
            let feature = tile.feature(self);

            let mut selected_resource = None;
            let mut selected_quantity = 2;

            if self.layer_data[Layer::Strategic][tile.index()] == 0 && tile.resource(self).is_none()
            {
                // Placing a small strategic resource here. Need to determine what type to place.
                if let Some(feature) = feature {
                    match feature {
                        Feature::Forest => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            (selected_resource, selected_quantity) = match diceroll {
                                0 => (Some(Resource::Uranium), uran_amt),
                                1 => (Some(Resource::Coal), coal_amt),
                                _ => (Some(Resource::Iron), iron_amt),
                            };
                        }
                        Feature::Jungle => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            (selected_resource, selected_quantity) = match diceroll {
                                0 => {
                                    if terrain_type == TerrainType::Hill {
                                        (Some(Resource::Iron), iron_amt)
                                    } else {
                                        (Some(Resource::Oil), oil_amt)
                                    }
                                }
                                1 => (Some(Resource::Coal), coal_amt),
                                _ => (Some(Resource::Aluminum), alum_amt),
                            };
                        }
                        Feature::Marsh => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            (selected_resource, selected_quantity) = match diceroll {
                                0 => (Some(Resource::Iron), iron_amt),
                                1 => (Some(Resource::Coal), coal_amt),
                                _ => (Some(Resource::Oil), oil_amt),
                            };
                        }
                        _ => (),
                    }
                } else {
                    match terrain_type {
                        TerrainType::Flatland => match base_terrain {
                            BaseTerrain::Grassland => {
                                (selected_resource, selected_quantity) = if tile.is_freshwater(self)
                                {
                                    (Some(Resource::Horses), horse_amt)
                                } else {
                                    let diceroll = self.random_number_generator.gen_range(0..5);
                                    if diceroll < 3 {
                                        (Some(Resource::Iron), iron_amt)
                                    } else {
                                        (Some(Resource::Horses), horse_amt)
                                    }
                                };
                            }
                            BaseTerrain::Desert => {
                                let diceroll = self.random_number_generator.gen_range(0..3);
                                (selected_resource, selected_quantity) = match diceroll {
                                    0 => (Some(Resource::Iron), iron_amt),
                                    1 => (Some(Resource::Aluminum), alum_amt),
                                    _ => (Some(Resource::Oil), oil_amt),
                                };
                            }
                            BaseTerrain::Plain => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                (selected_resource, selected_quantity) = if diceroll < 2 {
                                    (Some(Resource::Iron), iron_amt)
                                } else {
                                    (Some(Resource::Horses), horse_amt)
                                };
                            }
                            _ => {
                                let diceroll = self.random_number_generator.gen_range(0..4);
                                (selected_resource, selected_quantity) = match diceroll {
                                    0 => (Some(Resource::Iron), iron_amt),
                                    1 => (Some(Resource::Uranium), uran_amt),
                                    _ => (Some(Resource::Oil), oil_amt),
                                };
                            }
                        },
                        TerrainType::Hill => match base_terrain {
                            BaseTerrain::Grassland | BaseTerrain::Plain => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                (selected_resource, selected_quantity) = match diceroll {
                                    2 => (Some(Resource::Horses), horse_amt),
                                    n if n < 2 => (Some(Resource::Iron), iron_amt),
                                    _ => (Some(Resource::Coal), coal_amt),
                                };
                            }
                            _ => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                (selected_resource, selected_quantity) = if diceroll < 2 {
                                    (Some(Resource::Iron), iron_amt)
                                } else {
                                    (Some(Resource::Coal), coal_amt)
                                };
                            }
                        },
                        _ => {
                            unreachable!()
                        }
                    }
                }

                if let Some(selected_resource) = selected_resource {
                    // Probability distribution for the possible values of `radius`: 0, 1, 2
                    //
                    // Probability of generating 0: 1/4
                    // Probability of generating 1: 2/4 (includes original 1 and 3 converted to 1)
                    // Probability of generating 2: 1/4
                    let mut radius = self.random_number_generator.gen_range(0..4);
                    if radius > 2 {
                        radius = 1;
                    }

                    self.resource_query[tile.index()] =
                        Some((selected_resource, selected_quantity));
                    self.place_impact_and_ripples(tile, Layer::Strategic, radius);
                    num_left_to_place -= 1;
                }
            }
        }
    }

    // function AssignStartingPlots:AddModernMinorStrategicsToCityStates
    /// Add modern minor strategics to city states.
    ///
    /// This function places small quantities of modern strategic resources (Oil, Aluminum, Coal) in city states.
    /// Mordern strategics contain Oil, Aluminum, Coal.
    fn add_modern_minor_strategics_to_city_states(&mut self, map_parameters: &MapParameters) {
        let [_uran_amt, _horse_amt, oil_amt, _iron_amt, coal_amt, alum_amt] =
            get_small_strategic_resource_quantity_values(map_parameters.resource_setting);
        let candidate_resources_amount = [coal_amt, oil_amt, alum_amt];

        const CANDIDATE_STRATEGIC_RESOURCES: [Resource; 3] =
            [Resource::Coal, Resource::Oil, Resource::Aluminum];
        const PRIORITY_LIST_INDICES_OF_STRATEGIC_RESOURCES: [[usize; 6]; 3] = [
            [3, 4, 13, 11, 10, 9],
            [9, 1, 13, 14, 11, 10],
            [3, 4, 13, 9, 10, 11],
        ];

        // Get starting tiles of city states.
        let starting_tiles = self
            .starting_tile_and_city_state
            .keys()
            .copied()
            .collect::<Vec<_>>();

        for starting_tile in starting_tiles.into_iter() {
            let chosen_resource_index = self.random_number_generator.gen_range(0..4);
            if chosen_resource_index < 3 {
                let strategic_resource = CANDIDATE_STRATEGIC_RESOURCES[chosen_resource_index];
                let resource_amount = candidate_resources_amount[chosen_resource_index];
                let priority_list_indices_of_chosen_resource =
                    PRIORITY_LIST_INDICES_OF_STRATEGIC_RESOURCES[chosen_resource_index];

                let mut luxury_plot_lists =
                    self.generate_luxury_tile_lists_at_city_site(starting_tile, 3);

                let mut num_left_to_place = resource_amount;

                for &i in priority_list_indices_of_chosen_resource.iter() {
                    if num_left_to_place == 0 {
                        break;
                    }
                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        strategic_resource,
                        num_left_to_place,
                        1,
                        1.0,
                        None,
                        0,
                        0,
                        &luxury_plot_lists[i],
                    );
                }
            }
        }
    }

    /// Calculates the total quantity of a specified resource
    ///
    /// This function iterates through all resource assignments, checks if the name of each resource
    /// matches the provided resource name, and if so, adds the resource quantity to the total sum.
    /// Finally, it returns the total quantity of the specified resource.
    ///
    /// # Arguments
    ///
    /// - `resource`: The name of the resource to look for (string type).
    ///
    /// # Returns
    ///
    /// Returns the total quantity of the specified resource as `u32`.
    pub fn placed_resource_count(&self, resource: Resource) -> u32 {
        self.resource_query
            .iter()
            .filter_map(|assignment_resource| assignment_resource.as_ref())
            .filter(|(r, _)| *r == resource)
            .map(|(_, q)| *q)
            .sum()
    }

    // AssignStartingPlots:GenerateGlobalResourcePlotLists
    /// Generate the candidate tile lists for placing strategic resources on the entire map.
    ///
    /// Each `Vec` is shuffled to ensure randomness.
    ///
    /// # Returns
    ///
    /// - `[Vec<Tile>; 11]`: An array of vectors of tiles, where each inner vector represents a list of candidate tiles matching a specific criteria.
    ///   Each `Vec` is shuffled to ensure randomness.
    fn generate_strategic_resource_tile_lists_in_map(&mut self) -> [Vec<Tile>; 11] {
        let mut coast_list = Vec::new();
        let mut flatland_list = Vec::new(); // very complex
        let mut jungle_flat_list = Vec::new();
        let mut forest_flat_list = Vec::new();
        let mut marsh_list = Vec::new();
        let mut snow_flat_list = Vec::new();
        let mut dry_grass_flat_no_feature = Vec::new();
        let mut plains_flat_no_feature = Vec::new();
        let mut tundra_flat_no_feature = Vec::new();
        let mut desert_flat_no_feature = Vec::new();
        let mut hills_list = Vec::new();

        self.all_tiles().for_each(|tile| {
            if !self.player_collision_data[tile.index()] && tile.resource(self).is_none() {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                let feature = tile.feature(self);

                match terrain_type {
                    TerrainType::Water => {
                        if base_terrain == BaseTerrain::Coast
                            && feature != Some(Feature::Ice)
                            && feature != Some(Feature::Atoll)
                        {
                            coast_list.push(tile);
                            /* if tile.neighbor_tiles(map_parameters).iter().any(
                                |neighbor_tile| {
                                    neighbor_tile.terrain_type(self) != TerrainType::Water
                                },
                            ) {
                                region_coast_next_to_land_tile_list.push(tile);
                            } */
                        }
                    }
                    TerrainType::Flatland => {
                        if feature.is_none_or(|f| matches!(f, Feature::Forest | Feature::Jungle)) {
                            flatland_list.push(tile);
                        }
                        if let Some(feature) = feature {
                            match feature {
                                Feature::Forest => {
                                    forest_flat_list.push(tile);
                                    if base_terrain == BaseTerrain::Tundra {
                                        /* region_tundra_flat_including_forest_tile_list
                                        .push(tile); */
                                    } else {
                                        /* region_forest_flat_but_not_tundra_tile_list
                                        .push(tile); */
                                    }
                                }
                                Feature::Jungle => {
                                    jungle_flat_list.push(tile);
                                }
                                Feature::Marsh => {
                                    marsh_list.push(tile);
                                }
                                Feature::Floodplain => {
                                    /* region_flood_plain_tile_list.push(tile); */
                                }
                                _ => {}
                            }
                        } else {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    if tile.is_freshwater(self) {
                                        /* region_fresh_water_grass_flat_no_feature_tile_list
                                        .push(tile); */
                                    } else {
                                        dry_grass_flat_no_feature.push(tile);
                                    }
                                }
                                BaseTerrain::Desert => {
                                    desert_flat_no_feature.push(tile);
                                }
                                BaseTerrain::Plain => {
                                    plains_flat_no_feature.push(tile);
                                }
                                BaseTerrain::Tundra => {
                                    tundra_flat_no_feature.push(tile);
                                }
                                BaseTerrain::Snow => {
                                    snow_flat_list.push(tile);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        }
                    }
                    TerrainType::Mountain => {}
                    TerrainType::Hill => {
                        if base_terrain != BaseTerrain::Snow {
                            hills_list.push(tile);
                            /* if feature == None {
                                region_hill_open_tile_list.push(tile);
                            } else if feature == Some(Feature::Forest) {
                                region_hill_forest_tile_list.push(tile);
                                region_hill_covered_tile_list.push(tile);
                            } else if feature == Some(Feature::Jungle) {
                                region_hill_jungle_tile_list.push(tile);
                                region_hill_covered_tile_list.push(tile);
                            } */
                        }
                    }
                }
            }
        });

        let mut lists = [
            coast_list,
            flatland_list,
            jungle_flat_list,
            forest_flat_list,
            marsh_list,
            snow_flat_list,
            dry_grass_flat_no_feature,
            plains_flat_no_feature,
            tundra_flat_no_feature,
            desert_flat_no_feature,
            hills_list,
        ];

        // Shuffle each list. This is done to ensure that the order in which resources are placed is random.
        lists.iter_mut().for_each(|list| {
            list.shuffle(&mut self.random_number_generator);
        });

        lists
    }
}

/// This function determines quantity per tile for each strategic resource's small deposit size.
fn get_small_strategic_resource_quantity_values(resource_setting: ResourceSetting) -> [u32; 6] {
    // According to resource_setting, calculate the number of resources to place.
    let [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt] = match resource_setting {
        ResourceSetting::Sparse => [1, 1, 2, 1, 2, 2], // Sparse
        ResourceSetting::Abundant => [3, 3, 3, 3, 3, 3], // Abundant
        _ => [2, 2, 3, 2, 3, 3],                       // Default
    };

    [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt]
}
