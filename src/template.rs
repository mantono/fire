use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;

use crate::prop::Property;

pub fn substitution(input: String, vars: Vec<Property>) -> Result<String, SubstitutionError> {
    let vars: HashMap<String, String> = merge(vars);
    let mut reg = Handlebars::new();
    reg.register_escape_fn(no_escape);
    reg.set_strict_mode(true);
    reg.register_template_string("template", input).unwrap();
    match reg.render("template", &vars) {
        Ok(output) => Ok(output),
        Err(e) => Err(SubstitutionError::MissingValue(e.desc)),
    }
}

#[derive(Debug)]
pub enum SubstitutionError {
    MissingValue(String),
}

fn merge(mut maps: Vec<Property>) -> HashMap<String, String> {
    maps.sort();
    maps.into_iter()
        .map(|prop| (prop.key().to_string(), prop.value().to_string()))
        .collect()
}
