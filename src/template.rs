use handlebars::Handlebars;
use std::collections::HashMap;

use crate::prop::Property;

pub fn substitution(input: String, vars: Vec<Property>) -> Result<String, SubstitutionError> {
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

fn merge(maps: Vec<Property>) -> HashMap<String, String> {
    maps.into_iter()
        .rev()
        .map(|prop| (prop.key().to_string(), prop.value().to_string()))
        .collect()
}
