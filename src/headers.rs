use std::str::FromStr;

use serde::Deserialize;

pub type Header = (Key, Value);

#[derive(Debug, Clone)]
pub enum Error {
    Input(String),
    Key(String),
    Value(String),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct Key(String);

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Key(s.to_ascii_lowercase()))
    }
}

impl Key {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Value(String);

impl FromStr for Value {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Value(s.trim().to_string()))
    }
}

impl Value {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn header(key: &str, value: &str) -> Result<Header, Error> {
    let key: Key = match Key::from_str(key) {
        Ok(key) => key,
        Err(()) => return Err(Error::Key(key.to_string())),
    };

    let value: Value = match Value::from_str(value) {
        Ok(value) => value,
        Err(()) => return Err(Error::Value(value.to_string())),
    };

    Ok((key, value))
}
