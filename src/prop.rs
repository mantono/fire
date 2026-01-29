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
        .map(|prop| prop.map(|p| p.with_source(source)))
        .collect()
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
