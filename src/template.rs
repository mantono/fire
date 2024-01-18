use handlebars::RenderError;
use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;
use std::collections::HashSet;
use termcolor::ColorChoice;

use crate::{prop::Property, templ};

pub fn substitution(
    input: String,
    vars: Vec<Property>,
    interactive: bool,
    use_colors: bool,
) -> Result<String, SubstitutionError> {
    let keys: HashSet<String> = templ::find_keys(&input);
    let vars: HashMap<String, String> = resolve_values(interactive, use_colors, keys, merge(vars))?;
    let mut reg = Handlebars::new();
    reg.register_escape_fn(no_escape);
    reg.set_strict_mode(true);
    reg.register_template_string("template", input).unwrap();
    reg.render("template", &vars).map_err(|_| SubstitutionError::Rendering)
}

fn resolve_values(
    interactive: bool,
    use_colors: bool,
    keys: HashSet<String>,
    vars: HashMap<String, String>,
) -> Result<HashMap<String, String>, SubstitutionError> {
    let diff: HashSet<String> = keys
        .difference(&vars.clone().into_keys().collect())
        .into_iter()
        .map(|x| x.clone())
        .collect();

    if diff.is_empty() {
        Ok(vars)
    } else if interactive {
        let mut added: HashMap<String, String> = HashMap::with_capacity(diff.len());
        let theme = dialoguer::theme::ColorfulTheme::default();
        let mut input = if use_colors {
            dialoguer::Input::with_theme(&theme)
        } else {
            dialoguer::Input::new()
        };
        for key in diff {
            let value: String =
                input.with_prompt(key.clone()).allow_empty(false).interact_text().unwrap();
            added.insert(key, value);
        }
        let all = vars.into_iter().chain(added).collect();
        Ok(all)
    } else {
        let missing: String = diff.into_iter().next().unwrap();
        Err(SubstitutionError::MissingValue(missing))
    }
}

#[derive(Debug)]
pub enum SubstitutionError {
    MissingValue(String),
    Rendering,
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
