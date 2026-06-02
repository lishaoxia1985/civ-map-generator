use serde::{Deserialize, Serialize};

use crate::ruleset::common::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityStateType {
    pub name: String,
    pub friend_bonus_uniques: Vec<String>,
    pub ally_bonus_uniques: Vec<String>,
    pub color: [u8; 3],
}

impl Name for CityStateType {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
