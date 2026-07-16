use super::{Name, common::Yields};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
