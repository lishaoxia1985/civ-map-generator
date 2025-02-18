use rand::{seq::SliceRandom, Rng};

use crate::{
    component::{
        base_terrain::BaseTerrain, feature::Feature, resource::Resource, terrain_type::TerrainType,
    },
    tile_map::{
        tile::Tile, Layer, MapParameters, RegionDivideMethod, RegionType, ResourceSetting, TileMap,
    },
};

use super::assign_starting_tile::ResourceToPlace;

impl TileMap {
    pub fn place_bonus_resources(&mut self, map_parameters: &MapParameters) {
        // Adjust appearance rate per Resource Setting chosen by user.
        let bonus_multiplier = match map_parameters.resource_setting {
            // Sparse, so increase the number of tiles per bonus.
            ResourceSetting::Sparse => 1.5,
            // Abundant, so reduce the number of tiles per bonus.
            ResourceSetting::Abundant => 2.0 / 3.0,
            _ => 1.0,
        };

        let [extra_deer_list, desert_wheat_list, banana_list, coast_list, hills_open_list, dry_grass_flat_no_feature, grass_flat_no_feature, plains_flat_no_feature, tundra_flat_no_feature, desert_flat_no_feature, forest_flat_that_are_not_tundra] =
            self.generate_bonus_resource_plot_lists(map_parameters);

        self.place_fish(map_parameters, 10. * bonus_multiplier, &coast_list);
        self.place_sexy_bonus_at_civ_starts(map_parameters);
        self.add_extra_bonuses_to_hills_regions(map_parameters);

        let resources_to_place = [ResourceToPlace {
            resource: "Deer".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            8. * bonus_multiplier,
            Layer::Bonus,
            &extra_deer_list,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Wheat".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 0,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            10.0 * bonus_multiplier,
            Layer::Bonus,
            &desert_wheat_list,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Deer".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            12.0 * bonus_multiplier,
            Layer::Bonus,
            &tundra_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Bananas".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 0,
            max_radius: 3,
        }];
        self.process_resource_list(
            map_parameters,
            14.0 * bonus_multiplier,
            Layer::Bonus,
            &banana_list,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Wheat".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 2,
            max_radius: 3,
        }];
        self.process_resource_list(
            map_parameters,
            50.0 * bonus_multiplier,
            Layer::Bonus,
            &plains_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Bison".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 2,
            max_radius: 3,
        }];
        self.process_resource_list(
            map_parameters,
            60.0 * bonus_multiplier,
            Layer::Bonus,
            &plains_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Cow".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            18.0 * bonus_multiplier,
            Layer::Bonus,
            &grass_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Stone".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 1,
        }];
        self.process_resource_list(
            map_parameters,
            30.0 * bonus_multiplier,
            Layer::Bonus,
            &dry_grass_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Bison".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 1,
        }];
        self.process_resource_list(
            map_parameters,
            50.0 * bonus_multiplier,
            Layer::Bonus,
            &dry_grass_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Sheep".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 1,
        }];
        self.process_resource_list(
            map_parameters,
            13.0 * bonus_multiplier,
            Layer::Bonus,
            &hills_open_list,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Stone".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            15.0 * bonus_multiplier,
            Layer::Bonus,
            &tundra_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Stone".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 1,
            max_radius: 2,
        }];
        self.process_resource_list(
            map_parameters,
            19.0 * bonus_multiplier,
            Layer::Bonus,
            &desert_flat_no_feature,
            &resources_to_place,
        );

        let resources_to_place = vec![ResourceToPlace {
            resource: "Deer".to_string(),
            quantity: 1,
            weight: 100,
            min_radius: 3,
            max_radius: 4,
        }];
        self.process_resource_list(
            map_parameters,
            25.0 * bonus_multiplier,
            Layer::Bonus,
            &forest_flat_that_are_not_tundra,
            &resources_to_place,
        );
    }

    // function AssignStartingPlots:AddExtraBonusesToHillsRegions
    fn add_extra_bonuses_to_hills_regions(&mut self, map_parameters: &MapParameters) {
        // Identify Hills Regions, if any.
        let mut hills_region_indices = Vec::new();
        for (region_index, region) in self.region_list.iter().enumerate() {
            if region.region_type == RegionType::Hill {
                hills_region_indices.push(region_index);
            }
        }

        if hills_region_indices.is_empty() {
            return;
        }

        hills_region_indices.shuffle(&mut self.random_number_generator);

        for region_index in hills_region_indices {
            let terrain_statistic = &self.region_list[region_index].terrain_statistic;

            let hill_and_flatland_tile_num = terrain_statistic.terrain_type_sum[&TerrainType::Hill]
                + terrain_statistic.terrain_type_sum[&TerrainType::Flatland];
            // Evaluate the level of infertility in the region by comparing the rugged terrain of hills and mountains to the flat farmlands.
            let mut hills_ratio = (terrain_statistic.terrain_type_sum[&TerrainType::Hill]
                + terrain_statistic.terrain_type_sum[&TerrainType::Mountain])
                as f64
                / hill_and_flatland_tile_num as f64;
            let mut farm_ratio = (terrain_statistic.base_terrain_sum[&BaseTerrain::Grassland]
                + terrain_statistic.base_terrain_sum[&BaseTerrain::Plain])
                as f64
                / hill_and_flatland_tile_num as f64;
            if let RegionDivideMethod::WholeMapRectangle = map_parameters.region_divide_method {
                hills_ratio = (terrain_statistic.terrain_type_sum[&TerrainType::Hill]
                    + terrain_statistic.terrain_type_sum[&TerrainType::Mountain])
                    as f64
                    / (hill_and_flatland_tile_num
                        + terrain_statistic.terrain_type_sum[&TerrainType::Mountain])
                        as f64;
                farm_ratio = (terrain_statistic.base_terrain_sum[&BaseTerrain::Grassland]
                    + terrain_statistic.base_terrain_sum[&BaseTerrain::Plain])
                    as f64
                    / (hill_and_flatland_tile_num
                        + terrain_statistic.terrain_type_sum[&TerrainType::Mountain])
                        as f64;
            }
            // If the infertility quotient is greater than 1, it will increase the number of Bonuses placed, up to a maximum of twice the normal ratio.
            let infertility_quotient = 1.0 + f64::max(hills_ratio - farm_ratio, 0.0);

            let rectangle = self.region_list[region_index].rectangle;
            let landmass_id = self.region_list[region_index].landmass_id;

            let mut forests = Vec::new();
            let mut jungles = Vec::new();
            let mut flat_plains = Vec::new();
            let mut dry_hills = Vec::new();
            let mut flat_grass = Vec::new();
            let mut flat_tundra = Vec::new();
            for tile in rectangle.iter_tiles(map_parameters) {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                let feature = tile.feature(self);
                let area_id = tile.area_id(self);
                if tile.resource(self).is_none()
                    && matches!(terrain_type, TerrainType::Hill | TerrainType::Flatland)
                {
                    // Check plot for region membership. Only process this plot if it is a member.
                    if landmass_id == Some(area_id) || landmass_id == None {
                        if let Some(feature) = feature {
                            match feature {
                                Feature::Forest => {
                                    forests.push(tile);
                                }
                                Feature::Jungle => {
                                    jungles.push(tile);
                                }

                                Feature::Floodplain => {
                                    flat_plains.push(tile);
                                }
                                _ => {}
                            }
                        } else {
                            if terrain_type == TerrainType::Hill {
                                if matches!(
                                    base_terrain,
                                    BaseTerrain::Grassland
                                        | BaseTerrain::Plain
                                        | BaseTerrain::Tundra
                                ) && !tile.is_freshwater(self, map_parameters)
                                {
                                    dry_hills.push(tile);
                                }
                            } else if terrain_type == TerrainType::Flatland {
                                match base_terrain {
                                    BaseTerrain::Grassland => {
                                        flat_grass.push(tile);
                                    }
                                    BaseTerrain::Desert => {
                                        if tile.is_freshwater(self, map_parameters) {
                                            flat_plains.push(tile);
                                        }
                                    }
                                    BaseTerrain::Plain => {
                                        flat_plains.push(tile);
                                    }
                                    BaseTerrain::Tundra => {
                                        flat_tundra.push(tile);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            if dry_hills.len() > 0 {
                let resources_to_place = [ResourceToPlace {
                    resource: "Sheep".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 0,
                    max_radius: 1,
                }];
                self.process_resource_list(
                    map_parameters,
                    9. / infertility_quotient,
                    Layer::Bonus,
                    &dry_hills,
                    &resources_to_place,
                );
            }

            if !jungles.is_empty() {
                let resources_to_place = [ResourceToPlace {
                    resource: "Bananas".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 1,
                    max_radius: 2,
                }];
                self.process_resource_list(
                    map_parameters,
                    14. / infertility_quotient,
                    Layer::Bonus,
                    &jungles,
                    &resources_to_place,
                );
            }

            if !flat_tundra.is_empty() {
                let resources_to_place = [ResourceToPlace {
                    resource: "Deer".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 0,
                    max_radius: 1,
                }];
                self.process_resource_list(
                    map_parameters,
                    14. / infertility_quotient,
                    Layer::Bonus,
                    &flat_tundra,
                    &resources_to_place,
                );
            }

            if !flat_plains.is_empty() {
                let resources_to_place = [ResourceToPlace {
                    resource: "Wheat".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 0,
                    max_radius: 2,
                }];
                self.process_resource_list(
                    map_parameters,
                    18. / infertility_quotient,
                    Layer::Bonus,
                    &flat_plains,
                    &resources_to_place,
                );
            }

            if !flat_grass.is_empty() {
                let resources_to_place = [ResourceToPlace {
                    resource: "Cow".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 0,
                    max_radius: 2,
                }];
                self.process_resource_list(
                    map_parameters,
                    20. / infertility_quotient,
                    Layer::Bonus,
                    &flat_grass,
                    &resources_to_place,
                );
            }

            if !forests.is_empty() {
                let resources_to_place = [ResourceToPlace {
                    resource: "Deer".to_string(),
                    quantity: 1,
                    weight: 100,
                    min_radius: 1,
                    max_radius: 2,
                }];
                self.process_resource_list(
                    map_parameters,
                    24. / infertility_quotient,
                    Layer::Bonus,
                    &forests,
                    &resources_to_place,
                );
            }
        }
    }

    // function AssignStartingPlots:PlaceSexyBonusAtCivStarts
    // This function places a bonus resource in the third ring around a Civ's capital.
    // The added bonus is intended to make the starting location more appealing.
    // Third-ring resources take longer to develop but provide significant benefits in the late game.
    // Alternatively, if another city is settled nearby and takes control of this tile, the resource may benefit that city instead.
    fn place_sexy_bonus_at_civ_starts(&mut self, map_parameters: &MapParameters) {
        let bonus_type_associated_with_region_type = [
            (RegionType::Tundra, "Deer"),
            (RegionType::Jungle, "Bananas"),
            (RegionType::Forest, "Deer"),
            (RegionType::Desert, "Wheat"),
            (RegionType::Hill, "Sheep"),
            (RegionType::Plain, "Wheat"),
            (RegionType::Grassland, "Cow"),
            (RegionType::Hybrid, "Cow"),
        ];

        let mut plot_list = Vec::new();
        let mut fish_list = Vec::new();

        for i in 0..self.region_list.len() {
            let starting_tile = self.region_list[i].starting_tile;
            let region_type = self.region_list[i].region_type;
            let chosen_bonus_resource = bonus_type_associated_with_region_type
                .iter()
                .find(|(region_type_, _)| *region_type_ == region_type)
                .unwrap()
                .1;
            starting_tile
                .tiles_at_distance(3, map_parameters)
                .iter()
                .for_each(|&tile| {
                    let terrain_type = tile.terrain_type(self);
                    let base_terrain = tile.base_terrain(self);
                    let feature = tile.feature(self);
                    match chosen_bonus_resource {
                        "Deer" => {
                            if feature == Some(Feature::Forest) {
                                plot_list.push(tile);
                            } else if terrain_type == TerrainType::Flatland
                                && base_terrain == BaseTerrain::Tundra
                            {
                                plot_list.push(tile);
                            }
                        }
                        "Bananas" => {
                            if feature == Some(Feature::Jungle) {
                                plot_list.push(tile);
                            }
                        }
                        "Wheat" => {
                            if terrain_type == TerrainType::Flatland {
                                if base_terrain == BaseTerrain::Plain && feature.is_none() {
                                    plot_list.push(tile);
                                } else if feature == Some(Feature::Floodplain) {
                                    plot_list.push(tile);
                                } else if base_terrain == BaseTerrain::Desert
                                    && tile.is_freshwater(self, map_parameters)
                                {
                                    plot_list.push(tile);
                                }
                            }
                        }

                        "Sheep" => {
                            if terrain_type == TerrainType::Hill
                                && feature.is_none()
                                && matches!(
                                    base_terrain,
                                    BaseTerrain::Plain
                                        | BaseTerrain::Grassland
                                        | BaseTerrain::Tundra
                                )
                            {
                                plot_list.push(tile);
                            }
                        }
                        "Cow" => {
                            if terrain_type == TerrainType::Flatland
                                && feature.is_none()
                                && base_terrain == BaseTerrain::Grassland
                            {
                                plot_list.push(tile);
                            }
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                    if base_terrain == BaseTerrain::Coast
                        && feature != Some(Feature::Atoll)
                        && feature != Some(Feature::Ice)
                    {
                        fish_list.push(tile);
                    }
                });
            if plot_list.len() > 0 {
                plot_list.shuffle(&mut self.random_number_generator);
                self.place_specific_number_of_resources(
                    map_parameters,
                    Resource::Resource(chosen_bonus_resource.to_string()),
                    1,
                    1,
                    1.,
                    None,
                    0,
                    0,
                    &plot_list,
                );
                // Hills region, attempt to give them a second Sexy Sheep.
                if plot_list.len() > 1 && chosen_bonus_resource == "Sheep" {
                    self.place_specific_number_of_resources(
                        map_parameters,
                        Resource::Resource(chosen_bonus_resource.to_string()),
                        1,
                        1,
                        1.,
                        None,
                        0,
                        0,
                        &plot_list,
                    );
                }
            } else if fish_list.len() > 0 {
                fish_list.shuffle(&mut self.random_number_generator);
                self.place_specific_number_of_resources(
                    map_parameters,
                    Resource::Resource("Fish".to_string()),
                    1,
                    1,
                    1.,
                    None,
                    0,
                    0,
                    &fish_list,
                );
            }
        }
    }

    // function AssignStartingPlots:PlaceFish
    fn place_fish(&mut self, map_parameters: &MapParameters, frequency: f64, coast_list: &[Tile]) {
        if coast_list.is_empty() {
            return;
        }

        let num_fish_to_place = (coast_list.len() as f64 / frequency).ceil() as u32;
        let mut coast_list_iter = coast_list.iter().peekable();

        let mut num_left_to_place = num_fish_to_place;

        while num_left_to_place > 0 && coast_list_iter.peek().is_some() {
            let tile = *coast_list_iter.next().unwrap();
            if self.layer_data[&Layer::Fish][tile.index()] == 0 && tile.resource(self).is_none() {
                // Probability distribution for the possible values of fish_radius: 0, 1, 2, 3, 4, 5
                //
                // The probability for 0, 1, and 2 is 1/7 each
                // The probability for 3 is 2/7 (because when 3 or 6 is generated, fish_radius is set to 3)
                // The probability for 4 and 5 is 1/7 each
                let mut fish_radius = self.random_number_generator.gen_range(0..7);
                if fish_radius > 5 {
                    fish_radius = 3;
                }
                self.resource_query[tile.index()] =
                    Some((Resource::Resource("Fish".to_string()), 1));
                self.place_resource_impact(map_parameters, tile, Layer::Fish, fish_radius);
                num_left_to_place -= 1;
            }
        }
    }

    // AssignStartingPlots:GenerateGlobalResourcePlotLists
    /// Generates a list of `Vec` of tiles that are available for placing bonus resources.
    /// Each `Vec` is shuffled to ensure randomness.
    ///
    /// # Returns
    /// A `Vec` of shuffled `Vec` of tiles, where each inner `Vec` represents a collection
    /// of tiles that can be used to place bonus resources.
    ///
    fn generate_bonus_resource_plot_lists(
        &mut self,
        map_parameters: &MapParameters,
    ) -> [Vec<Tile>; 11] {
        let mut extra_deer_list = Vec::new(); // forest, tundra, (hill or flat)
        let mut desert_wheat_list = Vec::new(); // flood_plain or flat desert with fresh water
        let mut banana_list = Vec::new(); // jungle, (hill or flat)
        let mut coast_list = Vec::new();
        let mut hills_open_list = Vec::new();
        let mut dry_grass_flat_no_feature = Vec::new();
        let mut grass_flat_no_feature = Vec::new();
        let mut plains_flat_no_feature = Vec::new();
        let mut tundra_flat_no_feature = Vec::new();
        let mut desert_flat_no_feature = Vec::new();
        let mut forest_flat_that_are_not_tundra = Vec::new();

        self.iter_tiles().for_each(|tile| {
            if !self.player_collision_data[tile.index()] && tile.resource(self).is_none() {
                let terrain_type = tile.terrain_type(self);
                let base_terrain = tile.base_terrain(self);
                let feature = tile.feature(self);

                if base_terrain == BaseTerrain::Tundra && feature == Some(Feature::Forest) {
                    extra_deer_list.push(tile);
                }

                if feature == Some(Feature::Floodplain)
                    || (terrain_type == TerrainType::Flatland
                        && base_terrain == BaseTerrain::Desert
                        && feature == None
                        && tile.is_freshwater(self, map_parameters))
                {
                    desert_wheat_list.push(tile);
                }

                if feature == Some(Feature::Jungle) {
                    banana_list.push(tile);
                }

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
                        // flatland_list.push(tile);
                        if let Some(feature) = feature {
                            match feature {
                                Feature::Forest => {
                                    // forest_flat_list.push(tile);
                                    if base_terrain == BaseTerrain::Tundra {
                                        /* region_tundra_flat_including_forest_tile_list
                                        .push(tile); */
                                    } else {
                                        forest_flat_that_are_not_tundra.push(tile);
                                    }
                                }
                                Feature::Jungle => {
                                    // jungle_flat_list.push(tile);
                                }
                                Feature::Marsh => {
                                    // marsh_list.push(tile);
                                }
                                Feature::Floodplain => {
                                    /* region_flood_plain_tile_list.push(tile); */
                                }
                                _ => {}
                            }
                        } else {
                            match base_terrain {
                                BaseTerrain::Grassland => {
                                    grass_flat_no_feature.push(tile);
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
                                    // snow_flat_list.push(tile);
                                }
                                _ => {}
                            }
                        }
                    }
                    TerrainType::Mountain => {}
                    TerrainType::Hill => {
                        if base_terrain != BaseTerrain::Snow {
                            // hills_list.push(tile);
                            if feature == None {
                                hills_open_list.push(tile);
                            } /* else if feature == Some(Feature::Forest) {
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
            extra_deer_list,
            desert_wheat_list,
            banana_list,
            coast_list,
            hills_open_list,
            dry_grass_flat_no_feature,
            grass_flat_no_feature,
            plains_flat_no_feature,
            tundra_flat_no_feature,
            desert_flat_no_feature,
            forest_flat_that_are_not_tundra,
        ];

        // Shuffle each list. This is done to ensure that the order in which resources are placed is random.
        lists.iter_mut().for_each(|list| {
            list.shuffle(&mut self.random_number_generator);
        });

        lists
    }
}
