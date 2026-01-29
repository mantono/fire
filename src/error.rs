use std::error::Error as StdError;
use std::fmt::Debug;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{self, ExitCode, Termination};

use url::Url;

use crate::prop;
use crate::prop::ParsePropertyError;

pub trait Error: StdError + Termination {}

pub enum FireError {
    Timeout(Url),
    Connection(Url),
    FileNotFound(PathBuf),
    NoReadPermission(PathBuf),
    NotAFile(PathBuf),
    GenericIO(String),
    TemplateRendering,
    TemplateKey(String),
    Environment(ParsePropertyError),
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
            FireError::TemplateRendering => String::from("Unable to render request from template"),
            FireError::TemplateKey(key) => format!("Unable to render request due to missing value for key {key}"),
            FireError::Environment(err) => match err {
                prop::ParsePropertyError::Entry(entry) => format!("Invalid entry in environments file: {entry}"),
                prop::ParsePropertyError::Key(key) => format!("Invalid key in environments file: {key}"),
                prop::ParsePropertyError::Value(value) => format!("Invalid value in environments file: {value}"),
                prop::ParsePropertyError::File(file) => format!("Invalid environments file: {file}"),
            },
            FireError::Other(err) => format!("Error: {err}"),
        };

        f.write_str(&msg)
    }
}

impl Termination for FireError {
    fn report(self) -> process::ExitCode {
        match self {
            FireError::Timeout(_) => ExitCode::from(3),
            FireError::Connection(_) => ExitCode::from(4),
            FireError::FileNotFound(_) => ExitCode::from(5),
            FireError::NoReadPermission(_) => ExitCode::from(6),
            FireError::NotAFile(_) => ExitCode::from(7),
            FireError::GenericIO(_) => ExitCode::from(8),
            FireError::TemplateKey(_) => ExitCode::from(9),
            FireError::TemplateRendering => ExitCode::from(10),
            FireError::Environment(_) => ExitCode::from(11),
            FireError::Other(_) => ExitCode::from(1),
        }
    }
}

pub fn exit(err: FireError) -> ExitCode {
    eprintln!("{err}");
    err.report()
}
