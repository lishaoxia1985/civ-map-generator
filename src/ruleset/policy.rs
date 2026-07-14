use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyBranchInfo {
    pub name: String,
    pub era: String,
    pub prioritie: Option<HashMap<String, i8>>,
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
