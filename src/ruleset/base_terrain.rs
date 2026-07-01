use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ruleset::common::Yields;

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseTerrainInfo {
    pub name: String,
    pub r#type: String,
    #[serde(flatten)]
    pub yields: Yields,
    #[serde(default)]
    pub movement_cost: i8,
    #[serde(rename = "RGB")]
    #[serde(default)]
    pub rgb: [u8; 3],
    /// Latitude range for base terrain placement, where the first element is the minimum
    /// latitude and the second element is the maximum latitude.
    ///
    /// The interval type is dynamically determined by the boundary values:
    /// - `[0.0, 0.0]`: Valid only at the equator.
    /// - `[a, 1.0]`: A closed interval valid for `a <= latitude <= 1.0`.
    ///   - e.g. `[0.0, 1.0]` means that base terrain can be placed anywhere.
    /// - Other `[a, b]`: A half-open interval valid for `a <= latitude < b`.
    ///   This applies when `a != 0.0` and `b` is neither `0.0` nor `1.0`.
    ///
    /// # Notes
    ///
    /// The `latitude` range is affected by the `temperature` parameter in [`crate::map_parameters::MapParameters`].
    /// The default value loaded from the JSON file corresponds to the [`Temperature::Normal`](crate::map_parameters::Temperature::Normal) setting.
    /// See the [`crate::TileMap::generate_base_terrains`] method for more details.
    latitude: [f64; 2],
    #[serde(default)]
    pub uniques: Vec<String>,
    #[serde(default)]
    pub civilopedia_text: Vec<HashMap<String, String>>,
}

impl Name for BaseTerrainInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

impl BaseTerrainInfo {
    pub fn has_unique(&self, unique: &str) -> bool {
        self.uniques.iter().any(|x| x == unique)
    }
}
