use super::common::{RequiredTerrain, Yields};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub turns_to_build: i32,
    #[serde(default)]
    pub unique_to: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    pub shortcut_key: Option<char>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
}
