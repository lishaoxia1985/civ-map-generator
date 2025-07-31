//! This module defines the [`Ruleset`] struct and its associated methods.
//! It provides functionality to load and manage game rules from a ruleset *JSON* file, including beliefs,
//! buildings, nations, policies, quests, specialists, technologies, terrain types,
//! base terrains, features, natural wonders, tile improvements, tile resources,
//! units, unit promotions, and unit types.

use std::collections::HashMap;

use serde::de::DeserializeOwned;

use terrain::{
    base_terrain_info::BaseTerrainInfo, feature_info::FeatureInfo,
    natural_wonder_info::NaturalWonderInfo, terrain_type_info::TerrainTypeInfo,
};

pub mod belief;
pub mod building;
pub mod difficulty;
pub mod era;
pub mod global_unique;
pub mod nation;
pub mod policy;
pub mod quest;
pub mod ruin;
pub mod specialist;
pub mod tech;
pub mod terrain;
pub mod tile_improvement;
pub mod tile_resource;
pub mod unique;
pub mod unit;
pub mod unit_promotion;
pub mod unit_type;

use crate::ruleset::{
    belief::Belief, building::Building, difficulty::Difficulty, era::Era,
    global_unique::GlobalUnique, nation::Nation, policy::PolicyBranch, quest::Quest, ruin::Ruin,
    specialist::Specialist, tech::TechColumn, tile_improvement::TileImprovement,
    tile_resource::TileResource, unit::Unit, unit_promotion::UnitPromotion, unit_type::UnitType,
};

use self::tech::Technology;
pub trait Name {
    fn name(&self) -> String;
}

fn create_hashmap_from_json_file<T: DeserializeOwned + Name>(path: &str) -> HashMap<String, T> {
    let json_string_without_comment = load_json_file_and_strip_json_comments(path);
    let map: Vec<T> = serde_json::from_str(&json_string_without_comment)
        .unwrap_or_else(|_| panic!("{}'{}'", "Can't serde ", path));
    map.into_iter().map(|x| (x.name(), x)).collect()
}

#[derive(Debug)]
pub struct Ruleset {
    pub beliefs: HashMap<String, Belief>,
    pub buildings: HashMap<String, Building>,
    pub difficulties: HashMap<String, Difficulty>,
    pub eras: HashMap<String, Era>,
    pub global_uniques: GlobalUnique,
    pub nations: HashMap<String, Nation>,
    //pub policies: HashMap<String, Policy>,
    pub policy_branches: HashMap<String, PolicyBranch>,
    pub religions: Vec<String>,
    pub ruins: HashMap<String, Ruin>,
    pub quests: HashMap<String, Quest>,
    pub specialists: HashMap<String, Specialist>,
    pub technologies: HashMap<String, Technology>,

    pub terrain_types: HashMap<String, TerrainTypeInfo>,
    pub base_terrains: HashMap<String, BaseTerrainInfo>,
    pub features: HashMap<String, FeatureInfo>,
    pub natural_wonders: HashMap<String, NaturalWonderInfo>,

    pub tile_improvements: HashMap<String, TileImprovement>,
    pub tile_resources: HashMap<String, TileResource>,
    pub units: HashMap<String, Unit>,
    pub unit_promotions: HashMap<String, UnitPromotion>,
    pub unit_types: HashMap<String, UnitType>,
}

impl Default for Ruleset {
    fn default() -> Self {
        Self::new()
    }
}

impl Ruleset {
    pub fn new() -> Self {
        // TODO: load from json, for now just hardcode. This is a temporary solution.
        let beliefs: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Beliefs.json"
        ));

        //serde buildings
        let json_string_without_comment = load_json_file_and_strip_json_comments(include_str!(
            "../jsons/Civ V - Gods & Kings/Buildings.json"
        ));
        let mut buildings: Vec<Building> =
            serde_json::from_str(&json_string_without_comment).unwrap();

        let difficulties: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Difficulties.json"
        ));

        let eras: HashMap<_, _> =
            create_hashmap_from_json_file(include_str!("../jsons/Civ V - Gods & Kings/Eras.json"));

        let nations: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Nations.json"
        ));

        let policy_branches: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Policies.json"
        ));

        let quests: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Quests.json"
        ));

        // serde religions
        let json_string_without_comment = load_json_file_and_strip_json_comments(include_str!(
            "../jsons/Civ V - Gods & Kings/Religions.json"
        ));
        let religions: Vec<String> = serde_json::from_str(&json_string_without_comment).unwrap();

        let ruins: HashMap<_, _> =
            create_hashmap_from_json_file(include_str!("../jsons/Civ V - Gods & Kings/Ruins.json"));

        let specialists: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Specialists.json"
        ));

        // serde terrains
        let terrain_types: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/TerrainTypes.json"
        ));

        let base_terrains: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/BaseTerrains.json"
        ));

        let features: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/Features.json"
        ));

        let natural_wonders: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/NaturalWonders.json"
        ));

        let tile_improvements: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/TileImprovements.json"
        ));

        let tile_resources: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/TileResources.json"
        ));

        let units: HashMap<_, _> =
            create_hashmap_from_json_file(include_str!("../jsons/Civ V - Gods & Kings/Units.json"));

        let unit_promotions: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/UnitPromotions.json"
        ));

        let unit_types: HashMap<_, _> = create_hashmap_from_json_file(include_str!(
            "../jsons/Civ V - Gods & Kings/UnitTypes.json"
        ));

        // serde tech_columnes
        let json_string_without_comment = load_json_file_and_strip_json_comments(include_str!(
            "../jsons/Civ V - Gods & Kings/Techs.json"
        ));
        let mut tech_columnes: Vec<TechColumn> =
            serde_json::from_str(&json_string_without_comment).unwrap();

        tech_columnes.iter_mut().for_each(|tech_column| {
            for technology in tech_column.techs.iter_mut() {
                if technology.cost == 0 {
                    technology.cost = tech_column.tech_cost
                }
                technology.column = tech_column.column_number;
                technology.era.clone_from(&tech_column.era);

                // set building cost
                for building in buildings.iter_mut().filter(|building| {
                    building.required_tech == technology.name
                        && building.cost == 0
                        && !building
                            .uniques
                            .iter()
                            .any(|unique| unique == "Unbuildable")
                }) {
                    if building.is_wonder || building.is_national_wonder {
                        building.cost = tech_column.wonder_cost;
                    } else {
                        building.cost = tech_column.building_cost;
                    }
                }
            }
        });

        let technologies: HashMap<String, Technology> = tech_columnes
            .into_iter()
            .flat_map(|x| x.techs)
            .map(|x| (x.name.to_owned(), x))
            .collect();

        let buildings = buildings
            .into_iter()
            .map(|building| (building.name.to_owned(), building))
            .collect();

        // serde global_uniques
        let json_string_without_comment = load_json_file_and_strip_json_comments(include_str!(
            "../jsons/Civ V - Gods & Kings/GlobalUniques.json"
        ));
        let global_uniques: GlobalUnique =
            serde_json::from_str(&json_string_without_comment).unwrap();

        Self {
            beliefs,
            buildings,
            difficulties,
            eras,
            global_uniques,
            nations,
            //policies: policies,
            policy_branches,
            religions,
            ruins,
            quests,
            specialists,
            technologies,
            terrain_types,
            base_terrains,
            features,
            natural_wonders,
            tile_improvements,
            tile_resources,
            units,
            unit_promotions,
            unit_types,
        }
    }
}

fn load_json_file_and_strip_json_comments(path: &str) -> String {
    let json_string_with_comment = /* fs::read_to_string(path).unwrap() */path;
    strip_json_comments(json_string_with_comment, true)
}

/// Take a JSON string with comments and return the version without comments
/// which can be parsed well by serde_json as the standard JSON string.
/// Support line comment(//...) and block comment(/*...*/)
/// When preserve_locations is true this function will replace all the comments with spaces, so that JSON parsing
/// errors can point to the right location.
pub fn strip_json_comments(json_with_comments: &str, preserve_locations: bool) -> String {
    let mut json_without_comments = String::new();

    let mut block_comment_depth: u8 = 0;
    let mut is_in_string: bool = false; // Comments cannot be in strings

    for line in json_with_comments.split('\n') {
        let mut last_char: Option<char> = None;
        for cur_char in line.chars() {
            // Check whether we're in a string
            if block_comment_depth == 0 && last_char != Some('\\') && cur_char == '"' {
                is_in_string = !is_in_string;
            }

            // Check for line comment start
            if !is_in_string && last_char == Some('/') && cur_char == '/' {
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }
                break; // Stop outputting or parsing this line
            }
            // Check for block comment start
            if !is_in_string && last_char == Some('/') && cur_char == '*' {
                block_comment_depth += 1;
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }
            // Check for block comment end
            } else if !is_in_string && last_char == Some('*') && cur_char == '/' {
                if block_comment_depth > 0 {
                    block_comment_depth = block_comment_depth.saturating_sub(1);
                }
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }

            // Output last char if not in any block comment
            } else {
                if block_comment_depth != 0 {
                    if preserve_locations {
                        json_without_comments.push(' ');
                    }
                } else if let Some(last_char) = last_char {
                    json_without_comments.push(last_char);
                }
                last_char = Some(cur_char);
            }
        }

        // Add last char and newline if not in any block comment
        if let Some(last_char) = last_char {
            if block_comment_depth == 0 {
                json_without_comments.push(last_char);
            } else if preserve_locations {
                json_without_comments.push(' ');
            }
        }

        // Remove trailing whitespace from line
        while json_without_comments.ends_with(' ') {
            json_without_comments.pop();
        }
        json_without_comments.push('\n');
    }

    json_without_comments
}
