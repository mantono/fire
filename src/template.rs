use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;
use termcolor::ColorChoice;

use crate::prop::Property;

pub fn substitution(
    input: String,
    vars: Vec<Property>,
    interactive: bool,
    use_colors: bool,
) -> Result<String, SubstitutionError> {
    let vars: HashMap<String, String> = merge(vars);
    let mut reg = Handlebars::new();
    reg.register_escape_fn(no_escape);
    reg.set_strict_mode(true);
    reg.register_template_string("template", input).unwrap();
    render(reg, vars, interactive, use_colors)
}

// TODO: Write own "handlebar" template renderer that can return missing key on failed rendering
// instead of crate handlebar.

fn render(
    template: Handlebars,
    mut vars: HashMap<String, String>,
    interactive: bool,
    use_colors: bool,
) -> Result<String, SubstitutionError> {
    match template.render("template", &vars) {
        Ok(output) => Ok(output),
        Err(e) => match interactive {
            false => Err(SubstitutionError::MissingValue(e.desc)),
            true => {
                let column = e.column_no.unwrap_or(0);
                let line = e.line_no.unwrap_or(0);
                //template.render_template(template_string, data)
                log::info!("Missing at line {} col {}", line, column);
                let value: String = ask("foo", use_colors);
                vars.insert(String::from("foo"), value);
                render(template, vars, interactive, use_colors)
            }
        },
    }
}

fn ask(key: &str, use_colors: bool) -> String {
    "bar".to_string()
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
