use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Difficulty {
    name: String,
    base_happiness: i8,
    extra_happiness_per_luxury: i8,
    research_cost_modifier: f32,
    unit_cost_modifier: f32,
    unit_supply_base: i8,
    unit_supply_per_city: i8,
    building_cost_modifier: f32,
    policy_cost_modifier: f32,
    unhappiness_modifier: f32,
    barbarian_bonus: f32,
    barbarian_spawn_delay: i8,
    player_bonus_starting_units: Vec<String>,
    ai_city_growth_modifier: f32,
    ai_unit_cost_modifier: f32,
    ai_building_cost_modifier: f32,
    ai_wonder_cost_modifier: f32,
    ai_building_maintenance_modifier: f32,
    ai_unit_maintenance_modifier: f32,
    ai_unit_supply_modifier: f32,
    ai_free_techs: Vec<String>,
    ai_major_civ_bonus_starting_units: Vec<String>,
    #[serde(default)]
    ai_city_state_bonus_starting_units: Vec<String>,
    ai_unhappiness_modifier: f32,
    ais_exchange_techs: bool,
    turn_barbarians_can_enter_player_tiles: i16,
    clear_barbarian_camp_reward: i8,
}

impl Name for Difficulty {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
