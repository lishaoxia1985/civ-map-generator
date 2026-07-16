use super::{
    Name,
    common::{RequiredTerrain, Yields},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    pub name: String,
    pub resource_type: String,
    #[serde(default)]
    pub required_terrain: Vec<RequiredTerrain>,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub improvement: String,
    #[serde(default)]
    pub revealed_by: String,
    #[serde(default)]
    pub improvement_stats: HashMap<String, i8>,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub major_deposit_amount: HashMap<String, i8>,
    #[serde(default)]
    pub minor_deposit_amount: HashMap<String, i8>,
}

impl Name for ResourceInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
