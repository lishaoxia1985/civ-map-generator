use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BeliefInfo {
    name: String,
    r#type: String,
    uniques: Vec<String>,
}
