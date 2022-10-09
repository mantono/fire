mod args;
mod dbg;
mod error;
mod format;
mod headers;
mod http;
mod io;
mod logger;
mod prop;
mod template;

use crate::args::Args;
use crate::dbg::dbg_info;
use crate::error::exit;
use crate::format::ContentFormatter;
use crate::http::HttpRequest;
use crate::io::write;
use crate::io::write_color;
use crate::io::writeln;
use crate::io::writeln_spec;
use crate::logger::setup_logging;
use crate::prop::Property;
use crate::template::substitution;
use clap::Parser;
use error::FireError;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use template::SubstitutionError;
use termcolor::{Color, ColorSpec, StandardStream};
use url::Url;

fn main() -> ExitCode {
    match exec() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => exit(e),
    }
}

fn exec() -> Result<(), FireError> {
    let args: Args = Args::parse();
    setup_logging(args.verbosity_level);
    log::debug!("Config: {:?}", args);

    let mut stdout = StandardStream::stdout(args.use_colors());

    if args.print_dbg {
        write(&mut stdout, &dbg_info());
        return Ok(());
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
    let props: Vec<Property> = args.env().expect("Unable to load env vars");
    log::debug!("Received properties {:?}", props);

    // 3. Apply template substitution
    let content: String = substitution(file, props)?;

    // 4. Parse Validate format of request
    let mut request: HttpRequest = HttpRequest::from_str(&content).unwrap();

    // 5. Add default header, if missing
    request.set_default_headers().unwrap();

    // 6. Print request (optional)

    let syntax_hilighiting: bool = args.use_colors() != termcolor::ColorChoice::Never;
    let formatters: Vec<Box<dyn ContentFormatter>> = format::formatters(syntax_hilighiting);

    let req_headers = request.headers();

    let content_type: Option<&str> = request.header("content-type");

    if args.print_request() {
        let title: String = format!("{} {}", request.verb(), request.url().unwrap());
        writeln(&mut stdout, &title);
        let border = "━".repeat(title.len());
        writeln(&mut stdout, &border);

        if args.headers {
            let mut spec = ColorSpec::new();
            spec.set_dimmed(true);
            for (k, v) in &req_headers {
                writeln_spec(&mut stdout, &format!("{}: {}", k.as_str(), v.as_str()), &spec);
            }
            if request.body().is_some() {
                writeln(&mut stdout, "");
            }
        }

        if let Some(body) = request.body() {
            let content: String = formatters
                .iter()
                .filter(|fmt| fmt.accept(content_type))
                .fold(body.clone(), |content, fmt| fmt.format(content).unwrap());

            writeln(&mut stdout, &content);
        }
        writeln(&mut stdout, "");
    }

    // 7. Make request
    let url: Url = request.url().unwrap().clone();
    let request: ureq::Request = ureq::Request::from(request).timeout(args.timeout());

    let start: Instant = Instant::now();
    let response: Result<ureq::Response, ureq::Error> = request.call();
    let end: Instant = Instant::now();
    let duration: Duration = end.duration_since(start);

    // 8. Handle respone
    let response: http::HttpResponse = conv(response, url)?;
    let status: u16 = response.status();

    let status_color: Option<Color> = match status {
        200..=299 => Some(Color::Green),
        400..=499 => Some(Color::Yellow),
        500..=599 => Some(Color::Red),
        _ => None,
    };

    let body: &str = response.body();
    log::debug!("Body of response:\n{body}");

    let (body_len, unit): (usize, String) = if body.len() >= 1024 {
        ((body.len() / 1024), String::from("kb"))
    } else {
        (body.len(), String::from("b"))
    };

    let version: String = format!("{} ", response.version());

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
        for (key, value) in response.headers() {
            writeln_spec(&mut stdout, &format!("{}: {:?}", key, value), &spec);
        }
        if !body.is_empty() {
            io::writeln(&mut stdout, "");
        }
    }

    if !body.is_empty() {
        let content_type = response.header("content-type");
        let content: String = formatters
            .iter()
            .filter(|fmt| fmt.accept(content_type))
            .fold(body.to_string(), |content, fmt| fmt.format(content).unwrap());

        io::write(&mut stdout, &content);
        if !content.ends_with('\n') {
            io::writeln(&mut stdout, "");
        }
    }

    Ok(())
}

fn conv(
    res: Result<ureq::Response, ureq::Error>,
    url: Url,
) -> Result<http::HttpResponse, FireError> {
    let response: ureq::Response = match res {
        Ok(response) => response,
        Err(e) => match e {
            ureq::Error::Status(_, response) => response,
            ureq::Error::Transport(trans) => match trans.kind() {
                ureq::ErrorKind::Dns => return Err(FireError::Connection(url)),
                ureq::ErrorKind::ConnectionFailed => return Err(FireError::Connection(url)),

                ureq::ErrorKind::Io => return Err(FireError::Connection(url)),
                _ => {
                    return Err(FireError::Other(
                        trans.message().unwrap_or("Unknown transport error").to_string(),
                    ))
                }
            },
        },
    };

    let response: http::HttpResponse = response.into();
    Ok(response)
}

impl From<SubstitutionError> for FireError {
    fn from(e: SubstitutionError) -> Self {
        match e {
            SubstitutionError::MissingValue(err) => FireError::Template(err),
        }
    }
}
