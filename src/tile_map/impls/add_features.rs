use rand::{seq::SliceRandom, Rng};

use crate::{
    grid::WorldSizeType,
    map_parameters::Rainfall,
    ruleset::Ruleset,
    tile_component::{base_terrain::BaseTerrain, feature::Feature, terrain_type::TerrainType},
    tile_map::{MapParameters, TileMap},
};

impl TileMap {
    /// Add features to the tile map.
    pub fn add_features(&mut self, map_parameters: &MapParameters, ruleset: &Ruleset) {
        let grid = self.world_grid.grid;
        let rainfall = match map_parameters.rainfall {
            Rainfall::Arid => -4,
            Rainfall::Normal => 0,
            Rainfall::Wet => 4,
            Rainfall::Random => self.random_number_generator.gen_range(0..11) - 5,
        };

        let equator_adjustment = 0;
        let mut jungle_percent = 12;
        let mut forest_percent = 18;
        let mut marsh_percent = 3;
        let mut oasis_percent = 1;

        jungle_percent += rainfall;
        forest_percent += rainfall;
        marsh_percent += rainfall / 2;
        oasis_percent += rainfall / 4;

        let equator = equator_adjustment;

        let jungle_max_percent = jungle_percent;
        let forest_max_percent = forest_percent;
        let marsh_max_percent = marsh_percent;
        let oasis_max_percent = oasis_percent;

        let mut forest_count = 0;
        let mut jungle_count = 0;
        let mut marsh_count = 0;
        let mut oasis_count = 0;
        let mut num_land_plots = 0;
        let jungle_bottom = equator - (jungle_percent as f64 * 0.5).ceil() as i32;
        let jungle_top = equator + (jungle_percent as f64 * 0.5).ceil() as i32;

        for tile in self.all_tiles() {
            let latitude = tile.latitude(grid);

            /* **********start to add ice********** */
            if tile.is_impassable(self, ruleset) {
                continue;
            } else if tile.terrain_type(self) == TerrainType::Water {
                if !tile.has_river(self)
                    && ruleset.features["Ice"]
                        .occurs_on_type
                        .contains(&tile.terrain_type(self))
                    && ruleset.features["Ice"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                    && latitude > 0.78
                {
                    let mut score = self.random_number_generator.gen_range(0..100) as f64;
                    score += latitude * 100.;
                    if tile
                        .neighbor_tiles(grid)
                        .any(|tile| tile.terrain_type(self) != TerrainType::Water)
                    {
                        score /= 2.0;
                    }
                    let a = tile
                        .neighbor_tiles(grid)
                        .filter(|tile| tile.feature(self) == Some(Feature::Ice))
                        .count();
                    score += 10. * a as f64;
                    if score > 130. {
                        tile.set_feature(self, Feature::Ice);
                    }
                }
            }
            /* **********the end of add ice********** */
            else {
                /* **********start to add Floodplain********** */
                num_land_plots += 1;
                if tile.has_river(self)
                    && ruleset.features["Floodplain"]
                        .occurs_on_type
                        .contains(&tile.terrain_type(self))
                    && ruleset.features["Floodplain"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                {
                    tile.set_feature(self, Feature::Floodplain);
                    continue;
                }
                /* **********the end of add Floodplain********** */
                /* **********start to add oasis********** */
                else if ruleset.features["Oasis"]
                    .occurs_on_type
                    .contains(&tile.terrain_type(self))
                    && ruleset.features["Oasis"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                    && (oasis_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= oasis_max_percent
                    && self.random_number_generator.gen_range(0..4) == 1
                {
                    tile.set_feature(self, Feature::Oasis);
                    oasis_count += 1;
                    continue;
                }
                /* **********the end of add oasis********** */
                /* **********start to add march********** */
                if ruleset.features["Marsh"]
                    .occurs_on_type
                    .contains(&tile.terrain_type(self))
                    && ruleset.features["Marsh"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                    && (marsh_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= marsh_max_percent
                {
                    let mut score = 300;

                    let a = tile
                        .neighbor_tiles(grid)
                        .filter(|tile| tile.feature(self) == Some(Feature::Marsh))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        tile.set_feature(self, Feature::Marsh);
                        marsh_count += 1;
                        continue;
                    }
                };
                /* **********the end of add march********** */
                /* **********start to add jungle********** */
                if ruleset.features["Jungle"]
                    .occurs_on_type
                    .contains(&tile.terrain_type(self))
                    && ruleset.features["Jungle"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                    && (jungle_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= jungle_max_percent
                    && (latitude >= jungle_bottom as f64 / 100.
                        && latitude <= jungle_top as f64 / 100.)
                {
                    let mut score = 300;

                    let a = tile
                        .neighbor_tiles(grid)
                        .filter(|tile| tile.feature(self) == Some(Feature::Jungle))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        tile.set_feature(self, Feature::Jungle);
                        if tile.terrain_type(self) == TerrainType::Hill
                            && matches!(
                                tile.base_terrain(self),
                                BaseTerrain::Grassland | BaseTerrain::Plain
                            )
                        {
                            tile.set_base_terrain(self, BaseTerrain::Plain);
                        } else {
                            tile.set_terrain_type(self, TerrainType::Flatland);
                            tile.set_base_terrain(self, BaseTerrain::Plain);
                        }

                        jungle_count += 1;
                        continue;
                    }
                }
                /* **********the end of add jungle********** */
                /* **********start to add forest********** */
                if ruleset.features["Forest"]
                    .occurs_on_type
                    .contains(&tile.terrain_type(self))
                    && ruleset.features["Forest"]
                        .occurs_on_base
                        .contains(&tile.base_terrain(self))
                    && (forest_count as f64 * 100. / num_land_plots as f64).ceil() as i32
                        <= forest_max_percent
                {
                    let mut score = 300;

                    let a = tile
                        .neighbor_tiles(grid)
                        .filter(|tile| tile.feature(self) == Some(Feature::Forest))
                        .count();
                    match a {
                        0 => (),
                        1 => score += 50,
                        2 | 3 => score += 150,
                        4 => score -= 50,
                        _ => score -= 200,
                    };
                    if self.random_number_generator.gen_range(0..300) <= score {
                        tile.set_feature(self, Feature::Forest);
                        forest_count += 1;
                        continue;
                    }
                }
                /* **********the end of add forest********** */
            }
        }

        /* **********start to add atolls********** */
        self.add_atolls();
        /* **********the end of add atolls********** */
    }

    fn add_atolls(&mut self) {
        let grid = self.world_grid.grid;

        let biggest_water_area_id = self.get_biggest_water_area_id();

        let num_tiles = self.area_list[biggest_water_area_id].size;

        // If the biggest water area is too small, we can't place any atolls.
        if num_tiles <= grid.size.area() / 4 {
            return;
        }

        let atoll_target = match self.world_grid.world_size_type {
            WorldSizeType::Duel => 2,
            WorldSizeType::Tiny => 4,
            WorldSizeType::Small => 5,
            WorldSizeType::Standard => 7,
            WorldSizeType::Large => 9,
            WorldSizeType::Huge => 12,
        };

        let atoll_number = atoll_target + self.random_number_generator.gen_range(0..atoll_target);

        let mut alpha_list = Vec::new();
        let mut beta_list = Vec::new();
        let mut gamma_list = Vec::new();
        let mut delta_list = Vec::new();
        let mut epsilon_list = Vec::new();

        for tile in self.all_tiles() {
            if tile.base_terrain(self) == BaseTerrain::Coast
                && tile.feature(self) != Some(Feature::Ice)
            {
                // Collect all neighboring tiles that satisfy these conditions:
                // - Terrain: Hill or Flatland
                // - Base terrain: Neither Tundra nor Snow
                // - Feature: Not Ice
                let neighbor_tile_list: Vec<_> = tile
                    .neighbor_tiles(grid)
                    .filter(|neighbor| {
                        matches!(
                            neighbor.terrain_type(self),
                            TerrainType::Hill | TerrainType::Flatland
                        ) && neighbor.base_terrain(self) != BaseTerrain::Tundra
                            && neighbor.base_terrain(self) != BaseTerrain::Snow
                            && neighbor.feature(self) != Some(Feature::Ice)
                    })
                    .collect();

                // If there's exactly one valid neighbor, we can consider it as a candidate for an atoll.
                if neighbor_tile_list.len() == 1 {
                    let neighbor_tile = neighbor_tile_list[0];
                    let area_id = neighbor_tile.area_id(self);
                    let adjacent_land_area_size = self.area_list[area_id].size;
                    match adjacent_land_area_size {
                        76.. => continue,
                        41..=75 => epsilon_list.push(tile),
                        17..=40 => delta_list.push(tile),
                        8..=16 => gamma_list.push(tile),
                        3..=7 => beta_list.push(tile),
                        1..=2 => alpha_list.push(tile),
                        _ => unreachable!(),
                    }
                }
            }
        }

        alpha_list.shuffle(&mut self.random_number_generator);
        beta_list.shuffle(&mut self.random_number_generator);
        gamma_list.shuffle(&mut self.random_number_generator);
        delta_list.shuffle(&mut self.random_number_generator);
        epsilon_list.shuffle(&mut self.random_number_generator);

        // Determine maximum number able to be placed, per candidate category.
        let mut max_alpha = alpha_list.len().div_ceil(4);
        let mut max_beta = beta_list.len().div_ceil(5);
        let mut max_gamma = gamma_list.len().div_ceil(4);
        let mut max_delta = delta_list.len().div_ceil(3);
        let mut max_epsilon = epsilon_list.len().div_ceil(4);

        let mut alpha_list_iter = alpha_list.into_iter();
        let mut beta_list_iter = beta_list.into_iter();
        let mut gamma_list_iter = gamma_list.into_iter();
        let mut delta_list_iter = delta_list.into_iter();
        let mut epsilon_list_iter = epsilon_list.into_iter();

        for _ in 0..atoll_number {
            let diceroll = self.random_number_generator.gen_range(1..=100);
            let tile;

            match diceroll {
                1..=40 if max_alpha > 0 => {
                    tile = alpha_list_iter.next();
                    max_alpha -= 1;
                }
                41..=65 => {
                    if max_beta > 0 {
                        tile = beta_list_iter.next();
                        max_beta -= 1;
                    } else if max_alpha > 0 {
                        tile = alpha_list_iter.next();
                        max_alpha -= 1;
                    } else {
                        // Unable to place this Atoll
                        continue;
                    }
                }
                66..=80 => {
                    if max_gamma > 0 {
                        tile = gamma_list_iter.next();
                        max_gamma -= 1;
                    } else if max_beta > 0 {
                        tile = beta_list_iter.next();
                        max_beta -= 1;
                    } else if max_alpha > 0 {
                        tile = alpha_list_iter.next();
                        max_alpha -= 1;
                    } else {
                        // Unable to place this Atoll
                        continue;
                    }
                }
                81..=90 => {
                    if max_delta > 0 {
                        tile = delta_list_iter.next();
                        max_delta -= 1;
                    } else if max_gamma > 0 {
                        tile = gamma_list_iter.next();
                        max_gamma -= 1;
                    } else if max_beta > 0 {
                        tile = beta_list_iter.next();
                        max_beta -= 1;
                        // println!("- Beta site chosen");
                    } else if max_alpha > 0 {
                        tile = alpha_list_iter.next();
                        max_alpha -= 1;
                    } else {
                        // Unable to place this Atoll
                        continue;
                    }
                }
                _ => {
                    // This case should happen in 2 conditions:
                    // 1. diceroll in [91..=100];
                    // 2. diceroll in [1..=40] but max_alpha == 0
                    if max_epsilon > 0 {
                        tile = epsilon_list_iter.next();
                        max_epsilon -= 1;
                    } else if max_delta > 0 {
                        tile = delta_list_iter.next();
                        max_delta -= 1;
                    } else if max_gamma > 0 {
                        tile = gamma_list_iter.next();
                        max_gamma -= 1;
                    } else if max_beta > 0 {
                        tile = beta_list_iter.next();
                        max_beta -= 1;
                    } else if max_alpha > 0 {
                        tile = alpha_list_iter.next();
                        max_alpha -= 1;
                    } else {
                        // Unable to place this Atoll
                        continue;
                    }
                }
            }
            // Place the Atoll on the tile
            if let Some(tile) = tile {
                tile.set_feature(self, Feature::Atoll);
            }
        }
    }

    fn get_biggest_water_area_id(&self) -> usize {
        self.area_list
            .iter()
            .filter(|area| area.is_water)
            .max_by_key(|area| area.size)
            .expect("No area found!") // Ensure that there's at least one area.
            .id
    }
}
