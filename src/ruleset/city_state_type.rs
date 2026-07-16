use super::Name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityStateTypeInfo {
    pub name: String,
    pub friend_bonus_uniques: Vec<String>,
    pub ally_bonus_uniques: Vec<String>,
    pub color: [u8; 3],
}

impl Name for CityStateTypeInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
