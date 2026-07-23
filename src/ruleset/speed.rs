use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub years_per_turn: f64,
    pub until_turn: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedInfo {
    pub name: String,
    pub modifier: f64,
    pub production_cost_modifier: f64,
    pub gold_cost_modifier: f64,
    pub science_cost_modifier: f64,
    pub culture_cost_modifier: f64,
    pub faith_cost_modifier: f64,
    pub improvement_build_length_modifier: f64,
    pub barbarian_modifier: f64,
    pub gold_gift_modifier: f64,
    pub city_state_tribute_scaling_interval: f64,
    pub golden_age_length_modifier: f64,
    pub religious_pressure_adjacent_city: i32,
    pub peace_deal_duration: i32,
    pub deal_duration: i32,
    pub start_year: i32,
    pub turns: Vec<Turn>,
}
