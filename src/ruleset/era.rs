use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Era {
    name: String,
    research_agreement_cost: i16,
    starting_settler_count: i8,
    starting_worker_count: i8,
    starting_military_unit_count: i8,
    starting_military_unit: String,
    settler_population: i8,
    base_unit_buy_cost: i16,
    embark_defense: i8,
    start_percent: i8,
    friend_bonus: HashMap<String, Vec<String>>,
    ally_bonus: HashMap<String, Vec<String>>,
    #[serde(rename = "iconRGB")]
    icon_rgb: [u8; 3],
}

impl Name for Era {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
