use std::{collections::HashMap, fmt::Display, str::FromStr};

use reqwest::header::HeaderMap;
use serde::Deserialize;

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

pub trait Appendable {
    fn put_if_absent<T: Into<String>>(&mut self, key: &str, value: T) -> &mut Self;
}

impl Appendable for HashMap<HeaderKey, HeaderValue> {
    fn put_if_absent<T: Into<String>>(&mut self, key: &str, value: T) -> &mut Self {
        let key: HeaderKey = key.parse().unwrap();
        if !self.contains_key(&key) {
            let value: HeaderValue = value.into().parse().unwrap();
            self.insert(key, value);
        }
        self
    }
}
