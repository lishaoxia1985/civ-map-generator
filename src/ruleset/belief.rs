use super::Name;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BeliefInfo {
    name: String,
    r#type: String,
    uniques: Vec<String>,
}

impl Name for BeliefInfo {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
