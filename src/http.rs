use std::{collections::HashMap, fmt::Display, str::FromStr};

use reqwest::Url;
use serde::Deserialize;

use crate::headers::{header, Error, Header, Key, Value};

const USER_AGENT_KEY: &str = "user-agent";
const USER_AGENT: &str = "fire/0.1.0";
const CONTENT_LENGTH_KEY: &str = "content-length";
const HOST_KEY: &str = "host";

#[derive(Debug, Deserialize)]
pub struct HttpRequest {
    #[serde(alias = "method")]
    verb: Verb,
    url: String,
    body: Option<String>,
    #[serde(default)]
    headers: HashMap<Key, Value>,
}

impl HttpRequest {
    pub fn verb(&self) -> Verb {
        self.verb
    }

    pub fn url(&self) -> Result<Url, url::ParseError> {
        if self.url.starts_with("http://") || self.url.starts_with("https://") {
            Url::parse(&self.url)
        } else {
            Url::parse(&format!("https://{}", &self.url))
        }
    }

    pub fn headers(&self) -> HashMap<Key, Value> {
        self.headers.clone()
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        let key: Key = match Key::from_str(key) {
            Ok(key) => key,
            Err(_) => return None,
        };

        match self.headers.get(&key) {
            Some(v) => Some(v.as_str()),
            None => None,
        }
    }

    /// Set the _default_ values for headers:
    /// - `user-agent`
    /// - `content-length` (if request has a body)
    /// - `host` (if request URL contains a hostname)
    ///
    /// These default values will only be used if no explicit values are set in the request.
    pub fn set_default_headers(&mut self) -> Result<(), Error> {
        let mut default: Vec<Header> = Vec::with_capacity(3);

        if let Some(host) = self.url().unwrap().host_str() {
            default.push(header(HOST_KEY, host)?);
        }

        if self.has_body() {
            let content_length = self.body_size().to_string();
            default.push(header(CONTENT_LENGTH_KEY, &content_length)?);
        }

        default.push(header(USER_AGENT_KEY, USER_AGENT)?);

        default.into_iter().for_each(|(key, value)| {
            self.headers.entry(key).or_insert(value);
        });

        Ok(())
    }

    pub fn has_body(&self) -> bool {
        self.body_size() != 0
    }

    pub fn body(&self) -> &Option<String> {
        &self.body
    }

    pub fn body_size(&self) -> usize {
        match self.verb {
            Verb::Post | Verb::Put | Verb::Delete | Verb::Patch => match &self.body {
                Some(b) => b.len(),
                None => 0,
            },
            _ => 0,
        }
    }
}

impl FromStr for HttpRequest {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Deserialize)]
#[serde(rename_all(deserialize = "UPPERCASE"))]
pub enum Verb {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl Verb {
    pub fn body(&self) -> BodyStatus {
        match self {
            Verb::Get => BodyStatus::Discouraged,
            Verb::Head => BodyStatus::Discouraged,
            Verb::Post => BodyStatus::Permitted,
            Verb::Put => BodyStatus::Permitted,
            Verb::Delete => BodyStatus::Discouraged,
            Verb::Connect => BodyStatus::Discouraged,
            Verb::Options => BodyStatus::Discouraged,
            Verb::Trace => BodyStatus::Forbidden,
            Verb::Patch => BodyStatus::Permitted,
        }
    }
}

impl Display for Verb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = match self {
            Verb::Get => "GET",
            Verb::Head => "HEAD",
            Verb::Post => "POST",
            Verb::Put => "PUT",
            Verb::Delete => "DELETE",
            Verb::Connect => "CONNECT",
            Verb::Options => "OPTIONS",
            Verb::Trace => "TRACE",
            Verb::Patch => "PATCH",
        };

        f.write_str(s)
    }
}

pub enum BodyStatus {
    Permitted,
    Discouraged,
    Forbidden,
}

impl From<Verb> for reqwest::Method {
    fn from(verb: Verb) -> Self {
        match verb {
            Verb::Get => Self::GET,
            Verb::Head => Self::HEAD,
            Verb::Post => Self::POST,
            Verb::Put => Self::PUT,
            Verb::Delete => Self::DELETE,
            Verb::Connect => Self::CONNECT,
            Verb::Options => Self::OPTIONS,
            Verb::Trace => Self::TRACE,
            Verb::Patch => Self::PATCH,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reqwest::Url;

    use crate::http::Verb;

    use super::HttpRequest;

    #[test]
    fn test_parse_request_from_str() {
        let input = r###"
            # This is a comment
            method: POST
            url: api.github.com/markdown
            headers:
              accept: application/vnd.github+json
              content-type: application/json

            body: >
              {
                  "text": "**Hello** _world_!",
                  "mode": "markdown"
              }
        "###;

        let mut request = HttpRequest::from_str(input).unwrap();
        request.set_default_headers().unwrap();

        assert_eq!(Verb::Post, request.verb());

        let expected_url = Url::parse("https://api.github.com/markdown").unwrap();
        let actual_url = request.url().unwrap();

        assert_eq!(expected_url, actual_url);

        let content_type = request.header("content-type").unwrap();
        let host = request.header("host").unwrap();
        let accept = request.header("accept").unwrap();
        let user_agent = request.header("user-agent").unwrap();
        let content_size: usize = request.header("content-length").unwrap().parse().unwrap();

        assert_eq!("application/json", content_type);
        assert_eq!("api.github.com", host);
        assert_eq!("application/vnd.github+json", accept);
        assert_eq!("fire/", &user_agent[..5]);
        assert!(content_size > 0);

        assert!(request.body().is_some())
    }
}
