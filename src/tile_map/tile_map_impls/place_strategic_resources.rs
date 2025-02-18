use rand::{seq::SliceRandom, Rng};

use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    tile_map::{tile::Tile, Layer, MapParameters, ResourceSetting, TileMap},
};

use super::assign_starting_tile::ResourceToPlace;

impl TileMap {
    pub fn place_strategic_resources(&mut self, map_parameters: &MapParameters) {
        // Adjust amounts, if applicable, based on Resource Setting.
        let (uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt) =
            Self::get_major_strategic_resource_quantity_values(map_parameters.resource_setting);

        // Adjust appearance rate per Resource Setting chosen by user.
        let bonus_multiplier = match map_parameters.resource_setting {
            // Sparse, so increase the number of tiles per bonus.
            ResourceSetting::Sparse => 1.5,
            // Abundant, so reduce the number of tiles per bonus.
            ResourceSetting::Abundant => 2.0 / 3.0,
            _ => 1.0,
        };

        let [coast_list, flatland_list, jungle_flat_list, forest_flat_list, marsh_list, snow_flat_list, dry_grass_flat_no_feature, plains_flat_no_feature, tundra_flat_no_feature, desert_flat_no_feature, hills_list] =
            self.generate_strategic_resource_plot_lists(map_parameters);

        // Place Strategic resources.
        let resources_to_place = [
            ResourceToPlace {
                resource: "Oil".to_string(),
                quantity: oil_amt,
                weight: 65,
                min_radius: 1,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: "Uranium".to_string(),
                quantity: uran_amt,
                weight: 35,
                min_radius: 0,
                max_radius: 1,
            },
        ];
        self.process_resource_list(
            map_parameters,
            9.,
            Layer::Strategic,
            &marsh_list,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Oil".to_string(),
                quantity: oil_amt,
                weight: 40,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: "Aluminum".to_string(),
                quantity: alum_amt,
                weight: 15,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 45,
                min_radius: 1,
                max_radius: 2,
            },
        ];
        self.process_resource_list(
            map_parameters,
            16.,
            Layer::Strategic,
            &tundra_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Oil".to_string(),
                quantity: oil_amt,
                weight: 60,
                min_radius: 1,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: "Aluminum".to_string(),
                quantity: alum_amt,
                weight: 15,
                min_radius: 2,
                max_radius: 3,
            },
            ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 25,
                min_radius: 2,
                max_radius: 3,
            },
        ];
        self.process_resource_list(
            map_parameters,
            17.,
            Layer::Strategic,
            &snow_flat_list,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Oil".to_string(),
                quantity: oil_amt,
                weight: 65,
                min_radius: 0,
                max_radius: 1,
            },
            ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 35,
                min_radius: 1,
                max_radius: 1,
            },
        ];
        self.process_resource_list(
            map_parameters,
            13.,
            Layer::Strategic,
            &desert_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 26,
                min_radius: 0,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: "Coal".to_string(),
                quantity: coal_amt,
                weight: 35,
                min_radius: 1,
                max_radius: 3,
            },
            ResourceToPlace {
                resource: "Aluminum".to_string(),
                quantity: alum_amt,
                weight: 39,
                min_radius: 2,
                max_radius: 3,
            },
        ];
        self.process_resource_list(
            map_parameters,
            22.,
            Layer::Strategic,
            &hills_list,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Coal".to_string(),
                quantity: coal_amt,
                weight: 30,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: "Uranium".to_string(),
                quantity: uran_amt,
                weight: 70,
                min_radius: 1,
                max_radius: 2,
            },
        ];
        self.process_resource_list(
            map_parameters,
            33.,
            Layer::Strategic,
            &jungle_flat_list,
            &resources_to_place,
        );

        let resources_to_place = [
            ResourceToPlace {
                resource: "Coal".to_string(),
                quantity: coal_amt,
                weight: 30,
                min_radius: 1,
                max_radius: 2,
            },
            ResourceToPlace {
                resource: "Uranium".to_string(),
                quantity: uran_amt,
                weight: 70,
                min_radius: 1,
                max_radius: 1,
            },
        ];
        self.process_resource_list(
            map_parameters,
            39.,
            Layer::Strategic,
            &forest_flat_list,
            &resources_to_place,
        );

        let resources_to_place = [ResourceToPlace {
            resource: "Horses".to_string(),
            quantity: horse_amt,
            weight: 100,
            min_radius: 2,
            max_radius: 5,
        }];
        self.process_resource_list(
            map_parameters,
            33.,
            Layer::Strategic,
            &dry_grass_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = [ResourceToPlace {
            resource: "Horses".to_string(),
            quantity: horse_amt,
            weight: 100,
            min_radius: 1,
            max_radius: 4,
        }];
        self.process_resource_list(
            map_parameters,
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
        if self.placed_resource_count("Iron") < 8 {
            let resources_to_place = [ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Iron") < 4 * map_parameters.civilization_num {
            // print("Map has very low iron, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Iron".to_string(),
                quantity: iron_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Horses") < 4 * map_parameters.civilization_num {
            // print("Map has very low horse, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Horses".to_string(),
                quantity: horse_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &plains_flat_no_feature,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Horses") < 4 * map_parameters.civilization_num {
            // print("Map has very low horse, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Horses".to_string(),
                quantity: horse_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &dry_grass_flat_no_feature,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Coal") < 8 {
            // print("Map has very low coal, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Coal".to_string(),
                quantity: coal_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Coal") < 4 * map_parameters.civilization_num {
            // print("Map has very low coal, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Coal".to_string(),
                quantity: coal_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Oil") < 4 * map_parameters.civilization_num {
            // print("Map has very low oil, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Oil".to_string(),
                quantity: oil_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Aluminum") < 4 * map_parameters.civilization_num {
            // print("Map has very low aluminum, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Aluminum".to_string(),
                quantity: alum_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &hills_list,
                &resources_to_place,
            );
        }

        if self.placed_resource_count("Uranium") < 2 * map_parameters.civilization_num {
            // print("Map has very low uranium, adding another.");
            let resources_to_place = vec![ResourceToPlace {
                resource: "Uranium".to_string(),
                quantity: uran_amt,
                weight: 100,
                min_radius: 0,
                max_radius: 0,
            }];
            self.process_resource_list(
                map_parameters,
                f64::MAX,
                Layer::Strategic,
                &flatland_list,
                &resources_to_place,
            );
        }
    }

    // function AssignStartingPlots:PlaceOilInTheSea
    /// Places oil sources in coastal waters, with the amount being half of what is on land.
    /// If the map has too little ocean, it will place as much as can fit.
    ///
    /// # Warning
    /// This operation will invalidate the Strategic Resource Impact Table for future operations,
    /// so it should always be called last, even after minor resource placements.
    fn place_oil_in_the_sea(&mut self, map_parameters: &MapParameters, coast_list: &[Tile]) {
        // `resource_setting` is Abundant, increase amount.
        let sea_oil_amt = if let ResourceSetting::Abundant = map_parameters.resource_setting {
            6
        } else {
            4
        };
        let num_land_oil = self.placed_resource_count("Oil");

        let num_to_place = ((num_land_oil as f64 / 2.) / sea_oil_amt as f64) as u32;
        self.place_specific_number_of_resources(
            map_parameters,
            Resource::Resource("Oil".to_string()),
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
    fn place_small_quantities_of_strategics(
        &mut self,
        map_parameters: &MapParameters,
        frequency: f64,
        plot_list: &[Tile],
    ) {
        if plot_list.is_empty() {
            return;
        }

        let [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt] =
            Self::get_small_strategic_resource_quantity_values(map_parameters.resource_setting);

        let num_to_place = (plot_list.len() as f64 / frequency).ceil() as u32;

        let mut plot_list_iter = plot_list.iter().peekable();

        let mut num_left_to_place = num_to_place;

        while num_left_to_place > 0 && plot_list_iter.peek().is_some() {
            let tile = *plot_list_iter.next().unwrap();
            let terrain_type = tile.terrain_type(self);
            let base_terrain = tile.base_terrain(self);
            let feature = tile.feature(self);

            let mut selected_resource = None;
            let mut selected_quantity = 2;

            if self.layer_data[&Layer::Strategic][tile.index()] == 0
                && tile.resource(self).is_none()
            {
                // Placing a small strategic resource here. Need to determine what type to place.
                if let Some(feature) = feature {
                    match feature {
                        Feature::Forest => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            match diceroll {
                                0 => {
                                    selected_resource = Some("Uranium");
                                    selected_quantity = uran_amt;
                                }
                                1 => {
                                    selected_resource = Some("Coal");
                                    selected_quantity = coal_amt;
                                }
                                _ => {
                                    selected_resource = Some("Iron");
                                    selected_quantity = iron_amt;
                                }
                            }
                        }
                        Feature::Jungle => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            match diceroll {
                                0 => {
                                    if terrain_type == TerrainType::Hill {
                                        selected_resource = Some("Iron");
                                        selected_quantity = iron_amt;
                                    } else {
                                        selected_resource = Some("Oil");
                                        selected_quantity = oil_amt;
                                    }
                                }
                                1 => {
                                    selected_resource = Some("Coal");
                                    selected_quantity = coal_amt;
                                }
                                _ => {
                                    selected_resource = Some("Aluminum");
                                    selected_quantity = alum_amt;
                                }
                            }
                        }
                        Feature::Marsh => {
                            let diceroll = self.random_number_generator.gen_range(0..4);
                            match diceroll {
                                0 => {
                                    selected_resource = Some("Iron");
                                    selected_quantity = iron_amt;
                                }
                                1 => {
                                    selected_resource = Some("Coal");
                                    selected_quantity = coal_amt;
                                }
                                _ => {
                                    selected_resource = Some("Oil");
                                    selected_quantity = oil_amt;
                                }
                            }
                        }
                        _ => (),
                    }
                } else {
                    match terrain_type {
                        TerrainType::Flatland => match base_terrain {
                            BaseTerrain::Grassland => {
                                if tile.is_freshwater(self, map_parameters) {
                                    selected_resource = Some("Horses");
                                    selected_quantity = horse_amt;
                                } else {
                                    let diceroll = self.random_number_generator.gen_range(0..5);
                                    if diceroll < 3 {
                                        selected_resource = Some("Iron");
                                        selected_quantity = iron_amt;
                                    } else {
                                        selected_resource = Some("Horses");
                                        selected_quantity = horse_amt;
                                    }
                                }
                            }
                            BaseTerrain::Desert => {
                                let diceroll = self.random_number_generator.gen_range(0..3);
                                match diceroll {
                                    0 => {
                                        selected_resource = Some("Iron");
                                        selected_quantity = iron_amt;
                                    }
                                    1 => {
                                        selected_resource = Some("Aluminum");
                                        selected_quantity = alum_amt;
                                    }
                                    _ => {
                                        selected_resource = Some("Oil");
                                        selected_quantity = oil_amt;
                                    }
                                }
                            }
                            BaseTerrain::Plain => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                if diceroll < 2 {
                                    selected_resource = Some("Iron");
                                    selected_quantity = iron_amt;
                                } else {
                                    selected_resource = Some("Horses");
                                    selected_quantity = horse_amt;
                                }
                            }
                            _ => {
                                let diceroll = self.random_number_generator.gen_range(0..4);
                                match diceroll {
                                    0 => {
                                        selected_resource = Some("Iron");
                                        selected_quantity = iron_amt;
                                    }
                                    1 => {
                                        selected_resource = Some("Uranium");
                                        selected_quantity = uran_amt;
                                    }
                                    _ => {
                                        selected_resource = Some("Oil");
                                        selected_quantity = oil_amt;
                                    }
                                }
                            }
                        },
                        TerrainType::Hill => match base_terrain {
                            BaseTerrain::Grassland | BaseTerrain::Plain => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                if diceroll == 2 {
                                    selected_resource = Some("Horses");
                                    selected_quantity = horse_amt;
                                } else if diceroll < 2 {
                                    selected_resource = Some("Iron");
                                    selected_quantity = iron_amt;
                                } else {
                                    selected_resource = Some("Coal");
                                    selected_quantity = coal_amt;
                                }
                            }
                            _ => {
                                let diceroll = self.random_number_generator.gen_range(0..5);
                                if diceroll < 2 {
                                    selected_resource = Some("Iron");
                                    selected_quantity = iron_amt;
                                } else {
                                    selected_resource = Some("Coal");
                                    selected_quantity = coal_amt;
                                }
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

                    self.resource_query[tile.index()] = Some((
                        Resource::Resource(selected_resource.to_string()),
                        selected_quantity,
                    ));
                    self.place_resource_impact(map_parameters, tile, Layer::Strategic, radius);
                    num_left_to_place -= 1;
                }
            }
        }
    }

    // function AssignStartingPlots:AddModernMinorStrategicsToCityStates
    /// Add modern minor strategics to city states.
    /// Mordern strategics contain Oil, Aluminum, Coal.
    fn add_modern_minor_strategics_to_city_states(&mut self, map_parameters: &MapParameters) {
        let [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt] =
            Self::get_small_strategic_resource_quantity_values(map_parameters.resource_setting);
        for _ in 0..map_parameters.city_state_num {
            let city_state_starting_tile = self.city_state_starting_tile_and_region_index[1].0;
            let candidate_strategic_resources = ["Coal", "Oil", "Aluminum"];
            let candidate_resources_amount = [coal_amt, oil_amt, alum_amt];
            let priority_list_indices_of_strategic_resources = [
                [3, 4, 13, 11, 10, 9],
                [9, 1, 13, 14, 11, 10],
                [3, 4, 13, 9, 10, 11],
            ];

            let choosen_resource_index = self.random_number_generator.gen_range(0..4);
            if choosen_resource_index < 3 {
                let strategic_resource = candidate_strategic_resources[choosen_resource_index];
                let resource_amount = candidate_resources_amount[choosen_resource_index];
                let priority_list_indices_of_strategic_resource =
                    priority_list_indices_of_strategic_resources[choosen_resource_index];
                let mut luxury_plot_lists = self.generate_luxury_plot_lists_at_city_site(
                    map_parameters,
                    city_state_starting_tile,
                    3,
                    false,
                );

                let mut priority_list_indices_iter = priority_list_indices_of_strategic_resource
                    .iter()
                    .peekable();

                let mut num_left_to_place = resource_amount;

                while num_left_to_place > 0 && priority_list_indices_iter.peek().is_some() {
                    let i = *priority_list_indices_iter.next().unwrap();

                    luxury_plot_lists[i].shuffle(&mut self.random_number_generator);
                    num_left_to_place = self.place_specific_number_of_resources(
                        map_parameters,
                        Resource::Resource(strategic_resource.to_owned()),
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

    // TODO: We will implement this function later in the future.
    fn get_small_strategic_resource_quantity_values(resource_setting: ResourceSetting) -> [u32; 6] {
        // According to resource_setting, calculate the number of resources to place.
        let [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt] = match resource_setting {
            ResourceSetting::Sparse => [1, 1, 2, 1, 2, 2], // Sparse
            ResourceSetting::Abundant => [3, 3, 3, 3, 3, 3], // Abundant
            _ => [2, 2, 3, 2, 3, 3],                       // Default
        };

        [uran_amt, horse_amt, oil_amt, iron_amt, coal_amt, alum_amt]
    }

    /// Calculates the total quantity of a specified resource
    ///
    /// This function iterates through all resource assignments, checks if the name of each resource
    /// matches the provided resource name, and if so, adds the resource quantity to the total sum.
    /// Finally, it returns the total quantity of the specified resource.
    ///
    /// # Parameters
    /// - `resource`: The name of the resource to look for (string type).
    ///
    /// # Returns
    /// Returns the total quantity of the specified resource as `u32`.
    pub fn placed_resource_count(&self, resource: &str) -> u32 {
        self.resource_query
            .iter()
            .filter_map(|assignment_resource| {
                assignment_resource
                    .as_ref()
                    .and_then(|(assignment_resource, quantity)| {
                        (assignment_resource.name() == resource).then_some(*quantity)
                    })
            })
            .sum()
    }

    // AssignStartingPlots:GenerateGlobalResourcePlotLists
    /// Generates a list of `Vec` of tiles that are available for placing strategic resources.
    /// Each `Vec` is shuffled to ensure randomness.
    ///
    /// # Returns
    /// A `Vec` of shuffled `Vec` of tiles, where each inner `Vec` represents a collection
    /// of tiles that can be used to place strategic resources.
    ///
    fn generate_strategic_resource_plot_lists(
        &mut self,
        map_parameters: &MapParameters,
    ) -> [Vec<Tile>; 11] {
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

        self.iter_tiles().for_each(|tile| {
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
                        if feature.map_or(true, |f| matches!(f, Feature::Forest | Feature::Jungle))
                        {
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
                                    if tile.is_freshwater(self, map_parameters) {
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
