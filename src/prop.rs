use std::cmp::Ordering;
use std::{fmt::Display, path::Path, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    key: String,
    value: String,
    source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    EnvVar,
    File(usize),
    Arg,
}

impl PartialOrd for Source {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Source {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Source::EnvVar, Source::EnvVar) => Ordering::Equal,
            (Source::Arg, Source::Arg) => Ordering::Equal,
            (Source::File(d0), Source::File(d1)) => d1.cmp(d0),
            (Source::File(_), Source::EnvVar) => Ordering::Less,
            (Source::EnvVar, _) => Ordering::Greater,
            (Source::File(_), Source::Arg) => Ordering::Greater,
            (Source::Arg, _) => Ordering::Less,
        }
    }
}

impl Property {
    pub fn new(key: String, value: String, source: Source) -> Result<Property, ParsePropertyError> {
        Ok(Property { key, value, source })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn with_source(self, source: Source) -> Self {
        Property { source, ..self }
    }
}

impl Ord for Property {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source.cmp(&other.source)
    }
}

impl PartialOrd for Property {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn from_file(path: &Path) -> Result<Vec<Property>, ParsePropertyError> {
    let content: String = std::fs::read_to_string(path)?;
    let source: Source = source(path);

    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(Property::from_str)
        .try_fold(Vec::new(), |mut properties: Vec<Property>, property| {
            let resolved: Property = resolve_command(property?.with_source(source), &properties)?;
            properties.push(resolved);
            Ok(properties)
        })
}

fn resolve_command(
    mut property: Property,
    previous: &[Property],
) -> Result<Property, ParsePropertyError> {
    if let Some(command) =
        property.value.strip_prefix("$(").and_then(|value| value.strip_suffix(')'))
    {
        let output: std::process::Output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .envs(previous.iter().map(|property| (property.key(), property.value())))
            .output()
            .map_err(|_| ParsePropertyError::Command(property.key.clone()))?;
        if !output.status.success() {
            return Err(ParsePropertyError::Command(property.key));
        }
        let value: String = String::from_utf8(output.stdout)
            .map_err(|_| ParsePropertyError::Command(property.key.clone()))?;
        property.value = value.trim_end_matches('\n').to_string();
    }
    Ok(property)
}

fn source(path: &Path) -> Source {
    let depth: usize = path.components().count();
    Source::File(depth)
}

#[derive(Debug)]
pub enum ParsePropertyError {
    Entry(String),
    Key(String),
    Value(String),
    File(String),
    Command(String),
}

impl From<std::io::Error> for ParsePropertyError {
    fn from(e: std::io::Error) -> Self {
        ParsePropertyError::File(e.to_string())
    }
}

impl Display for ParsePropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsePropertyError::Entry(entry) => write!(f, "Invalid entry: {}", entry),
            ParsePropertyError::Key(key) => write!(f, "Invalid key: {}", key),
            ParsePropertyError::Value(value) => write!(f, "Invalid value: {}", value),
            ParsePropertyError::File(file) => write!(f, "Invalid value: {}", file),
            ParsePropertyError::Command(key) => {
                write!(f, "Command assignment failed for variable: {key}")
            }
        }
    }
}

impl std::error::Error for ParsePropertyError {}

const DELIMITER: char = '=';

impl std::str::FromStr for Property {
    type Err = ParsePropertyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(DELIMITER) {
            Some((key, value)) => Property::new(normalize(key), normalize(value), Source::EnvVar),
            None => Err(ParsePropertyError::Entry(s.to_string())),
        }
    }
}

fn normalize(input: &str) -> String {
    let b: &[_] = &['\'', '"'];
    input.trim().trim_matches(b).to_string()
}

impl TryFrom<(String, String)> for Property {
    type Error = ParsePropertyError;

    fn try_from(value: (String, String)) -> Result<Self, Self::Error> {
        Property::new(value.0, value.1, Source::EnvVar)
    }
}

#[cfg(test)]
mod tests {
    use crate::prop::Source;

    use super::{ParsePropertyError, Property};

    #[test]
    fn command_assignment_is_resolved_when_file_is_loaded() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".env");
        std::fs::write(&path, "id=$(printf generated)")?;

        let properties: Vec<Property> = super::from_file(&path)?;
        assert_eq!(properties[0].key(), "id");
        assert_eq!(properties[0].value(), "generated");
        assert_eq!(properties[0].source, Source::File(path.components().count()));
        Ok(())
    }

    #[test]
    fn assignments_run_once_in_order_and_reuse_the_resolved_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".sec");
        let marker: std::path::PathBuf = directory.path().join("calls");
        std::fs::write(&path, format!(
            "marker={}\nseed=first\nid=$(printf x >> \"$marker\"; printf '%s-generated' \"$seed\")\nseed=second\ncopy=$(printf '%s/%s' \"$id\" \"$seed\")",
            marker.display()
        ))?;

        let properties: Vec<Property> = super::from_file(&path)?;
        assert_eq!(properties[2].value(), "first-generated");
        assert_eq!(properties[4].value(), "first-generated/second");
        let rendered: String = crate::template::substitution(
            "https://example.com/{{id}} {\"ref\":\"{{id}}\"} {{copy}}".to_string(),
            properties,
            false,
            false,
            false,
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(rendered, "https://example.com/first-generated {\"ref\":\"first-generated\"} first-generated/second");
        assert_eq!(std::fs::read_to_string(marker)?, "x");
        Ok(())
    }

    #[test]
    fn command_failures_stop_loading_without_exposing_secrets(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".sec");
        let marker: std::path::PathBuf = directory.path().join("later");
        [
            "printf secret; printf secret >&2; exit 7",
            "fire_nonexistent_command_32",
            "printf '\\377'",
            "(",
        ]
        .into_iter()
        .try_for_each(|command: &str| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::write(
                &path,
                format!(
                    "marker={}\ntoken=$({command})\nlater=$(printf x > \"$marker\")",
                    marker.display()
                ),
            )?;
            let error: ParsePropertyError = super::from_file(&path).unwrap_err();
            assert!(matches!(&error, ParsePropertyError::Command(key) if key == "token"));
            assert_eq!(error.to_string(), "Command assignment failed for variable: token");
            assert!(!format!("{error:?}").contains("secret"));
            assert!(!marker.exists());
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn shell_supports_quotes_pipes_and_nested_substitution(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join("development.env");
        std::fs::write(&path, "value=\"$(printf '%s' \"$(printf 'hello world')\" | tr 'a-z' 'A-Z')\"\nempty=$(printf '')\npath=$(printf '%s' \"$PATH\")")?;
        let properties: Vec<Property> = super::from_file(&path)?;
        assert_eq!(properties[0].value(), "HELLO WORLD");
        assert_eq!(properties[1].value(), "");
        assert_eq!(properties[2].value(), std::env::var("PATH")?);
        Ok(())
    }

    #[test]
    fn only_whole_file_values_execute_commands() -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".env");
        std::fs::write(&path, "foo=bar\nquoted='some value'\nembedded=prefix-$(printf generated)\nincomplete=$(printf generated\n")?;
        let properties: Vec<Property> = super::from_file(&path)?;
        assert_eq!(
            properties.iter().map(Property::value).collect::<Vec<&str>>(),
            vec![
                "bar",
                "some value",
                "prefix-$(printf generated)",
                "$(printf generated"
            ]
        );
        let argument: Property = "id=$(printf generated)".parse()?;
        let environment: Property =
            Property::try_from(("id".to_string(), "$(printf generated)".to_string()))?;
        assert_eq!(argument.value(), "$(printf generated)");
        assert_eq!(environment.value(), "$(printf generated)");
        Ok(())
    }

    #[test]
    fn dynamic_values_keep_existing_source_precedence() -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".env");
        std::fs::write(&path, "id=$(printf generated)")?;
        let mut properties: Vec<Property> = super::from_file(&path)?;
        properties.push(Property::new("id".to_string(), "system".to_string(), Source::EnvVar)?);
        let render = |properties: Vec<Property>| {
            crate::template::substitution("{{id}}".to_string(), properties, false, false, false)
                .map_err(|error| format!("{error:?}"))
        };
        assert_eq!(render(properties.clone())?, "generated");
        properties.push(Property::new("id".to_string(), "argument".to_string(), Source::Arg)?);
        assert_eq!(render(properties)?, "argument");
        Ok(())
    }

    proptest::proptest! {
        #[test]
        fn command_output_preserves_data_and_only_strips_trailing_newlines(
            payload in "[A-Za-z0-9 _=$(){};|&<>]{0,64}",
            newlines in 0usize..6,
        ) {
            let directory: tempfile::TempDir = tempfile::tempdir().unwrap();
            let path: std::path::PathBuf = directory.path().join(".sec");
            let value: String = format!("  start-{payload}-end  ");
            let suffix: String = "\\n".repeat(newlines);
            std::fs::write(&path, format!(
                "payload=\"{value}\"\nvalue=$(printf '%s\\n%s{suffix}' \"$payload\" \"$payload\")"
            )).unwrap();
            let properties: Vec<Property> = super::from_file(&path).unwrap();
            proptest::prop_assert_eq!(properties[0].value(), value.as_str());
            proptest::prop_assert_eq!(properties[1].value(), format!("{value}\n{value}"));
        }
    }

    #[test]
    fn test_properties_sort_order() -> Result<(), ParsePropertyError> {
        let env_var = Property::new("key".to_string(), "env_var".to_string(), Source::EnvVar)?;
        let file_root = Property::new("key".to_string(), "file_root".to_string(), Source::File(0))?;
        let file_child =
            Property::new("key".to_string(), "file_child".to_string(), Source::File(1))?;
        let arg_var = Property::new("key".to_string(), "arg".to_string(), Source::Arg)?;

        let mut props: Vec<Property> = vec![file_root, file_child, arg_var, env_var];
        props.sort();

        assert_eq!("arg", props[0].value());
        assert_eq!("file_child", props[1].value());
        assert_eq!("file_root", props[2].value());
        assert_eq!("env_var", props[3].value());

        Ok(())
    }
}
