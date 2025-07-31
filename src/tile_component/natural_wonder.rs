use crate::ruleset::Ruleset;

#[derive(Clone)]
pub enum NaturalWonder {
    NaturalWonder(String),
}

impl NaturalWonder {
    pub fn name(&self) -> &str {
        match self {
            NaturalWonder::NaturalWonder(name) => name,
        }
    }

    pub fn impassable(&self, ruleset: &Ruleset) -> bool {
        ruleset.natural_wonders[self.name()].impassable
    }
}
