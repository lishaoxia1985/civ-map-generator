use super::{Name, enums::VictoryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyBranchInfo {
    pub name: String,
    pub era: String,
    /// The priority that Civilization choose this policy branch is up to its victory type.
    pub priorities: HashMap<VictoryType, u8>,
    pub uniques: Vec<String>,
    pub policies: Vec<Policy>,
}

// TODO: Will not derive `Clone` in the future.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Policy {
    pub name: String,
    pub uniques: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub row: u8,
    #[serde(default)]
    pub column: u8,
}

impl Name for PolicyBranchInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
