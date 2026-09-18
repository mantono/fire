use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_STDOUT_BYTES: usize = 64 * 1024;

/// Errors that can occur while invoking a dynamic (command) fallback's shell
/// command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerError {
    /// The shell process itself could not be launched.
    Launch(String),
    /// The shell process launched but exited with a non-zero status.
    ExitStatus(i32),
    /// The command produced stdout bytes that are not valid UTF-8.
    NonUtf8,
    /// The command did not finish before the configured timeout.
    Timeout,
    /// The command produced more stdout than the configured limit.
    OutputTooLarge,
}

/// Run `command` via the platform shell and return its normalized stdout.
///
/// Only trailing CR/LF line endings are removed; all other whitespace is
/// preserved verbatim. A non-zero exit status, a failure to launch the
/// shell, or non-UTF-8 stdout each yield a distinct [`RunnerError`].
#[cfg(unix)]
pub fn run_command(command: &str) -> Result<String, RunnerError> {
    run_with_shell("sh", "-c", command)
}

#[cfg(windows)]
pub fn run_command(command: &str) -> Result<String, RunnerError> {
    run_with_shell("cmd", "/C", command)
}

fn run_with_shell(shell: &str, shell_arg: &str, command: &str) -> Result<String, RunnerError> {
    run_with_limits(shell, shell_arg, command, COMMAND_TIMEOUT, MAX_STDOUT_BYTES)
}

fn run_with_limits(
    shell: &str,
    shell_arg: &str,
    command: &str,
    timeout: Duration,
    max_stdout_bytes: usize,
) -> Result<String, RunnerError> {
    let mut child = Command::new(shell)
        .arg(shell_arg)
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| RunnerError::Launch(error.to_string()))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let (sender, receiver) = mpsc::sync_channel(1);

    thread::spawn(move || {
        let mut reader = stdout;
        let mut bytes: Vec<u8> = Vec::with_capacity(max_stdout_bytes.saturating_add(1));
        let result = reader
            .by_ref()
            .take(max_stdout_bytes.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });

    let deadline: Instant = Instant::now() + timeout;
    let mut stdout: Option<Vec<u8>> = None;

    loop {
        match receiver.try_recv() {
            Ok(Ok(bytes)) if bytes.len() > max_stdout_bytes => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunnerError::OutputTooLarge);
            }
            Ok(Ok(bytes)) => stdout = Some(bytes),
            Ok(Err(error)) => return Err(RunnerError::Launch(error.to_string())),
            Err(TryRecvError::Disconnected) if stdout.is_none() => {
                return Err(RunnerError::Launch(String::from("stdout reader disconnected")))
            }
            Err(TryRecvError::Disconnected) => {}
            Err(TryRecvError::Empty) => {}
        }

        if let Some(status) =
            child.try_wait().map_err(|error| RunnerError::Launch(error.to_string()))?
        {
            if let Some(stdout) = stdout {
                return normalize(Output {
                    status,
                    stdout,
                    stderr: Vec::new(),
                });
            }
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RunnerError::Timeout);
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn normalize(output: Output) -> Result<String, RunnerError> {
    if !output.status.success() {
        let code: i32 = output.status.code().unwrap_or(-1);
        return Err(RunnerError::ExitStatus(code));
    }

    let stdout: String = String::from_utf8(output.stdout).map_err(|_| RunnerError::NonUtf8)?;
    Ok(trim_trailing_line_ending(&stdout))
}

fn trim_trailing_line_ending(value: &str) -> String {
    value.trim_end_matches(['\r', '\n']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_failure_is_surfaced() {
        let err =
            run_with_shell("definitely-not-a-real-shell-binary-xyz", "-c", "echo hi").unwrap_err();
        match err {
            RunnerError::Launch(_) => {}
            other => panic!("expected Launch, got {:?}", other),
        }
    }

    #[cfg(unix)]
    #[test]
    fn trailing_crlf_is_removed_but_inner_whitespace_survives() {
        let result = run_command("printf 'foo  bar  \\r\\n'").unwrap();
        assert_eq!("foo  bar  ", result);
    }

    #[cfg(unix)]
    #[test]
    fn nonzero_exit_status_is_surfaced() {
        let err = run_command("exit 3").unwrap_err();
        assert_eq!(RunnerError::ExitStatus(3), err);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_stdout_is_surfaced() {
        // `\377` is a POSIX octal escape (0xFF) supported by both dash and
        // bash. The non-portable `\xff` hex escape is silently ignored by
        // dash (Ubuntu's default `/bin/sh`), which prints the literal bytes
        // `\xff` instead of a single invalid byte, so it must not be used
        // here.
        let err = run_command("printf '\\377'").unwrap_err();
        assert_eq!(RunnerError::NonUtf8, err);
    }

    #[cfg(windows)]
    #[test]
    fn trailing_crlf_is_removed_but_inner_whitespace_survives_windows() {
        let result = run_command("echo foo  bar  ").unwrap();
        assert_eq!("foo  bar  ", result);
    }

    #[cfg(windows)]
    #[test]
    fn nonzero_exit_status_is_surfaced_windows() {
        let err = run_command("exit 3").unwrap_err();
        assert_eq!(RunnerError::ExitStatus(3), err);
    }

    #[cfg(unix)]
    #[test]
    fn command_is_killed_when_it_exceeds_the_timeout() {
        let err =
            run_with_limits("sh", "-c", "sleep 1", std::time::Duration::from_millis(5), 64 * 1024)
                .unwrap_err();
        assert_eq!(RunnerError::Timeout, err);
    }

    #[cfg(unix)]
    #[test]
    fn command_output_is_capped() {
        let err = run_with_limits(
            "sh",
            "-c",
            "yes | head -c 65537",
            std::time::Duration::from_secs(1),
            64 * 1024,
        )
        .unwrap_err();
        assert_eq!(RunnerError::OutputTooLarge, err);
    }
}
