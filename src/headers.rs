use std::str::FromStr;

use serde::Deserialize;

pub type Header = (HeaderKey, HeaderValue);

#[derive(Debug, Clone)]
pub enum HeaderError {
    Input(String),
    Key(String),
    Value(String),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct HeaderKey(String);

impl FromStr for HeaderKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HeaderKey(s.to_ascii_lowercase()))
    }
}

impl HeaderKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderValue(String);

impl FromStr for HeaderValue {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HeaderValue(s.trim().to_string()))
    }
}

impl HeaderValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn header(key: &str, value: &str) -> Result<Header, HeaderError> {
    let key: HeaderKey = match HeaderKey::from_str(key) {
        Ok(key) => key,
        Err(()) => return Err(HeaderError::Key(key.to_string())),
    };

    let value: HeaderValue = match HeaderValue::from_str(value) {
        Ok(value) => value,
        Err(()) => return Err(HeaderError::Value(value.to_string())),
    };

    Ok((key, value))
}
