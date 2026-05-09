use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ruleset::Yields;

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Specialist {
    name: String,
    #[serde(flatten)]
    pub yields: Yields,
    great_person_points: HashMap<String, i8>,
    #[serde(default)]
    color: [u8; 3],
}

impl Name for Specialist {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
