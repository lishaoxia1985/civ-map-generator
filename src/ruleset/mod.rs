//! This module defines the [`Ruleset`] struct and its associated methods.
//! It provides functionality to load and manage game rules from a ruleset *JSON* file, including beliefs,
//! buildings, nations, policies, quests, specialists, technologies, terrain types,
//! base terrains, features, natural wonders, tile improvements, tile resources,
//! units, unit promotions, and unit types.
//!
//! # Error Handling
//!
//! The [`Ruleset::new`] method will panic if any JSON file cannot be loaded or parsed.
//! For production use, consider implementing proper error handling with `Result` types.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

pub mod base_terrain;
pub mod belief;
pub mod building;
pub mod city_state_type;
pub mod common;
pub mod difficulty;
pub mod enums;
pub mod era;
pub mod feature;
pub mod global_unique;
pub mod nation;
pub mod natural_wonder;
pub mod policy;
pub mod quest;
pub mod resource;
pub mod ruin;
pub mod specialist;
pub mod tech;
pub mod terrain_type;
pub mod tile_improvement;
pub mod unit;
pub mod unit_promotion;
pub mod unit_type;

use crate::ruleset::{
    base_terrain::BaseTerrainInfo,
    belief::BeliefInfo,
    building::BuildingInfo,
    city_state_type::CityStateTypeInfo,
    common::Name,
    difficulty::DifficultyInfo,
    era::EraInfo,
    feature::FeatureInfo,
    global_unique::GlobalUnique,
    nation::NationInfo,
    natural_wonder::NaturalWonderInfo,
    policy::PolicyBranch,
    quest::Quest,
    resource::ResourceInfo,
    ruin::Ruin,
    specialist::Specialist,
    tech::{TechColumn, Technology},
    terrain_type::TerrainTypeInfo,
    tile_improvement::TileImprovementInfo,
    unit::Unit,
    unit_promotion::UnitPromotionInfo,
    unit_type::UnitTypeInfo,
};

fn create_hashmap_from_json_file<T: DeserializeOwned + Name>(path: &str) -> HashMap<String, T> {
    let json_string_without_comment = load_json_file_and_strip_json_comments(path);
    let map: Vec<T> = serde_json::from_str(&json_string_without_comment)
        .unwrap_or_else(|e| panic!("Failed to parse JSON file '{}': {}", path, e));
    map.into_iter().map(|x| (x.name(), x)).collect()
}

#[derive(Debug)]
pub struct Ruleset {
    // The structs related to terrains
    pub terrain_types: HashMap<String, TerrainTypeInfo>,
    pub base_terrains: HashMap<String, BaseTerrainInfo>,
    pub features: HashMap<String, FeatureInfo>,
    pub natural_wonders: HashMap<String, NaturalWonderInfo>,
    pub resources: HashMap<String, ResourceInfo>,

    pub ruins: HashMap<String, Ruin>,

    pub tile_improvements: HashMap<String, TileImprovementInfo>,

    pub buildings: HashMap<String, BuildingInfo>,
    pub specialists: HashMap<String, Specialist>,

    pub units: HashMap<String, Unit>,
    pub unit_promotions: HashMap<String, UnitPromotionInfo>,
    pub unit_types: HashMap<String, UnitTypeInfo>,

    pub religions: Vec<String>,
    pub beliefs: HashMap<String, BeliefInfo>,

    pub nations: HashMap<String, NationInfo>,
    pub city_state_types: HashMap<String, CityStateTypeInfo>,

    // pub policies: HashMap<String, Policy>,
    pub policy_branches: HashMap<String, PolicyBranch>,

    pub technologies: HashMap<String, Technology>,

    pub quests: HashMap<String, Quest>,

    pub difficulties: HashMap<String, DifficultyInfo>,
    pub eras: HashMap<String, EraInfo>,
    pub global_uniques: GlobalUnique,
}

impl Default for Ruleset {
    /// Creates a default ruleset.
    ///
    /// The default ruleset is based on the `Civ V - Gods & Kings` ruleset.
    /// Views the folder in the path [`src/jsons/Civ V - Gods & Kings`] for more information.
    fn default() -> Self {
        let ruleset_json_folder =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/jsons/Civ V - Gods & Kings");
        Self::new(ruleset_json_folder)
    }
}

impl Ruleset {
    /// Creates a new Ruleset from a folder containing json files.
    ///
    /// The folder should the same structure as the folder [`src/jsons/Civ V - Gods & Kings`].
    /// Views the folder in the path [`src/jsons/Civ V - Gods & Kings`] for more information.
    pub fn new(ruleset_json_folder: PathBuf) -> Self {
        let terrain_types: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("TerrainType.json")
                .to_str()
                .unwrap(),
        );

        let base_terrains: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("BaseTerrain.json")
                .to_str()
                .unwrap(),
        );

        let features: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Feature.json").to_str().unwrap(),
        );

        let natural_wonders: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("NaturalWonder.json")
                .to_str()
                .unwrap(),
        );

        let resources: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Resource.json").to_str().unwrap(),
        );

        let ruins: HashMap<_, _> =
            create_hashmap_from_json_file(ruleset_json_folder.join("Ruin.json").to_str().unwrap());

        let tile_improvements: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("TileImprovement.json")
                .to_str()
                .unwrap(),
        );

        let specialists: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("Specialist.json")
                .to_str()
                .unwrap(),
        );

        let units: HashMap<_, _> =
            create_hashmap_from_json_file(ruleset_json_folder.join("Unit.json").to_str().unwrap());

        let unit_promotions: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("UnitPromotion.json")
                .to_str()
                .unwrap(),
        );

        let unit_types: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("UnitType.json").to_str().unwrap(),
        );

        let beliefs: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Belief.json").to_str().unwrap(),
        );

        let mut buildings: HashMap<_, BuildingInfo> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Building.json").to_str().unwrap(),
        );

        let difficulties: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("Difficulty.json")
                .to_str()
                .unwrap(),
        );

        let eras: HashMap<_, _> =
            create_hashmap_from_json_file(ruleset_json_folder.join("Era.json").to_str().unwrap());

        let nations: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Nation.json").to_str().unwrap(),
        );

        let city_state_types: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder
                .join("CityStateType.json")
                .to_str()
                .unwrap(),
        );

        let policy_branches: HashMap<_, _> = create_hashmap_from_json_file(
            ruleset_json_folder.join("Policy.json").to_str().unwrap(),
        );

        let quests: HashMap<_, _> =
            create_hashmap_from_json_file(ruleset_json_folder.join("Quest.json").to_str().unwrap());

        // serde religions
        let json_string_without_comment = load_json_file_and_strip_json_comments(
            ruleset_json_folder.join("Religion.json").to_str().unwrap(),
        );
        let religions: Vec<String> = serde_json::from_str(&json_string_without_comment).unwrap();

        // serde tech_columnes
        let json_string_without_comment = load_json_file_and_strip_json_comments(
            ruleset_json_folder
                .join("Technology.json")
                .to_str()
                .unwrap(),
        );
        let mut tech_columnes: Vec<TechColumn> =
            serde_json::from_str(&json_string_without_comment).unwrap();

        tech_columnes.iter_mut().for_each(|tech_column| {
            // Set tech cost
            for technology in tech_column.techs.iter_mut() {
                // We only set the cost for technology that the cost is not set.
                // 0 means the cost is not set yet by `JSON`
                if technology.cost == 0 {
                    technology.cost = tech_column.tech_cost
                }
                technology.column = tech_column.column_number;
                technology.era.clone_from(&tech_column.era);

                // Set building cost
                for building in buildings.values_mut() {
                    // We only set the cost if the condition below is met:
                    // 1. The building has a required tech
                    // 2. The building's cost is not set yet (0 means that the cost is not set yet by `JSON`)
                    // 3. The building can be built by the player
                    if building.required_tech == technology.name
                        && building.cost == 0
                        && !building
                            .uniques
                            .iter()
                            .any(|unique| unique == "Unbuildable")
                    {
                        if building.is_wonder || building.is_national_wonder {
                            building.cost = tech_column.wonder_cost;
                        } else {
                            building.cost = tech_column.building_cost;
                        }
                    }
                }
            }
        });

        let technologies: HashMap<String, Technology> = tech_columnes
            .into_iter()
            .flat_map(|x| x.techs)
            .map(|x| (x.name.to_owned(), x))
            .collect();

        // serde global_uniques
        let json_string_without_comment = load_json_file_and_strip_json_comments(
            ruleset_json_folder
                .join("GlobalUnique.json")
                .to_str()
                .unwrap(),
        );
        let global_uniques: GlobalUnique =
            serde_json::from_str(&json_string_without_comment).unwrap();

        Self {
            terrain_types,
            base_terrains,
            features,
            natural_wonders,
            ruins,
            tile_improvements,
            resources,
            buildings,
            specialists,
            units,
            unit_promotions,
            unit_types,
            religions,
            beliefs,
            nations,
            city_state_types,
            policy_branches,
            technologies,
            quests,
            difficulties,
            eras,
            global_uniques,
        }
    }
}

fn load_json_file_and_strip_json_comments(path: &str) -> String {
    let json_string_with_comment = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read JSON file '{}': {}", path, e));
    strip_json_comments(&json_string_with_comment, true)
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
