use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechColumn {
    pub column_number: i8,
    pub era: String,
    pub tech_cost: i16,
    pub building_cost: i16,
    #[serde(default)]
    pub wonder_cost: i16,
    pub techs: Vec<Technology>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Technology {
    pub name: String,
    #[serde(default)]
    pub cost: i16,
    pub row: i8,
    #[serde(default)]
    pub column: i8,
    #[serde(default)]
    pub era: String,
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub quote: String,
}
