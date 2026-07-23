use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EraInfo {
    name: String,
    research_agreement_cost: i32,
    starting_settler_count: i32,
    starting_worker_count: i32,
    starting_military_unit_count: i32,
    starting_military_unit: String,
    settler_population: i32,
    base_unit_buy_cost: i32,
    embark_defense: i32,
    start_percent: i32,
    city_sound: String,
    #[serde(rename = "iconRGB")]
    icon_rgb: [u8; 3],
}
