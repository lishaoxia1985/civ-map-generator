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

use crate::ruleset::enums::*;
use enum_map::{Enum, EnumArray, EnumMap};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

// Notes: we don't re-export the mod `enums` by `pub use`,
// so we make it publically.
pub mod enums;

// The modules we re-export at the following code.
mod base_terrain;
mod belief;
mod building;
mod city_state_type;
mod common;
mod difficulty;
mod era;
mod feature;
mod global_unique;
mod nation;
mod natural_wonder;
mod policy;
mod quest;
mod resource;
mod ruin;
mod specialist;
mod speed;
mod tech;
mod terrain_type;
mod tile_improvement;
mod unit;
mod unit_promotion;
mod unit_type;
mod victory_type;

pub use crate::ruleset::{
    base_terrain::*, belief::*, building::*, city_state_type::*, common::*, difficulty::*, era::*,
    feature::*, global_unique::*, nation::*, natural_wonder::*, policy::*, quest::*, resource::*,
    ruin::*, specialist::*, speed::*, tech::*, terrain_type::*, tile_improvement::*, unit::*,
    unit_promotion::*, unit_type::*, victory_type::*,
};

/// Creates an [`EnumMap`] from a JSON file.
fn create_enum_map_from_json_file<M, T>(path: PathBuf) -> EnumMap<M, T>
where
    M: EnumStr + EnumArray<T>,
    T: DeserializeOwned,
{
    let json_string_without_comment = load_json_file_and_strip_json_comments(path);
    let items: Vec<T> =
        serde_json::from_str(&json_string_without_comment).expect("Failed to parse JSON file");

    let mut items_iter = items.into_iter();

    EnumMap::from_fn(|_| items_iter.next().expect("Not enough items in JSON file"))
}

#[derive(Debug)]
pub struct Ruleset {
    // The structs related to terrains
    pub terrain_types: EnumMap<TerrainType, TerrainTypeInfo>,
    pub base_terrains: EnumMap<BaseTerrain, BaseTerrainInfo>,
    pub features: EnumMap<Feature, FeatureInfo>,
    pub natural_wonders: EnumMap<NaturalWonder, NaturalWonderInfo>,
    pub resources: EnumMap<Resource, ResourceInfo>,

    pub ruins: EnumMap<Ruin, RuinInfo>,

    pub tile_improvements: EnumMap<TileImprovement, TileImprovementInfo>,

    pub buildings: EnumMap<Building, BuildingInfo>,
    pub specialists: EnumMap<Specialist, SpecialistInfo>,

    pub units: EnumMap<Unit, UnitInfo>,
    pub unit_promotions: EnumMap<UnitPromotion, UnitPromotionInfo>,
    pub unit_types: EnumMap<UnitType, UnitTypeInfo>,

    pub beliefs: EnumMap<Belief, BeliefInfo>,

    pub nations: EnumMap<Nation, NationInfo>,
    pub city_state_types: EnumMap<CityStateType, CityStateTypeInfo>,

    pub policy_branches: EnumMap<PolicyBranch, PolicyBranchInfo>,
    pub policies: EnumMap<Policy, PolicyInfo>,

    pub technologies: EnumMap<Technology, TechnologyInfo>,

    pub quests: EnumMap<Quest, QuestInfo>,

    pub difficulties: EnumMap<Difficulty, DifficultyInfo>,
    pub speeds: EnumMap<Speed, SpeedInfo>,
    pub eras: EnumMap<Era, EraInfo>,
    pub victory_types: EnumMap<VictoryType, VictoryTypeInfo>,

    pub global_uniques: GlobalUnique,
    pub religions: Vec<Religion>,
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
        /* **********Loading standard ruleset JSON file********** */

        let terrain_types: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("TerrainType.json"));

        let base_terrains: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("BaseTerrain.json"));

        let features: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Feature.json"));

        let natural_wonders: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("NaturalWonder.json"));

        let resources: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Resource.json"));

        let ruins: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Ruin.json"));

        let tile_improvements: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("TileImprovement.json"));

        let specialists: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Specialist.json"));

        let units: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Unit.json"));

        let unit_promotions: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("UnitPromotion.json"));

        let unit_types: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("UnitType.json"));

        let beliefs: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Belief.json"));

        // Note: We will set building's cost later, so now it is mutable.
        let mut buildings: EnumMap<_, BuildingInfo> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Building.json"));

        let difficulties: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Difficulty.json"));

        let eras: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Era.json"));

        let nations: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Nation.json"));

        let city_state_types: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("CityStateType.json"));

        let policy_branches: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("PolicyBranch.json"));

        let quests: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Quest.json"));

        let victory_types: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("VictoryType.json"));

        let speeds: EnumMap<_, _> =
            create_enum_map_from_json_file(ruleset_json_folder.join("Speed.json"));

        /* **********End of Loading standard ruleset JSON file********** */

        /* **********The JSON file below we should tackle by special way********** */

        // serde `Religion`
        let religions: Vec<Religion> = (0..Religion::LENGTH).map(Religion::from_usize).collect();

        // serde `global_uniques`
        let json_string_without_comment =
            load_json_file_and_strip_json_comments(ruleset_json_folder.join("GlobalUnique.json"));
        let global_uniques: GlobalUnique =
            serde_json::from_str(&json_string_without_comment).unwrap();

        // serde `TechColumn`
        let json_string_without_comment =
            load_json_file_and_strip_json_comments(ruleset_json_folder.join("Technology.json"));
        let mut tech_columnes: Vec<TechColumn> = serde_json::from_str(&json_string_without_comment)
            .expect("Failed to parse Technology.json");

        // Store techs and related wonders and buildings costs in a map for faster lookup
        let mut tech_and_wonder_or_building_cost = HashMap::new();

        tech_columnes.iter_mut().for_each(|tech_column| {
            for technology in tech_column.techs.iter_mut() {
                // We only set the cost for technology when the cost is not set.
                // 0 means the cost is not set yet by `JSON`.
                if technology.cost == 0 {
                    technology.cost = tech_column.tech_cost;
                }

                // Assign column index and era to the technology.
                technology.column = tech_column.column_number;
                technology.era = tech_column.era.clone();

                tech_and_wonder_or_building_cost.insert(
                    &technology.name,
                    (tech_column.wonder_cost, tech_column.building_cost),
                );
            }
        });

        // Set building cost
        //
        // We only set the cost if the condition below is met:
        // 1. The building has a required tech
        // 2. The building's cost is not set yet (0 means that the cost is not set yet by `JSON`)
        // 3. The building can be built by the player
        for building in buildings.values_mut() {
            if building.cost != 0
                || building
                    .required_terrain
                    .extra_conditions
                    .iter()
                    .any(|condition| condition == "Unbuildable")
            {
                continue;
            }

            // Get wonder cost and building cost according to the required technologies
            let Some(&(wonder_cost, building_cost)) =
                tech_and_wonder_or_building_cost.get(&building.required_tech)
            else {
                unreachable!(
                    "Building {} requires tech {}, which is not in the tech column",
                    building.name, building.required_tech
                );
            };

            building.cost = if building.is_wonder || building.is_national_wonder {
                wonder_cost
            } else {
                building_cost
            };
        }

        let mut technology_info_iter = tech_columnes.into_iter().flat_map(|x| x.techs);

        let technologies: EnumMap<Technology, TechnologyInfo> = EnumMap::from_fn(|_| {
            technology_info_iter
                .next()
                .expect("Not enough items in JSON file")
        });

        // TODO: Will not use `clone` here in the future.
        let mut policy_info_iter = policy_branches
            .values()
            .flat_map(|policy_branch: &PolicyBranchInfo| policy_branch.policies.clone());

        let policies: EnumMap<Policy, PolicyInfo> = EnumMap::from_fn(|_| {
            policy_info_iter
                .next()
                .expect("Not enough items in JSON file")
        });

        Self {
            terrain_types,
            base_terrains,
            features,
            natural_wonders,
            resources,
            ruins,
            tile_improvements,
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
            policies,
            technologies,
            quests,
            difficulties,
            speeds,
            victory_types,
            eras,
            global_uniques,
        }
    }
}

fn load_json_file_and_strip_json_comments(path: PathBuf) -> String {
    let json_string_with_comment = fs::read_to_string(path).expect("Failed to read JSON file");
    strip_json_comments(&json_string_with_comment, true)
}

/// Take a JSON string with comments and return the version without comments
/// which can be parsed well by serde_json as the standard JSON string.
/// Support line comment(//...) and block comment(/\*...\*/)
///
/// When `preserve_locations` is true this function will replace all the comments with spaces, so that JSON parsing
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
