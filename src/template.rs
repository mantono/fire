use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::prop::Property;
use crate::runner::{self, RunnerError};
use crate::templ::{self, Fallback, Reference};

pub fn substitution(
    input: String,
    vars: Vec<Property>,
    interactive: bool,
    use_colors: bool,
    trim: bool,
    allow_command_fallbacks: bool,
) -> Result<String, SubstitutionError> {
    let refs: Vec<Reference> = templ::scan(&input);
    let props: HashMap<String, String> = merge(vars);

    let mut render_vars: HashMap<String, String> = props.clone();
    let mut occupied_keys: HashSet<String> = props.keys().cloned().collect();
    occupied_keys.extend(refs.iter().map(|reference: &Reference| reference.name.clone()));
    let mut rewrites: Vec<Option<String>> = Vec::with_capacity(refs.len());
    let mut prompt_names: HashSet<String> = HashSet::new();
    let mut missing: Option<String> = None;

    for reference in &refs {
        let resolved: Option<String> = match props.get(&reference.name).cloned() {
            Some(value) => Some(value),
            None => match &reference.fallback {
                Fallback::None => None,
                Fallback::Static(value) => Some(value.clone()),
                Fallback::Command(command) => {
                    Some(resolve_command_fallback(command, allow_command_fallbacks)?)
                }
            },
        };

        match (&reference.fallback, resolved) {
            (Fallback::None, Some(value)) => {
                render_vars.insert(reference.name.clone(), value);
                rewrites.push(None);
            }
            (Fallback::None, None) => {
                prompt_names.insert(reference.name.clone());
                rewrites.push(None);
            }
            (_, Some(value)) => {
                let key: String = internal_key(&mut occupied_keys);
                render_vars.insert(key.clone(), value);
                rewrites.push(Some(key));
            }
            (_, None) => {
                missing.get_or_insert_with(|| reference.name.clone());
                rewrites.push(None);
            }
        }
    }

    if let Some(name) = missing {
        return Err(SubstitutionError::MissingValue(name));
    }

    if !prompt_names.is_empty() {
        if interactive {
            let prompted: HashMap<String, String> = prompt_for(prompt_names, use_colors, trim);
            render_vars.extend(prompted);
        } else {
            let name: String = prompt_names.into_iter().next().unwrap();
            return Err(SubstitutionError::MissingValue(name));
        }
    }

    let template: String = rewrite_template(&input, &refs, &rewrites);

    let mut reg = Handlebars::new();
    reg.register_escape_fn(no_escape);
    reg.set_strict_mode(true);
    reg.register_template_string("template", template).unwrap();
    reg.render("template", &render_vars).map_err(|_| SubstitutionError::Rendering)
}

/// Resolve a dynamic (command) fallback, gated by `allow`.
///
/// The command is never invoked unless `allow` is `true`; reaching this
/// point without permission yields a distinct, deterministic error instead
/// of attempting execution.
fn resolve_command_fallback(command: &str, allow: bool) -> Result<String, SubstitutionError> {
    if !allow {
        return Err(SubstitutionError::CommandFallbackNotAllowed);
    }

    runner::run_command(command).map_err(SubstitutionError::CommandFallbackFailed)
}

fn internal_key(occupied: &mut HashSet<String>) -> String {
    (0_usize..)
        .map(|index: usize| format!("__fire_ref_{index}"))
        .find(|candidate: &String| occupied.insert(candidate.clone()))
        .unwrap()
}

/// Rebuild `input` with every extended-fallback occurrence's span replaced by
/// its unique internal Handlebars key, leaving plain occurrences untouched so
/// existing normal-template rendering behavior is preserved.
fn rewrite_template(input: &str, refs: &[Reference], rewrites: &[Option<String>]) -> String {
    let mut output: String = String::with_capacity(input.len());
    let mut cursor: usize = 0;

    for (reference, rewrite) in refs.iter().zip(rewrites.iter()) {
        output.push_str(&input[cursor..reference.span.start]);
        match rewrite {
            Some(key) => {
                output.push_str("{{");
                output.push_str(key);
                output.push_str("}}");
            }
            None => output.push_str(&input[reference.span.start..reference.span.end]),
        }
        cursor = reference.span.end;
    }

    output.push_str(&input[cursor..]);
    output
}

fn prompt_for(names: HashSet<String>, use_colors: bool, trim: bool) -> HashMap<String, String> {
    let mut added: HashMap<String, String> = HashMap::with_capacity(names.len());
    let theme = dialoguer::theme::ColorfulTheme::default();

    for name in names {
        let value: String = if use_colors {
            dialoguer::Input::with_theme(&theme)
                .with_prompt(name.clone())
                .allow_empty(false)
                .interact_text()
                .unwrap()
        } else {
            dialoguer::Input::new()
                .with_prompt(name.clone())
                .allow_empty(false)
                .interact_text()
                .unwrap()
        };

        let value: String = if trim { value.trim().into() } else { value };

        added.insert(name, value);
    }

    added
}

#[derive(Debug)]
pub enum SubstitutionError {
    MissingValue(String),
    Rendering,
    CommandFallbackNotAllowed,
    CommandFallbackFailed(RunnerError),
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

    use super::{merge, substitution, SubstitutionError};

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

    #[test]
    fn static_fallback_used_when_property_absent() {
        let input = String::from("{{FOO:bar}}");
        let result = substitution(input, vec![], false, false, false, false).unwrap();
        assert_eq!("bar", result);
    }

    #[test]
    fn sourced_value_overrides_static_fallback_for_every_source() -> Result<(), ParsePropertyError>
    {
        let sources = [
            Source::Arg,
            Source::EnvVar,
            Source::File(0),
            Source::File(1),
        ];

        for source in sources {
            let prop = Property::new(String::from("FOO"), String::from("value"), source)?;
            let input = String::from("{{FOO:bar}}");
            let result = substitution(input, vec![prop], false, false, false, false).unwrap();
            assert_eq!("value", result, "source {:?} did not override fallback", source);
        }

        Ok(())
    }

    #[test]
    fn sourced_value_overrides_command_fallback_for_every_source() -> Result<(), ParsePropertyError>
    {
        let sources = [
            Source::Arg,
            Source::EnvVar,
            Source::File(0),
            Source::File(1),
        ];

        for source in sources {
            let prop = Property::new(String::from("FOO"), String::from("value"), source)?;
            let input = String::from("{{FOO|echo bar}}");
            let result = substitution(input, vec![prop], false, false, false, false).unwrap();
            assert_eq!("value", result, "source {:?} did not override fallback", source);
        }

        Ok(())
    }

    #[test]
    fn empty_sourced_value_overrides_fallback() -> Result<(), ParsePropertyError> {
        let prop = Property::new(String::from("FOO"), String::new(), Source::Arg)?;
        let input = String::from("{{FOO:bar}}");
        let result = substitution(input, vec![prop], false, false, false, false).unwrap();
        assert_eq!("", result);

        Ok(())
    }

    #[test]
    fn per_occurrence_fallbacks_resolve_independently_when_missing() {
        let input = String::from("{{FOO:a}} {{FOO:b}}");
        let result = substitution(input, vec![], false, false, false, false).unwrap();
        assert_eq!("a b", result);
    }

    #[test]
    fn per_occurrence_supplied_value_replaces_every_occurrence() -> Result<(), ParsePropertyError> {
        let prop = Property::new(String::from("FOO"), String::from("value"), Source::Arg)?;
        let input = String::from("{{FOO}} {{FOO:a}} {{FOO:b}}");
        let result = substitution(input, vec![prop], false, false, false, false).unwrap();
        assert_eq!("value value value", result);

        Ok(())
    }

    #[test]
    fn fallback_occurrence_never_triggers_interactive_prompt() {
        // interactive is true, but the only occurrence has a static fallback, so no
        // prompt must be attempted (which would otherwise hang/panic in a test
        // without an interactive terminal attached).
        let input = String::from("{{FOO:bar}}");
        let result = substitution(input, vec![], true, false, false, false).unwrap();
        assert_eq!("bar", result);
    }

    #[test]
    fn legacy_missing_value_behavior_is_preserved() {
        let input = String::from("{{FOO}}");
        let err = substitution(input, vec![], false, false, false, false).unwrap_err();
        match err {
            SubstitutionError::MissingValue(name) => assert_eq!("FOO", name),
            other => panic!("expected MissingValue, got {:?}", other),
        }
    }

    #[test]
    fn handlebars_blocks_receive_supplied_properties() -> Result<(), ParsePropertyError> {
        let enabled: Property =
            Property::new(String::from("ENABLED"), String::from("true"), Source::Arg)?;
        let input: String = String::from("{{#if ENABLED}}enabled{{/if}}");

        let result: String = substitution(input, vec![enabled], false, false, false, false).unwrap();

        assert_eq!("enabled", result);
        Ok(())
    }

    #[test]
    fn fallback_rewrite_does_not_overwrite_a_supplied_internal_name(
    ) -> Result<(), ParsePropertyError> {
        let supplied: Property =
            Property::new(String::from("__fire_ref_1"), String::from("provided"), Source::Arg)?;
        let input: String = String::from("{{__fire_ref_1}} {{FOO:bar}}");

        let result: String =
            substitution(input, vec![supplied], false, false, false, false).unwrap();

        assert_eq!("provided bar", result);
        Ok(())
    }

    #[test]
    fn supplied_internal_name_does_not_overwrite_a_fallback() -> Result<(), ParsePropertyError> {
        let supplied: Property =
            Property::new(String::from("__fire_ref_0"), String::from("provided"), Source::Arg)?;
        let input: String = String::from("{{FOO:bar}} {{__fire_ref_0}}");

        let result: String =
            substitution(input, vec![supplied], false, false, false, false).unwrap();

        assert_eq!("bar provided", result);
        Ok(())
    }

    #[test]
    fn sourced_value_bypasses_authorization_and_runner_for_command_fallback(
    ) -> Result<(), ParsePropertyError> {
        // If this fallback command were actually invoked, it would fail (the binary does not
        // exist), regardless of the `--allow-command-fallbacks` permission. A sourced value must
        // short-circuit both the authorization check and the runner entirely.
        let prop = Property::new(String::from("FOO"), String::from("value"), Source::Arg)?;
        let input = String::from("{{FOO|definitely-not-a-real-command-xyz}}");

        let result =
            substitution(input.clone(), vec![prop.clone()], false, false, false, false).unwrap();
        assert_eq!("value", result);

        let result = substitution(input, vec![prop], false, false, false, true).unwrap();
        assert_eq!("value", result);

        Ok(())
    }

    #[test]
    fn command_fallback_denied_without_permission_yields_distinct_error() {
        let input = String::from("{{FOO|echo bar}}");
        let err = substitution(input, vec![], false, false, false, false).unwrap_err();
        match err {
            SubstitutionError::CommandFallbackNotAllowed => {}
            other => panic!("expected CommandFallbackNotAllowed, got {:?}", other),
        }
    }

    #[test]
    fn command_fallback_executes_when_allowed() {
        let input = String::from("{{FOO|echo bar}}");
        let result = substitution(input, vec![], false, false, false, true).unwrap();
        assert_eq!("bar", result);
    }
}
