use std::error::Error as StdError;
use std::fmt::Debug;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{self, ExitCode, Termination};

use url::Url;

use crate::prop;
use crate::prop::ParsePropertyError;
use crate::runner::RunnerError;

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
    /// A dynamic (command) fallback was reached without the opt-in
    /// `--allow-command-fallbacks` flag being present.
    CommandFallbackNotAllowed,
    /// A dynamic (command) fallback's shell command could not be launched,
    /// exited with a non-zero status, or produced non-UTF-8 output.
    ///
    /// The variant only carries [`RunnerError`], which never contains
    /// resolved fallback values or command stdout, so no secret material is
    /// exposed by this error.
    CommandFallbackFailed(RunnerError),
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
            FireError::CommandFallbackNotAllowed => String::from(
                "Dynamic command fallback requires --allow-command-fallbacks",
            ),
            FireError::CommandFallbackFailed(err) => match err {
                RunnerError::Launch(msg) => {
                    format!("Unable to launch dynamic command fallback: {msg}")
                }
                RunnerError::ExitStatus(code) => {
                    format!("Dynamic command fallback exited with status {code}")
                }
                RunnerError::NonUtf8 => {
                    String::from("Dynamic command fallback produced non-UTF-8 output")
                }
                RunnerError::Timeout => String::from("Dynamic command fallback timed out"),
                RunnerError::OutputTooLarge => {
                    String::from("Dynamic command fallback produced too much output")
                }
            },
            FireError::Other(err) => format!("Error: {err}"),
        };

        f.write_str(&msg)
    }
}

impl FireError {
    /// The stable exit code reported for this error.
    ///
    /// Kept as its own method (rather than inline in [`Termination::report`])
    /// so tests can assert on the concrete `u8` value without relying on
    /// `ExitCode`'s opaque, non-comparable representation.
    fn exit_code(&self) -> u8 {
        match self {
            FireError::Timeout(_) => 3,
            FireError::Connection(_) => 4,
            FireError::FileNotFound(_) => 5,
            FireError::NoReadPermission(_) => 6,
            FireError::NotAFile(_) => 7,
            FireError::GenericIO(_) => 8,
            FireError::TemplateKey(_) => 9,
            FireError::TemplateRendering => 10,
            FireError::Environment(_) => 11,
            FireError::CommandFallbackNotAllowed => 12,
            FireError::CommandFallbackFailed(_) => 13,
            FireError::Other(_) => 1,
        }
    }
}

impl Termination for FireError {
    fn report(self) -> process::ExitCode {
        ExitCode::from(self.exit_code())
    }
}

pub fn exit(err: FireError) -> ExitCode {
    eprintln!("{err}");
    err.report()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn sample(variant: &FireError) -> u8 {
        variant.exit_code()
    }

    #[test]
    fn command_fallback_not_allowed_has_a_dedicated_message() {
        let err = FireError::CommandFallbackNotAllowed;
        assert_eq!("Dynamic command fallback requires --allow-command-fallbacks", err.to_string());
    }

    #[test]
    fn command_fallback_launch_failure_message_excludes_stdout() {
        let err = FireError::CommandFallbackFailed(RunnerError::Launch(String::from(
            "No such file or directory",
        )));
        assert_eq!(
            "Unable to launch dynamic command fallback: No such file or directory",
            err.to_string()
        );
    }

    #[test]
    fn command_fallback_exit_status_failure_message() {
        let err = FireError::CommandFallbackFailed(RunnerError::ExitStatus(3));
        assert_eq!("Dynamic command fallback exited with status 3", err.to_string());
    }

    #[test]
    fn command_fallback_non_utf8_failure_message() {
        let err = FireError::CommandFallbackFailed(RunnerError::NonUtf8);
        assert_eq!("Dynamic command fallback produced non-UTF-8 output", err.to_string());
    }

    #[test]
    fn command_fallback_timeout_failure_message() {
        let err = FireError::CommandFallbackFailed(RunnerError::Timeout);
        assert_eq!("Dynamic command fallback timed out", err.to_string());
    }

    #[test]
    fn command_fallback_excessive_output_failure_message() {
        let err = FireError::CommandFallbackFailed(RunnerError::OutputTooLarge);
        assert_eq!("Dynamic command fallback produced too much output", err.to_string());
    }

    /// Documents the stable exit codes chosen for the two new variants: 1
    /// and 3-11 are already used by other `FireError` variants, so the
    /// dynamic-command-fallback variants claim the next free codes, 12 and
    /// 13, and must never change once released.
    #[test]
    fn command_fallback_exit_codes_are_stable_and_documented() {
        assert_eq!(12, sample(&FireError::CommandFallbackNotAllowed));
        assert_eq!(13, sample(&FireError::CommandFallbackFailed(RunnerError::NonUtf8)));
    }

    #[test]
    fn all_variant_exit_codes_are_unique() {
        let variants: Vec<FireError> = vec![
            FireError::Timeout(Url::parse("http://example.com").unwrap()),
            FireError::Connection(Url::parse("http://example.com").unwrap()),
            FireError::FileNotFound(PathBuf::from("x")),
            FireError::NoReadPermission(PathBuf::from("x")),
            FireError::NotAFile(PathBuf::from("x")),
            FireError::GenericIO(String::from("io")),
            FireError::TemplateRendering,
            FireError::TemplateKey(String::from("KEY")),
            FireError::Environment(ParsePropertyError::Entry(String::from("entry"))),
            FireError::CommandFallbackNotAllowed,
            FireError::CommandFallbackFailed(RunnerError::NonUtf8),
            FireError::Other(String::from("other")),
        ];

        let codes: Vec<u8> = variants.iter().map(FireError::exit_code).collect();
        let unique: HashSet<u8> = codes.iter().copied().collect();
        assert_eq!(codes.len(), unique.len(), "exit codes must be pairwise distinct: {codes:?}");
    }
}
