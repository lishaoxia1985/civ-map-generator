use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyInfo {
    pub name: String,
    pub base_happiness: i32,
    pub extra_happiness_per_luxury: i32,
    pub research_cost_modifier: f32,
    pub unit_cost_modifier: f32,
    pub unit_supply_base: i32,
    pub unit_supply_per_city: i32,
    pub building_cost_modifier: f32,
    pub policy_cost_modifier: f32,
    pub unhappiness_modifier: f32,
    pub barbarian_bonus: f32,
    pub barbarian_spawn_delay: i32,
    pub player_bonus_starting_units: Vec<String>,
    pub ai_city_growth_modifier: f32,
    pub ai_unit_cost_modifier: f32,
    pub ai_building_cost_modifier: f32,
    pub ai_wonder_cost_modifier: f32,
    pub ai_building_maintenance_modifier: f32,
    pub ai_unit_maintenance_modifier: f32,
    pub ai_unit_supply_modifier: f32,
    pub ai_free_techs: Vec<String>,
    pub ai_major_civ_bonus_starting_units: Vec<String>,
    #[serde(default)]
    pub ai_city_state_bonus_starting_units: Vec<String>,
    pub ai_unhappiness_modifier: f32,
    pub ais_exchange_techs: bool,
    pub turn_barbarians_can_enter_player_tiles: i32,
    pub clear_barbarian_camp_reward: i32,
}
