use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ruleset::common::{RequiredTerrain, Yields};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileImprovementInfo {
    pub name: String,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub required_terrain: Vec<RequiredTerrain>,
    #[serde(default)]
    pub required_tech: String,
    #[serde(default)]
    pub turns_to_build: i8,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub shortcut_key: Option<char>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
}

impl Name for TileImprovementInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
