use std::fmt::Write;
use std::{fmt::Display, path::Path, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Ord)]
pub struct Property {
    key: String,
    value: String,
    prio: i8,
}

pub const HIGHEST_PRIO: i8 = -127;
pub const DEFAULT_PRIO: i8 = 0;
pub const LOWEST_PRIO: i8 = 127;

impl Property {
    pub fn new(key: String, value: String, prio: i8) -> Result<Property, ParsePropertyError> {
        Ok(Property { key, value, prio })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn with_prio(self, prio: i8) -> Self {
        Property { prio, ..self }
    }
}

impl std::cmp::PartialOrd for Property {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.prio.cmp(&other.prio))
    }
}

pub fn from_file(path: &Path) -> Result<Vec<Property>, ParsePropertyError> {
    let content: String = std::fs::read_to_string(path)?;
    let prio: i8 = priority(path);
    let props: Vec<Property> = content
        .lines()
        .into_iter()
        .map(|line| Property::from_str(line).unwrap())
        .map(|prop| prop.with_prio(prio))
        .collect();

    Ok(props)
}

fn priority(path: &Path) -> i8 {
    let depth: usize = path.components().count();
    let delta: usize = depth * 2;
    // `.env` should have lower priority than `dev.env`
    let adj_delta: usize = match path.file_name() {
        Some(_) => delta + 1,
        None => delta,
    };

    DEFAULT_PRIO - (adj_delta.clamp(0, 126) as i8)
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
            Some((key, value)) => Property::new(normalize(key), normalize(value), DEFAULT_PRIO),
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
        Property::new(value.0, value.1, DEFAULT_PRIO)
    }
}

#[cfg(test)]
mod tests {
    use super::{ParsePropertyError, Property, DEFAULT_PRIO, HIGHEST_PRIO, LOWEST_PRIO};

    #[test]
    fn test_properties_sort_order() -> Result<(), ParsePropertyError> {
        let prop0 = Property::new("key".to_string(), "v0".to_string(), HIGHEST_PRIO)?;
        let prop1 = Property::new("key".to_string(), "v1".to_string(), DEFAULT_PRIO)?;
        let prop2 = Property::new("key".to_string(), "v2".to_string(), LOWEST_PRIO)?;

        let mut props: Vec<Property> = vec![prop2, prop1, prop0];
        props.sort();

        assert_eq!("v0", props[0].value());
        assert_eq!("v1", props[1].value());
        assert_eq!("v2", props[2].value());

        Ok(())
    }
}
