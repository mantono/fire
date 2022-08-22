use handlebars::Handlebars;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    hash::Hash,
};

/* lazy_static! {
    static ref TEMPLATE_KEY: Regex = Regex::new(r"(?<=\${)\w+(?=})").unwrap();
} */

// Use this instead? https://crates.io/crates/handlebars
pub fn substitution(
    input: String,
    vars: Vec<HashMap<String, String>>,
) -> Result<String, SubstitutionError> {
    let vars: HashMap<String, String> = merge(vars);
    let mut reg = Handlebars::new();
    reg.register_template_string("template", input).unwrap();
    let output: String = reg.render("template", &vars).unwrap();
    Ok(output)
}

#[derive(Debug)]
pub enum SubstitutionError {
    MissingValue(String),
}

fn merge(maps: Vec<HashMap<String, String>>) -> HashMap<String, String> {
    maps.into_iter()
        .rev()
        .reduce(|mut acc, next| {
            acc.extend(next);
            acc
        })
        .unwrap_or_default()
}

fn build_map(
    keys: HashSet<String>,
    values: HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    let map: HashMap<String, String> = keys
        .iter()
        .map(|key| match values.get(key) {
            Some(value) => Ok((key.clone(), value.clone())),
            None => return Err(key),
        })
        .filter_map(|kv| kv.ok())
        .collect();

    Ok(map)
}
