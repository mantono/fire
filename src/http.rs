use std::{collections::HashMap, fmt::Display, str::FromStr};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Url,
};
use serde::Deserialize;

use crate::headers::Appendable;

#[derive(Debug, Deserialize)]
pub struct HttpRequest {
    verb: Verb,
    url: String,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
}

const USER_AGENT_KEY: &str = "user-agent";
const USER_AGENT: &str = "fire/0.1.0";
const CONTENT_LENGTH_KEY: &str = "content-length";
const HOST_KEY: &str = "host";

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

    pub fn headers(&self) -> HeaderMap<HeaderValue> {
        let h = self.headers.clone().unwrap_or_default();
        let mut headers = HeaderMap::with_capacity(h.len());
        for (key, value) in h {
            let (k, v) = Self::header(&key, &value);
            headers.append(k, v);
        }

        if let Some(host) = self.url().unwrap().host_str() {
            headers.put_if_absent(HOST_KEY, host);
        }

        let body_size: String = self.body_size().to_string();
        headers.put_if_absent(USER_AGENT_KEY, USER_AGENT);
        headers.put_if_absent(CONTENT_LENGTH_KEY, body_size);
        headers
    }

    fn header(key: &str, value: &str) -> (HeaderName, HeaderValue) {
        let k = HeaderName::from_str(key).unwrap();
        let v = HeaderValue::from_str(value).unwrap();
        (k, v)
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

    use reqwest::{
        header::{HeaderMap, HeaderValue},
        Url,
    };

    use crate::http::Verb;

    use super::HttpRequest;

    #[test]
    fn test_parse_request_from_str() {
        let input = r###"
            # This is a comment
            verb: POST
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

        let request = HttpRequest::from_str(input).unwrap();

        assert_eq!(Verb::Post, request.verb());

        let expected_url = Url::parse("https://api.github.com/markdown").unwrap();
        let actual_url = request.url().unwrap();

        assert_eq!(expected_url, actual_url);

        let headers: HeaderMap<HeaderValue> = request.headers();

        let content_type = headers.get("content-type").unwrap();
        let host = headers.get("host").unwrap();
        let accept = headers.get("accept").unwrap();
        let user_agent = headers.get("user-agent").unwrap().to_str().unwrap();
        let content_size: usize =
            headers.get("content-length").unwrap().to_str().unwrap().parse().unwrap();

        assert_eq!("application/json", content_type);
        assert_eq!("api.github.com", host);
        assert_eq!("application/vnd.github+json", accept);
        assert_eq!("fire/", &user_agent[..5]);
        assert!(content_size > 0);

        assert!(request.body().is_some())
    }
}
