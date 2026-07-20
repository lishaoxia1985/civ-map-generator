use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EraInfo {
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
    city_sound: String,
    #[serde(rename = "iconRGB")]
    icon_rgb: [u8; 3],
}
