use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyBranch {
    pub name: String,
    pub era: String,
    pub prioritie: Option<HashMap<String, i8>>,
    pub uniques: Vec<String>,
    pub policies: Vec<Policy>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub uniques: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub row: i8,
    #[serde(default)]
    pub column: i8,
}

impl Name for PolicyBranch {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
