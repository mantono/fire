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
use crate::io::writeln_spec;
use crate::logger::setup_logging;
use crate::prop::Property;
use crate::template::substitution;
use clap::Parser;
use reqwest::blocking::Response;
use reqwest::Url;
use std::fmt::Debug;
use std::fmt::Display;
use std::path::PathBuf;
use std::process;
use std::process::Termination;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use template::SubstitutionError;
use termcolor::{Color, ColorSpec, StandardStream};

fn main() -> Result<(), FireError> {
    let args: Args = Args::parse();
    setup_logging(args.verbosity_level);
    log::debug!("Config: {:?}", args);

    let mut stdout = StandardStream::stdout(args.use_colors());

    if args.print_dbg {
        write(&mut stdout, &dbg_info());
        process::exit(0);
    }

    // 1. Read file content
    let file = match std::fs::read_to_string(args.file()) {
        Ok(file) => file,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound => {
                    Err(FireError::FileNotFound(args.file().to_path_buf()))
                }
                std::io::ErrorKind::PermissionDenied => {
                    Err(FireError::NoReadPermission(args.file().to_path_buf()))
                }
                _ => Err(FireError::GenericIO(e.to_string())),
            }
        }
    };
    // 2. Read enviroment variables from system environment and extra environments supplied via cli
    // 3. Apply template substitution

    let mut env_vars: Vec<Property> = std::env::vars()
        .into_iter()
        .map(Property::try_from)
        .filter_map(|p| p.ok())
        .collect();

    let props: Vec<Property> = args
        .env()
        .into_iter()
        .map(|file| prop::from_file(&file).unwrap())
        .flatten()
        .collect();

    env_vars.extend(props);

    log::debug!("Received properties {:?}", env_vars);

    let content: String = substitution(file, env_vars)?;
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
        .timeout(args.timeout())
        .headers(request.headers());

    let req = match request.body() {
        Some(body) => req.body(body.clone()).build().unwrap(),
        None => req.build().unwrap(),
    };

    let start: Instant = Instant::now();
    let resp: Result<Response, reqwest::Error> = client.execute(req);
    let end: Instant = Instant::now();
    let resp: Response = match resp {
        Ok(response) => response,
        Err(e) => {
            if e.is_timeout() {
                return Err(FireError::Timeout(e.url().unwrap().clone()));
            } else if e.is_connect() {
                return Err(FireError::Connection(e.url().unwrap().clone()));
            } else {
                return Err(FireError::Other(e.to_string()));
            }
        }
    };

    let duration: Duration = end.duration_since(start);
    // 8. Print response if successful, or error, if not

    let version = resp.version();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = match resp.text() {
        Ok(body) => body,
        Err(e) => return Err(FireError::Other(e.to_string())),
    };

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

    Ok(())
}

enum FireError {
    Timeout(Url),
    Connection(Url),
    FileNotFound(PathBuf),
    NoReadPermission(PathBuf),
    NotAFile(PathBuf),
    GenericIO(String),
    Template(String),
    Other(String),
}

impl Debug for FireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for FireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg: String = match &self {
            FireError::Timeout(url) => format!("Request to {url} timed out"),
            FireError::Connection(url) => format!("Unable to connect to URL {url}, verify that the URL is correct and that you have a working internet connection"),
            &FireError::FileNotFound(path) => format!("Could not find file {:?}", path.clone()),
            FireError::GenericIO(err) => format!("IO error: {err}"),
            FireError::NotAFile(path) => format!("{:?} exists but it is not a file", path.clone()),
            FireError::NoReadPermission(path) => format!("No permission to read file {:?}", path.clone()),
            FireError::Template(msg) => format!("Unable to render request from template. {msg}"),
            FireError::Other(err) => format!("Error: {err}"),
        };

        f.write_str(&msg)
    }
}

impl Termination for FireError {
    fn report(self) -> process::ExitCode {
        todo!()
    }
}

impl From<SubstitutionError> for FireError {
    fn from(e: SubstitutionError) -> Self {
        match e {
            SubstitutionError::MissingValue(err) => FireError::Template(err),
        }
    }
}
