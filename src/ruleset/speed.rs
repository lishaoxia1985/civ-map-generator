use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub years_per_turn: f32,
    pub until_turn: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedInfo {
    pub name: String,
    pub modifier: f32,
    pub production_cost_modifier: f32,
    pub gold_cost_modifier: f32,
    pub science_cost_modifier: f32,
    pub culture_cost_modifier: f32,
    pub faith_cost_modifier: f32,
    pub improvement_build_length_modifier: f32,
    pub barbarian_modifier: f32,
    pub gold_gift_modifier: f32,
    pub city_state_tribute_scaling_interval: f32,
    pub golden_age_length_modifier: f32,
    pub religious_pressure_adjacent_city: i32,
    pub peace_deal_duration: i32,
    pub deal_duration: i32,
    pub start_year: i32,
    pub turns: Vec<Turn>,
}
