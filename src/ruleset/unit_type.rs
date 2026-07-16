use super::Name;
use serde::{Deserialize, Serialize};

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
