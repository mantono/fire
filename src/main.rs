mod args;
mod dbg;
mod headers;
mod http;
mod io;
mod logger;
mod prop;
mod template;

#[macro_use]
extern crate lazy_static;

use crate::args::Args;
use crate::dbg::dbg_info;
use crate::http::HttpRequest;
use crate::io::write;
use crate::io::write_color;
use crate::io::writeln;
use crate::io::writeln_color;
use crate::io::writeln_spec;
use crate::logger::setup_logging;
use crate::prop::Property;
use crate::template::substitution;
use clap::Parser;
use handlebars::template::Parameter;
use headers::Appendable;
use log::Metadata;
use regex::Regex;
use reqwest::blocking::{Request as RwReq, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Body, Method, StatusCode, Url};
use serde::Deserialize;
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::{collections::HashMap, process};
use termcolor::{Color, ColorSpec, StandardStream};

fn main() {
    let args: Args = Args::parse();
    setup_logging(args.verbosity_level);
    log::debug!("Config: {:?}", args);

    let mut stdout = StandardStream::stdout(args.use_colors());

    if args.print_dbg {
        write(&mut stdout, &dbg_info());
        process::exit(0);
    }

    // 1. Read file content
    let file = std::fs::read_to_string(args.file).unwrap();
    // 2. Read enviroment variables from system environment and extra environments supplied via cli
    // 3. Apply template substitution

    let mut env_vars: Vec<Property> = std::env::vars()
        .into_iter()
        .map(Property::try_from)
        .filter_map(|p| p.ok())
        .collect();

    let props: Vec<Property> = args
        .env
        .into_iter()
        .map(|file| prop::from_file(&file).unwrap())
        .flatten()
        .collect();

    env_vars.extend(props);

    log::debug!("Received properties {:?}", env_vars);

    let content: String = substitution(file, env_vars).unwrap();
    log::debug!("Content with template substitution done:\n{}", content);

    // 4. Parse Validate format of request
    let request: HttpRequest = HttpRequest::from_str(&content).unwrap();
    // 5. Add user-agent header if missing
    // 6. Add content-length header if missing
    // 7. Make (and optionally print) request
    let client = reqwest::blocking::Client::new();

    log::info!("{} {}", request.verb(), request.url().unwrap());

    if let Some(body) = request.body() {
        log::info!("{body}");
    }

    let req = client
        .request(request.verb().into(), request.url().unwrap())
        .headers(request.headers());

    let req = match request.body() {
        Some(body) => req.body(body.clone()).build().unwrap(),
        None => req.build().unwrap(),
    };

    let start: Instant = Instant::now();
    let resp: Result<Response, reqwest::Error> = client.execute(req);
    let end: Instant = Instant::now();
    let resp: Response = resp.unwrap();

    let duration: Duration = end.duration_since(start);
    // 8. Print response if successful, or error, if not

    let version = resp.version();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.text().unwrap();

    let status_color: Option<Color> = match status.as_u16() {
        200..=299 => Some(Color::Green),
        400..=499 => Some(Color::Yellow),
        500..=599 => Some(Color::Red),
        _ => None,
    };

    let (body_len, unit): (usize, String) = if body.len() >= 1024 {
        ((body.len() / 1024), String::from("kb"))
    } else {
        (body.len(), String::from("b"))
    };

    let version: String = format!("{version:?} ");
    write(&mut stdout, &version);

    let status: String = status.to_string();
    write_color(&mut stdout, &status, status_color);

    let outcome: String = format!(" {} ms {} {}", duration.as_millis(), body_len, unit);
    writeln(&mut stdout, &outcome);

    let border_len: usize = version.len() + status.len() + outcome.len();
    let border = "━".repeat(border_len);
    writeln(&mut stdout, &border);

    if args.headers {
        let mut spec = ColorSpec::new();
        spec.set_dimmed(true);
        for (k, v) in headers {
            writeln_spec(&mut stdout, &format!("{}: {:?}", k.unwrap(), v), &spec);
        }
    }
    if !body.is_empty() {
        writeln(&mut stdout, &format!("\n{body}"));
    }
}
