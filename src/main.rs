mod args;
mod dbg;
mod fmt;
mod logger;

use crate::args::Args;
use crate::dbg::dbg_info;
use crate::fmt::write;
use crate::logger::setup_logging;
use clap::Parser;
use log::Metadata;
use reqwest::blocking::{Request as RwReq, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Body, Method, Url};
use serde::Deserialize;
use std::ascii::AsciiExt;
use std::borrow::Borrow;
use std::convert::Infallible;
use std::fmt::Display;
use std::str::FromStr;
use std::{collections::HashMap, process};
use termcolor::{Color, StandardStream};

fn main() {
    let args: Args = Args::parse();
    setup_logging(args.verbosity_level);
    log::debug!("Config: {:?}", args);
    log::info!("This is a log message");

    if args.print_dbg {
        println!("{}", dbg_info());
        process::exit(0);
    }

    // 1. Read file content
    let file = std::fs::read_to_string(args.file).unwrap();
    // 2. Read enviroment variables from system environment and extra environments supplied via cli
    // 3. Apply template substitution
    // 4. Parse Validate  format of request
    let mut request: HttpRequest = HttpRequest::from_str(&file).unwrap();
    // 5. Add user-agent header if missing
    request.set_user_agent("fire/0.1.0");
    // 6. Add content-length header if missing
    // 7. Make (and optionally print) request
    let client = reqwest::blocking::Client::new();

    let req = client
        .request(request.verb().into(), request.url().unwrap())
        .headers(request.headers())
        .build()
        .unwrap();

    let resp: Response = client.execute(req).unwrap();
    // 8. Print response if successful, or error, if not

    let version = resp.version();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.text().unwrap();

    println!("{:?} {}\n{:?}\n\n{:?}", version, status, headers, body);
}

#[derive(Deserialize, Debug)]
struct HttpRequest {
    verb: Verb,
    url: String,
    proto: Protocol,
    body: Option<String>,
    headers: HashMap<String, String>,
}

const USER_AGENT_KEY: &'static str = "user-agent";
const CONTENT_LENGTH_KEY: &'static str = "content-length";
const HOST_KEY: &'static str = "host";

impl HttpRequest {
    pub fn verb(&self) -> Verb {
        self.verb
    }

    pub fn url(&self) -> Result<Url, url::ParseError> {
        let url: String = self.url.to_ascii_lowercase();
        if url.starts_with("http://") || url.starts_with("https://") {
            Url::parse(&url)
        } else {
            Url::parse(&format!("{}://{}", &self.proto, &self.url))
        }
    }

    pub fn headers(&self) -> HeaderMap<HeaderValue> {
        let h = self.headers.clone();
        let mut headers = HeaderMap::with_capacity(h.len());
        for (k, v) in h {
            let k = HeaderName::from_str(&k).unwrap();
            let v = HeaderValue::from_str(&v).unwrap();
            headers.append(k, v);
        }
        headers
    }

    pub fn set_user_agent(&mut self, agent: &str) -> &mut Self {
        if !self.headers.contains_key(USER_AGENT_KEY) {
            self.headers.insert(USER_AGENT_KEY.to_string(), agent.to_string());
        }
        self
    }

    pub fn set_content_length(&mut self) -> &mut Self {
        if !self.headers.contains_key(CONTENT_LENGTH_KEY) {
            let length: usize = self.body_size();
            self.headers.insert(CONTENT_LENGTH_KEY.to_string(), length.to_string());
        }
        self
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines().filter(|line| !line.starts_with("#"));
        let first: &str = match lines.next() {
            Some(first) => first,
            None => return Err("File is empty".to_string()),
        };
        let mut parts = first.split_ascii_whitespace();

        let verb: Verb = match parts.next() {
            Some(v) => Verb::from_str(v)?,
            None => {
                return Err("Expected a HTTP method on first line, but none were found".to_string())
            }
        };

        let url: String =
            match parts.next() {
                Some(p) => p.to_string(),
                None => return Err(
                    "Expected a URL on first line one after the HTTP method, but none were found"
                        .to_string(),
                ),
            };

        let req = HttpRequest {
            verb,
            url,
            proto: Protocol::default(),
            body: None,
            headers: HashMap::with_capacity(4),
        };

        Ok(req)
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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

#[derive(Debug, Copy, Clone, Deserialize)]
enum Protocol {
    Http,
    Https,
}

impl Default for Protocol {
    fn default() -> Self {
        Self::Https
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http => f.write_str("http"),
            Protocol::Https => f.write_str("https"),
        }
    }
}
