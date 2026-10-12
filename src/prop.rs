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

pub fn from_file(path: &Path, execute_shell: bool) -> Result<Vec<Property>, ParsePropertyError> {
    let content: String = std::fs::read_to_string(path)?;
    let source: Source = source(path);

    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Property::from_str(line).map(|property| property.with_source(source)))
        .filter_map(|property| match property {
            Ok(property) if !execute_shell && shell_command(property.value()).is_some() => None,
            property => Some(property.and_then(resolve_command)),
        })
        .collect()
}

fn shell_command(value: &str) -> Option<&str> {
    value.strip_prefix("$(").and_then(|value| value.strip_suffix(')'))
}

fn resolve_command(mut property: Property) -> Result<Property, ParsePropertyError> {
    if let Some(command) = shell_command(property.value()) {
        let output: std::process::Output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stderr(std::process::Stdio::null())
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

        let properties: Vec<Property> = super::from_file(&path, true)?;
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
        std::fs::write(
            &path,
            format!(
                "id=$(printf x >> '{}'; printf generated)\ncopy=$(cat '{}'; printf y >> '{}')",
                marker.display(),
                marker.display(),
                marker.display()
            ),
        )?;

        let properties: Vec<Property> = super::from_file(&path, true)?;
        assert_eq!(properties[0].value(), "generated");
        assert_eq!(properties[1].value(), "x");
        let rendered: String = crate::template::substitution(
            "https://example.com/{{id}} {\"ref\":\"{{id}}\"} {{copy}}".to_string(),
            properties,
            false,
            false,
            false,
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(rendered, "https://example.com/generated {\"ref\":\"generated\"} x");
        assert_eq!(std::fs::read_to_string(marker)?, "xy");
        Ok(())
    }

    fn shell_path() -> Result<String, Box<dyn std::error::Error>> {
        // Git Bash converts native Windows PATH entries and adds shell paths.
        let output: std::process::Output = std::process::Command::new("sh")
            .args(["-c", "printf '%s' \"$PATH\""])
            .output()?;
        if !output.status.success() {
            return Err("Failed to read shell PATH".into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    #[test]
    fn commands_inherit_system_values_not_earlier_file_assignments(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let inherited_path: String = shell_path()?;
        let key: &str = "FIRE_TEST_EARLIER_ASSIGNMENT_32";
        let inherited_value: String = std::env::var(key).unwrap_or_default();
        [".env", ".sec"].into_iter().try_for_each(
            |filename: &str| -> Result<(), Box<dyn std::error::Error>> {
                let path: std::path::PathBuf = directory.path().join(filename);
                std::fs::write(&path, format!(
                    "PATH={inherited_path}:/fire-file-only-path\n{key}=static-file-value\nstatic_copy=$(printf '%s' \"${key}\")\n{key}=$(printf command-file-value)\ncommand_copy=$(printf '%s' \"${key}\")\npath=$(printf '%s' \"$PATH\")"
                ))?;
                let properties: Vec<Property> = super::from_file(&path, true)?;
                assert_eq!(properties[2].value(), inherited_value);
                assert_eq!(properties[4].value(), inherited_value);
                assert_eq!(properties[5].value(), inherited_path);
                Ok(())
            },
        )?;
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
            "printf a)-$(printf b",
        ]
        .into_iter()
        .try_for_each(|command: &str| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::write(
                &path,
                format!("token=$({command})\nlater=$(printf x > '{}')", marker.display()),
            )?;
            let error: ParsePropertyError = super::from_file(&path, true).unwrap_err();
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
        let properties: Vec<Property> = super::from_file(&path, true)?;
        assert_eq!(properties[0].value(), "HELLO WORLD");
        assert_eq!(properties[1].value(), "");
        assert_eq!(properties[2].value(), shell_path()?);
        Ok(())
    }

    #[test]
    fn only_whole_file_values_execute_commands() -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let path: std::path::PathBuf = directory.path().join(".env");
        std::fs::write(&path, "foo=bar\nquoted='some value'\nembedded=prefix-$(printf generated)\nincomplete=$(printf generated\n")?;
        let properties: Vec<Property> = super::from_file(&path, true)?;
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
        let mut properties: Vec<Property> = super::from_file(&path, true)?;
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
        fn noisy_stderr_is_discarded_without_changing_stdout_or_exit_status(
            diagnostic_bytes in 65536usize..131073,
            exit_status in 1u8..=255,
        ) {
            let directory: tempfile::TempDir = tempfile::tempdir().unwrap();
            let path: std::path::PathBuf = directory.path().join(".sec");
            // P9R: output() used to buffer stderr even though Fire never uses it.
            // Noisy commands could exhaust memory while returning a tiny value.
            // Check that stderr goes directly to /dev/null: checking only the
            // returned value would also pass with the old, unbounded buffer.
            let command: String = format!(
                "test /dev/stderr -ef /dev/null || exit 99; printf '%{diagnostic_bytes}s' diagnostic >&2; printf token"
            );
            std::fs::write(&path, format!("token=$({command})")).unwrap();
            let properties: Vec<Property> = super::from_file(&path, true).unwrap();
            proptest::prop_assert_eq!(properties[0].value(), "token");

            // Discarding diagnostics must not hide an unsuccessful exit.
            std::fs::write(&path, format!("token=$({command}; exit {exit_status})")).unwrap();
            let error: ParsePropertyError = super::from_file(&path, true).unwrap_err();
            proptest::prop_assert!(matches!(error, ParsePropertyError::Command(key) if key == "token"));
        }

        #[test]
        fn disabling_shell_is_equivalent_to_removing_command_assignments(
            commands in proptest::collection::vec("[A-Za-z0-9 _=$(){};|&<>]{0,64}", 1..8),
        ) {
            let directory: tempfile::TempDir = tempfile::tempdir().unwrap();
            let path: std::path::PathBuf = directory.path().join(".sec");
            let static_lines: String = commands.iter().enumerate()
                .map(|(index, command)| format!("literal_{index}=prefix-$({command})\n"))
                .collect();
            let command_lines: String = commands.iter().enumerate()
                .map(|(index, command)| format!("dynamic_{index}=\"$({command})\"\n"))
                .collect();
            let literals: &str = "static_value=kept\nincomplete=$(printf incomplete\n";
            std::fs::write(&path, format!("{command_lines}{literals}{static_lines}{command_lines}")).unwrap();
            let disabled: Vec<Property> = super::from_file(&path, false).unwrap();
            std::fs::write(&path, format!("{literals}{static_lines}")).unwrap();
            let omitted: Vec<Property> = super::from_file(&path, true).unwrap();
            proptest::prop_assert_eq!(disabled, omitted);
        }

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
                "payload=\"{value}\"\nvalue=$(printf '%s\\n%s{suffix}' '{value}' '{value}')"
            )).unwrap();
            let properties: Vec<Property> = super::from_file(&path, true).unwrap();
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
