mod args;
mod dbg;
mod error;
mod format;
mod io;
mod logger;
mod prop;
mod templ;
mod template;

use crate::args::Args;
use crate::dbg::dbg_info;
use crate::error::exit;
use crate::format::ContentFormatter;
use crate::io::write;
use crate::io::write_color;
use crate::io::writeln;
use crate::io::writeln_spec;
use crate::logger::setup_logging;
use crate::prop::Property;
use crate::template::substitution;
use clap::Parser;
use error::FireError;
use httpx::HttpRequest;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use template::SubstitutionError;
use termcolor::{Color, ColorSpec, StandardStream};

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

    // Read file content
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

    // Read enviroment variables from system environment and extra environments supplied via cli
    let props: Vec<Property> = args.env().expect("Unable to load env vars");
    log::debug!("Received properties {:?}", props);

    // Apply template substitution
    let content: String =
        substitution(file, props, args.interactive(), args.try_colors(), args.trim)?;

    // Parse Validate format of request
    let mut request: HttpRequest = HttpRequest::from_str(&content).unwrap();

    // Add default header, if missing
    request.set_default_headers().unwrap();

    // Print request (optional)
    let syntax_hilighiting: bool = args.try_colors();
    let formatters: Vec<Box<dyn ContentFormatter>> = format::formatters(syntax_hilighiting);

    let req_headers = request.headers();

    let content_type: Option<&str> = request.header("content-type");

    if args.print_request() {
        let title: String = format!("{} {}", request.method(), request.url().unwrap());
        writeln(&mut stdout, &title);
        let border = "━".repeat(title.len());
        writeln(&mut stdout, &border);

        if args.print_headers() {
            let mut spec = ColorSpec::new();
            spec.set_dimmed(true);
            for (k, v) in &req_headers {
                let value: &str = v.to_str().unwrap_or("**Invalid header value**");
                writeln_spec(&mut stdout, &format!("{}: {}", k.as_str(), value), &spec);
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

    // Ask for confirmation (optional)
    let fire: bool = if args.ask() {
        let theme = dialoguer::theme::ColorfulTheme::default();
        let prompt = if args.use_colors() != termcolor::ColorChoice::Never {
            dialoguer::Confirm::with_theme(&theme)
        } else {
            dialoguer::Confirm::new()
        };
        prompt
            .with_prompt("Confirm")
            .interact()
            .expect("Unrecoverable terminal I/O error")
    } else {
        true
    };

    if !fire {
        log::debug!("Request cancelled by user");
        return Ok(());
    }

    // Make request
    let start: Instant = Instant::now();
    let response: httpx::HttpResponse = httpx::request::call(request, args.timeout())?;
    let end: Instant = Instant::now();
    let duration: Duration = end.duration_since(start);

    // Handle respone
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

    if args.print_headers() {
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

impl From<SubstitutionError> for FireError {
    fn from(e: SubstitutionError) -> Self {
        match e {
            SubstitutionError::MissingValue(err) => FireError::TemplateKey(err),
            SubstitutionError::Rendering => FireError::TemplateRendering,
        }
    }
}

impl From<httpx::TransportError> for FireError {
    fn from(e: httpx::TransportError) -> Self {
        match e {
            httpx::TransportError::Timeout(url, _) => Self::Timeout(url),
            httpx::TransportError::Connection(url) => Self::Connection(url),
            httpx::TransportError::UnknownHost(url) => Self::Connection(url),
            httpx::TransportError::Other(msg) => Self::Other(msg),
        }
    }
}
