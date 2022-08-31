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

    let vars: HashMap<String, String> = maps
        .into_iter()
        .rev()
        .map(|prop| (prop.key().to_string(), prop.value().to_string()))
        .collect();

    log::debug!("Resolved properties: {:?}", vars);

    vars
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::prop::{ParsePropertyError, Property, Source};

    use super::merge;

    #[test]
    fn test_merge_properties() -> Result<(), ParsePropertyError> {
        let props: Vec<Property> = vec![
            Property::new(String::from("key"), String::from("file0"), Source::File(0))?,
            Property::new(String::from("key"), String::from("file1"), Source::File(1))?,
            Property::new(String::from("key"), String::from("env"), Source::EnvVar)?,
            Property::new(String::from("key"), String::from("arg"), Source::Arg)?,
        ];

        let vars: HashMap<String, String> = merge(props);
        assert_eq!("arg", vars["key"]);

        Ok(())
    }
}
