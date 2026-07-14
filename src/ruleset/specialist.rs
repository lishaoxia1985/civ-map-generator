use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ruleset::common::Yields;

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistInfo {
    name: String,
    #[serde(flatten)]
    pub yields: Yields,
    great_person_points: HashMap<String, i8>,
    #[serde(default)]
    color: [u8; 3],
}

impl Name for SpecialistInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
