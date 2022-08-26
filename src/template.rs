use handlebars::Handlebars;
use std::collections::HashMap;

use crate::prop::Property;

pub fn substitution(input: String, vars: Vec<Property>) -> Result<String, SubstitutionError> {
    let vars: HashMap<String, String> = merge(vars);
    let mut reg = Handlebars::new();
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

fn merge(maps: Vec<Property>) -> HashMap<String, String> {
    maps.into_iter()
        .map(|prop| (prop.key().to_string(), prop.value().to_string()))
        .collect()
}
