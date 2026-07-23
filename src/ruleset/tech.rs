use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechColumn {
    pub column_number: u8,
    pub era: String,
    pub tech_cost: i32,
    pub building_cost: i32,
    pub wonder_cost: i32,
    pub techs: Vec<TechnologyInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TechnologyInfo {
    pub name: String,
    #[serde(default)]
    pub cost: i32,
    pub row: u8,
    #[serde(default)]
    pub column: u8,
    #[serde(default)]
    pub era: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub quote: String,
}
