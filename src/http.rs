use std::{fmt::Display, str::FromStr};

use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Url,
};

use crate::headers::Appendable;

#[derive(Debug)]
pub struct HttpRequest {
    verb: Verb,
    url: String,
    body: Option<String>,
    headers: Vec<Header>,
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
        let h = self.headers.clone();
        let mut headers = HeaderMap::with_capacity(h.len());
        for Header { key, value } in h {
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

lazy_static! {
    static ref DELIMITER: Regex = Regex::new(r"\n\s*\n").unwrap();
    static ref COMMENT: Regex = Regex::new(r"^[[:blank:]]*#").unwrap();
}

fn verb_and_url(line: &str) -> Result<(Verb, String), String> {
    let mut parts = line.split_ascii_whitespace();
    let verb: Verb = match parts.next() {
        Some(v) => Verb::from_str(v)?,
        None => return Err("Expected a HTTP method on first line, but none were found".to_string()),
    };

    let url: String = match parts.next() {
        Some(p) => p.to_string(),
        None => {
            return Err(
                "Expected a URL on first line one after the HTTP method, but none were found"
                    .to_string(),
            )
        }
    };

    Ok((verb, url))
}

impl FromStr for HttpRequest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Divide the file content into two parts, top and bottom
        let parts: Vec<&str> = DELIMITER.splitn(s, 2).collect();
        // Top contains verb, url and headers
        let top: &str = parts[0];
        // Bottom contains an optional body
        let bottom: Option<&&str> = parts.get(1);

        let mut lines = top.lines().filter(|line| !COMMENT.is_match(line));
        let first_line: &str = match lines.next() {
            Some(first) => first,
            None => return Err("File is empty".to_string()),
        };

        let (verb, url) = verb_and_url(first_line)?;

        let headers: Vec<Header> = lines
            .take_while(|line| !line.is_empty())
            .map(|line| line.to_string())
            .map(|line| Header::from_str(&line).unwrap())
            .collect();

        let body: Option<String> = bottom.map(|v| v.to_owned().to_owned());

        let req = HttpRequest {
            verb,
            url,
            body,
            headers,
        };

        Ok(req)
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    key: String,
    value: String,
}

impl Header {
    pub fn new(key: String, value: String) -> Result<Header, ParseHeaderError> {
        Ok(Header { key, value })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug)]
pub enum ParseHeaderError {
    Entry(String),
    Key(String),
    Value(String),
}

impl FromStr for Header {
    type Err = ParseHeaderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts.len() {
            2 => {
                let key: String = parts[0].trim().to_string();
                let value: String = parts[1].trim().to_string();
                Header::new(key, value)
            }
            _ => Err(ParseHeaderError::Entry(s.to_string())),
        }
    }
}

impl TryFrom<(String, String)> for Header {
    type Error = ParseHeaderError;

    fn try_from(value: (String, String)) -> Result<Self, Self::Error> {
        Header::new(value.0, value.1)
    }
}

#[derive(Debug, Clone, Copy)]
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

impl FromStr for Verb {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CONNECT" => Ok(Verb::Connect),
            "DELETE" => Ok(Verb::Delete),
            "GET" => Ok(Verb::Get),
            "HEAD" => Ok(Verb::Head),
            "OPTIONS" => Ok(Verb::Options),
            "PATCH" => Ok(Verb::Patch),
            "POST" => Ok(Verb::Post),
            "PUT" => Ok(Verb::Put),
            "TRACE" => Ok(Verb::Trace),
            _ => Err(format!("Invalid HTTP method '{}'", s)),
        }
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
