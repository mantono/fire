use std::str::FromStr;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

fn header(key: &str, value: &str) -> (HeaderName, HeaderValue) {
    let k = HeaderName::from_str(key).unwrap();
    let v = HeaderValue::from_str(value).unwrap();
    (k, v)
}

pub trait Appendable {
    fn put_if_absent<T: Into<String>>(&mut self, key: &str, value: T) -> &mut Self;
}

impl Appendable for HeaderMap {
    fn put_if_absent<T: Into<String>>(&mut self, key: &str, value: T) -> &mut Self {
        if !self.contains_key(key) {
            let v: String = value.into();
            let (k, v) = header(key, v.as_str());
            self.insert(k, v);
        }
        self
    }
}
