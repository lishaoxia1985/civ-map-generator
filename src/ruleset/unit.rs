#[cfg(feature = "use-hashbrown")]
use hashbrown::{HashMap, HashSet};

#[cfg(not(feature = "use-hashbrown"))]
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    name: String,
    unit_type: String,
    movement: i8,
    #[serde(default)]
    strength: i16,
    #[serde(default)]
    cost: i16,
    #[serde(default)]
    obsolete_tech: String,
    #[serde(default)]
    unique_to: String,
    #[serde(default)]
    replaces: String,
    #[serde(default)]
    upgrades_to: String,
    #[serde(default)]
    hurry_cost_modifier: i8,
    #[serde(default)]
    uniques: Vec<String>,
    civilopedia_text: Option<Vec<HashMap<String, String>>>,
    #[serde(default)]
    promotions: Vec<String>,
    #[serde(default)]
    attack_sound: String,
}

impl Name for Unit {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
