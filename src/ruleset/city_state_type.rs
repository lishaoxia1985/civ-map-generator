use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityStateTypeInfo {
    pub name: String,
    pub friend_bonus_uniques: Vec<String>,
    pub ally_bonus_uniques: Vec<String>,
    pub color: [u8; 3],
}
