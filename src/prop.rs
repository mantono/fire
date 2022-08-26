use std::fmt::Write;
use std::{fmt::Display, path::Path, str::FromStr};

#[derive(Debug, Clone)]
pub struct Property {
    key: String,
    value: String,
}

impl Property {
    pub fn new(key: String, value: String) -> Result<Property, ParsePropertyError> {
        Ok(Property { key, value })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

pub fn from_file(path: &Path) -> Result<Vec<Property>, ParsePropertyError> {
    let content: String = std::fs::read_to_string(path)?;
    let props: Vec<Property> = content
        .lines()
        .into_iter()
        .map(|line| Property::from_str(line).unwrap())
        .collect();

    Ok(props)
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
            Some((key, value)) => Property::new(normalize(key), normalize(value)),
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
        Property::new(value.0, value.1)
    }
}
