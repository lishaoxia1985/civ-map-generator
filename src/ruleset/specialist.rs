#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Specialist {
    name: String,
    #[serde(default)]
    food: i8,
    #[serde(default)]
    production: i8,
    #[serde(default)]
    science: i8,
    #[serde(default)]
    gold: i8,
    #[serde(default)]
    culture: i8,
    #[serde(default)]
    faith: i8,
    #[serde(default)]
    happiness: i8,
    great_person_points: HashMap<String, i8>,
    color: Option<[u8; 3]>,
}

impl Name for Specialist {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
