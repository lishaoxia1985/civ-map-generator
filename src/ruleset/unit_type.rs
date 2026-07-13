use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitTypeInfo {
    name: String,
    movement_type: String,
}

impl Name for UnitTypeInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
