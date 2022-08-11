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
use reqwest::{Method, Url};
use serde::Deserialize;
use std::convert::Infallible;
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

    let file = std::fs::read_to_string(args.file).unwrap();
    let req: Request = toml::from_str(&file).unwrap();

    let client = reqwest::blocking::Client::new();

    let req = client
        .request(req.method(), req.url().unwrap())
        .headers(req.headers())
        .header("user-agent", "fire/0.1.0")
        .build()
        .unwrap();

    let resp: Response = client.execute(req).unwrap();

    let version = resp.version();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.text().unwrap();

    println!("{:?} {}\n{:?}\n\n{:?}", version, status, headers, body);
}

#[derive(Deserialize, Debug)]
struct Request {
    verb: Option<Verb>,
    url: String,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
}

impl Request {
    pub fn method(&self) -> Method {
        self.verb.unwrap_or_default().into()
    }

    pub fn url(&self) -> Result<Url, url::ParseError> {
        reqwest::Url::from_str(&self.url)
    }

    pub fn headers(&self) -> HeaderMap<HeaderValue> {
        let h = self.headers.clone().unwrap_or_default();
        let mut headers = HeaderMap::with_capacity(h.len());
        for (k, v) in h {
            let k = HeaderName::from_str(&k).unwrap();
            let v = HeaderValue::from_str(&v).unwrap();
            headers.append(k, v);
        }
        headers
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

impl Default for Verb {
    fn default() -> Self {
        Self::Get
    }
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
