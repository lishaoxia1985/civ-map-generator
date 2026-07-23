use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }

    // Define JSON files to monitor for changes
    let monitored_files = [
        "BaseTerrain.json",
        "Belief.json",
        "Building.json",
        "CityStateType.json",
        "Difficulty.json",
        "Era.json",
        "Feature.json",
        "Nation.json",
        "NaturalWonder.json",
        "PolicyBranch.json",
        "Quest.json",
        "Resource.json",
        "Ruin.json",
        "Specialist.json",
        "Speed.json",
        "TerrainType.json",
        "TileImprovement.json",
        "Unit.json",
        "UnitPromotion.json",
        "UnitType.json",
        "VictoryType.json",
    ];

    // Declare files that should trigger rebuilds when changed
    for file in &monitored_files {
        println!(
            "cargo:rerun-if-changed=src/jsons/Civ V - Gods & Kings/{}",
            file
        );
    }

    let json_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("jsons")
        .join("Civ V - Gods & Kings");

    let enums_dir = Path::new("src/ruleset/enums");

    /**********************************************
    Generate enum from JSON files. Each file contains an array of objects,
    where the 'name' field is located at the top level of each array element.
    **********************************************/
    // Dynamically generate enum mappings
    let enum_mappings: Vec<(&str, String, String)> = monitored_files
        .iter()
        .map(|&json_file| {
            let enum_name = json_file.strip_suffix(".json").unwrap();
            let rust_file_name = to_snake_case(enum_name);

            (json_file, rust_file_name, enum_name.to_string())
        })
        .collect();

    // Generate Rust files
    // For example:
    //   When `json_file` is "TerrainType.json", `rust_file` is "terrain_type.rs", and `enum_name` is "TerrainType".
    for (json_file, rust_file_name, enum_name) in enum_mappings.iter() {
        let json_path = json_dir.join(json_file);
        let rust_path = enums_dir.join(format!("{}.rs", rust_file_name));

        // Validate JSON file existence
        if !json_path.exists() {
            panic!("JSON file not found: {}", json_path.display());
        }

        create_enum_from_json(json_path, rust_path, enum_name.as_str());
    }
    /**********************************************
    End of enum generation. Successfully processed JSON arrays where each element
    contains a top-level 'name' field for enum variant creation.
    **********************************************/

    /**********************************************
    Generate enum from JSON files where the 'name' field is NESTED within child objects
    . Requires path traversal to access enum variant identifiers.
    **********************************************/
    // Generate enum `Technology` from JSON file `Technology.json`
    const TECHNOLOGY_JSON_FILE: &str = "Technology.json";
    const TECHNOLOGY_RUST_FILE_NAME: &str = "technology";
    let technology_json_path = json_dir.join(TECHNOLOGY_JSON_FILE);
    let technology_rust_path = enums_dir.join(format!("{}.rs", TECHNOLOGY_RUST_FILE_NAME));
    create_technology_enum_from_json(technology_json_path, technology_rust_path);

    // Generates enum `Policy` from a JSON file `PolicyBranch.json`
    const POLICY_JSON_FILE: &str = "PolicyBranch.json";
    const POLICY_RUST_FILE_NAME: &str = "policy";
    let policy_json_path = json_dir.join(POLICY_JSON_FILE);
    let policy_rust_path = enums_dir.join(format!("{}.rs", POLICY_RUST_FILE_NAME));
    create_policy_enum_from_json(policy_json_path, policy_rust_path);

    //
    const RELIGION_JSON_FILE: &str = "Religion.json";
    const RELIGION_RUST_FILE_NAME: &str = "religion";
    let religion_json_path = json_dir.join(RELIGION_JSON_FILE);
    let religion_rust_path = enums_dir.join(format!("{}.rs", RELIGION_RUST_FILE_NAME));
    create_religion_enum_from_json(religion_json_path, religion_rust_path);

    /**********************************************
    End of enum generation. Successfully processed JSON arrays where 'name' fields are
    NESTED in child structures (e.g., 'metadata.name') for enum variant creation.
    **********************************************/

    let mut rust_file_names: Vec<_> = enum_mappings
        .iter()
        .map(|(_, rust_file_name, _)| rust_file_name.as_str())
        .collect();

    rust_file_names.push(TECHNOLOGY_RUST_FILE_NAME);
    rust_file_names.push(POLICY_RUST_FILE_NAME);
    rust_file_names.push(RELIGION_RUST_FILE_NAME);

    // Generate mod.rs using dynamic enum list
    generate_mod_file(enums_dir, &rust_file_names);
}

/// Generates mod.rs file that re-exports all generated enum types
fn generate_mod_file(output_dir: &Path, rust_file_names: &[&str]) {
    let mod_path = output_dir.join("mod.rs");
    let mut file = File::create(&mod_path)
        .unwrap_or_else(|e| panic!("Failed to create {}: {}", mod_path.display(), e));

    writeln!(file, "// Auto-generated file. Do not edit manually.").unwrap();
    writeln!(file, "// Re-exports all generated enum types").unwrap();
    writeln!(file).unwrap();

    for rust_file_name in rust_file_names {
        let module_name = rust_file_name;
        writeln!(file, "mod {};", module_name).unwrap();
        writeln!(file, "pub use {}::*;", module_name).unwrap();
    }

    writeln!(file).unwrap();
    writeln!(
        file,
        "/// Trait for infallible conversion between enum variants and string representations"
    )
    .unwrap();
    writeln!(file, "/// **PANICS** if string does not match any variant").unwrap();
    writeln!(file, "pub trait EnumStr {{").unwrap();
    writeln!(
        file,
        "    /// Converts enum variant to its canonical string representation"
    )
    .unwrap();
    writeln!(file, "    fn as_str(&self) -> &'static str;").unwrap();
    writeln!(file).unwrap();
    writeln!(
        file,
        "    /// Converts string to enum variant **PANICS on invalid input**"
    )
    .unwrap();
    writeln!(file, "    ///").unwrap();
    writeln!(file, "    /// # Panics").unwrap();
    writeln!(
        file,
        "    /// Panics if `s` does not match any variant's string representation"
    )
    .unwrap();
    writeln!(file, "    fn from_str(s: &str) -> Self;").unwrap();
    writeln!(file, "}}").unwrap();
}

/// Converts PascalCase to snake_case (e.g., `NaturalWonder` -> `natural_wonder`)
fn to_snake_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len() * 2);
    let mut prev_lower = false;

    for (i, c) in name.char_indices() {
        if c.is_uppercase() {
            // Handle consecutive uppercase letters (e.g., XMLParser -> xml_parser)
            if i > 0 && prev_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            result.push(c);
            prev_lower = true;
        }
    }

    // Clean potential leading/trailing underscores
    result.trim_matches('_').to_string()
}

fn generate_enum_code(enum_name: &str, enum_variants: &[String], names: &[&str]) -> String {
    let mut output = String::new();
    output.push_str("// Auto-generated by build.rs, DO NOT EDIT\n");
    output.push_str("use super::EnumStr;\n"); // Import the EnumStr trait from parent module
    output.push_str("use enum_map::Enum;\n");
    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push('\n');

    // Generate enum definition with required derives
    output.push_str(
        "#[derive(Enum, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Debug)]\n",
    );
    output.push_str(&format!("pub enum {} {{\n", enum_name));

    // Add enum variants to output
    for variant in enum_variants.iter() {
        output.push_str(&format!("    {},\n", variant));
    }

    output.push_str("}\n\n");

    // Implement EnumStr trait for the enum
    output.push_str(&format!("impl EnumStr for {} {{\n", enum_name));

    // Implement as_str() method (variant to string)
    output.push_str("    fn as_str(&self) -> &'static str {\n");
    output.push_str("        match self {\n");

    for (variant, name) in enum_variants.iter().zip(names.iter()) {
        output.push_str(&format!(
            "            {}::{} => \"{}\",\n",
            enum_name, variant, name
        ));
    }

    output.push_str("        }\n");
    output.push_str("    }\n\n");

    // Implement from_str() method (string to variant, panics on invalid input)
    output.push_str("    fn from_str(s: &str) -> Self {\n");
    output.push_str("        match s {\n");

    for (variant, name) in enum_variants.iter().zip(names.iter()) {
        output.push_str(&format!(
            "            \"{}\" => {}::{},\n",
            name, enum_name, variant
        ));
    }

    output.push_str("            _ => panic!(\"Invalid value for {}: {{}}\", s),\n");
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("}\n");

    output
}

/// Creates an enum from a JSON file.
fn create_enum_from_json(json_path: PathBuf, dest_path: PathBuf, enum_name: &str) {
    // Load and preprocess JSON file (removing comments)
    let json_string_without_comment = load_json_file_and_strip_json_comments(json_path);

    // Parse JSON into a vector of values
    let value_list: Vec<Value> =
        serde_json::from_str(&json_string_without_comment).expect("Failed to parse JSON");

    // Extract 'name' field from each JSON object
    let names: Vec<&str> = value_list
        .iter()
        .map(|value| {
            value
                .get("name")
                .and_then(|v| v.as_str())
                .expect("Can't get name")
        })
        .collect();

    // Convert JSON names to valid Rust enum variant
    let enum_variants: Vec<String> = names
        .iter()
        .map(|name| {
            let variant: String = name
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars).collect(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<String>>()
                .join("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            variant
        })
        .collect();

    // Generate the Rust code
    let output = generate_enum_code(enum_name, &enum_variants, &names);

    // Write generated code to output file
    let mut file = File::create(dest_path).expect("Could not create output file");
    file.write_all(output.as_bytes())
        .expect("Could not write to file");
}

fn create_technology_enum_from_json(json_path: PathBuf, dest_path: PathBuf) {
    let enum_name = "Technology";

    // Load and preprocess JSON file (removing comments)
    let json_string_without_comment = load_json_file_and_strip_json_comments(json_path);

    // Parse JSON into a vector of values
    let value_list: Vec<Value> =
        serde_json::from_str(&json_string_without_comment).expect("Can't serde current json file");
    let names: Vec<&str> = value_list
        .iter()
        .filter_map(|item| item["techs"].as_array())
        .flatten()
        .filter_map(|tech| tech["name"].as_str())
        .collect();

    // Convert JSON names to valid Rust enum variant
    let enum_variants: Vec<String> = names
        .iter()
        .map(|name| {
            let variant: String = name
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars).collect(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<String>>()
                .join("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            variant
        })
        .collect();

    // Generate the Rust code
    let output = generate_enum_code(enum_name, &enum_variants, &names);

    // Write generated code to output file
    let mut file = File::create(dest_path).expect("Could not create output file");
    file.write_all(output.as_bytes())
        .expect("Could not write to file");
}

fn create_policy_enum_from_json(json_path: PathBuf, dest_path: PathBuf) {
    let enum_name = "Policy";

    // Load and preprocess JSON file (remove comments)
    let json_string_without_comment = load_json_file_and_strip_json_comments(json_path);

    // Parse JSON into a vector of civilization policy trees
    let value_list: Vec<Value> =
        serde_json::from_str(&json_string_without_comment).expect("Failed to parse JSON file");

    // Extract all policy names from every civilization's 'policies' array
    let names: Vec<&str> = value_list
        .iter()
        .filter_map(|civ| civ.get("policies").and_then(|p| p.as_array()))
        .flatten()
        .filter_map(|policy| policy.get("name").and_then(|n| n.as_str()))
        .collect();

    // Convert policy names to valid Rust enum variants
    // - Capitalize first letter of each word
    // - Remove whitespace and non-alphanumeric characters
    let enum_variants: Vec<String> = names
        .iter()
        .map(|name| {
            name.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    chars
                        .next()
                        .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect()
        })
        .collect();

    // Generate Rust enum code
    let output = generate_enum_code(enum_name, &enum_variants, &names);

    // Write generated code to destination file
    let mut file = File::create(dest_path).expect("Failed to create output file");
    file.write_all(output.as_bytes())
        .expect("Failed to write to file");
}

fn create_religion_enum_from_json(json_path: PathBuf, dest_path: PathBuf) {
    let enum_name = "Religion";

    // Load and preprocess JSON file (remove comments)
    let json_string_without_comment = load_json_file_and_strip_json_comments(json_path);

    // Parse JSON into a vector of religions
    let names: Vec<&str> =
        serde_json::from_str(&json_string_without_comment).expect("Failed to parse JSON file");

    // Convert religion names to valid Rust enum variants
    // - Capitalize first letter of each word
    // - Remove whitespace and non-alphanumeric characters
    let enum_variants: Vec<String> = names
        .iter()
        .map(|name| {
            name.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    chars
                        .next()
                        .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join("")
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect()
        })
        .collect();

    // Generate Rust enum code
    let output = generate_enum_code(enum_name, &enum_variants, &names);

    // Write generated code to destination file
    let mut file = File::create(dest_path).expect("Failed to create output file");
    file.write_all(output.as_bytes())
        .expect("Failed to write to file");
}

fn load_json_file_and_strip_json_comments(path: PathBuf) -> String {
    let json_string_with_comment = fs::read_to_string(path).unwrap();
    strip_json_comments(&json_string_with_comment, true)
}

/// Take a JSON string with comments and return the version without comments
/// which can be parsed well by serde_json as the standard JSON string.
/// Support line comment(//...) and block comment(/\*...\*/)
///
/// When `preserve_locations` is true this function will replace all the comments with spaces, so that JSON parsing
/// errors can point to the right location.
pub fn strip_json_comments(json_with_comments: &str, preserve_locations: bool) -> String {
    let mut json_without_comments = String::new();

    let mut block_comment_depth: u8 = 0;
    let mut is_in_string: bool = false; // Comments cannot be in strings

    for line in json_with_comments.split('\n') {
        let mut last_char: Option<char> = None;
        for cur_char in line.chars() {
            // Check whether we're in a string
            if block_comment_depth == 0 && last_char != Some('\\') && cur_char == '"' {
                is_in_string = !is_in_string;
            }

            // Check for line comment start
            if !is_in_string && last_char == Some('/') && cur_char == '/' {
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }
                break; // Stop outputting or parsing this line
            }
            // Check for block comment start
            if !is_in_string && last_char == Some('/') && cur_char == '*' {
                block_comment_depth += 1;
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }
            // Check for block comment end
            } else if !is_in_string && last_char == Some('*') && cur_char == '/' {
                if block_comment_depth > 0 {
                    block_comment_depth = block_comment_depth.saturating_sub(1);
                }
                last_char = None;
                if preserve_locations {
                    json_without_comments.push_str("  ");
                }

            // Output last char if not in any block comment
            } else {
                if block_comment_depth != 0 {
                    if preserve_locations {
                        json_without_comments.push(' ');
                    }
                } else if let Some(last_char) = last_char {
                    json_without_comments.push(last_char);
                }
                last_char = Some(cur_char);
            }
        }

        // Add last char and newline if not in any block comment
        if let Some(last_char) = last_char {
            if block_comment_depth == 0 {
                json_without_comments.push(last_char);
            } else if preserve_locations {
                json_without_comments.push(' ');
            }
        }

        // Remove trailing whitespace from line
        while json_without_comments.ends_with(' ') {
            json_without_comments.pop();
        }
        json_without_comments.push('\n');
    }

    json_without_comments
}
