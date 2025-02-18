use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct Unique {
    pub placeholder_text: String,
    pub params: Vec<String>,
    pub conditionals: Vec<Unique>,
}

impl Unique {
    pub fn new(unique: &str) -> Self {
        static PARAMS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[(.+?)\]").unwrap());
        static CONDITION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<([^>]*)>").unwrap());

        let unique_without_conditionals = CONDITION_REGEX.replace_all(unique, "");
        let unique_without_conditionals = unique_without_conditionals.trim();

        let placeholder_text = PARAMS_REGEX
            .replace_all(unique_without_conditionals, "[]")
            .to_string();

        let params = PARAMS_REGEX
            .captures_iter(unique_without_conditionals)
            .map(|cap| cap[1].to_owned())
            .collect();

        let conditionals = CONDITION_REGEX
            .captures_iter(unique)
            .map(|cap| Self::new(&cap[1]))
            .collect();

        Self {
            placeholder_text,
            params,
            conditionals,
        }
    }
}
