use std::{path::PathBuf, time::Duration};

use clap::Parser;
use termcolor::ColorChoice;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Set verbosity level, 0 - 5
    ///
    /// Set the verbosity level, from 0 (least amount of output) to 5 (most verbose). Note that
    /// logging level configured via RUST_LOG overrides this setting.
    #[clap(short = 'v', long = "verbosity", default_value = "1")]
    pub verbosity_level: u8,

    /// Print debug information
    ///
    /// Print debug information about current build for binary, useful for when an issue is
    /// encountered and reported
    #[clap(short = 'D', long = "debug")]
    pub print_dbg: bool,

    /// Enable colors
    ///
    /// Enable output with colors. By default colors will be used if the terminal seems
    /// to support colors.
    #[clap(short = 'c', long = "colors")]
    enable_colors: bool,

    /// Disable colors
    ///
    /// Disable output with colors. By default colors will be used if the terminal seems
    /// to support colors.
    #[clap(short = 'C', long = "no-colors")]
    disable_colors: bool,

    /// Show headers
    ///
    /// Print any headers received in the response
    #[clap(short = 'H', long)]
    pub headers: bool,

    /// Request file
    ///
    /// Request template file which contains the request that should be executed
    #[clap(value_parser)]
    file: PathBuf,

    /// Environment file(s)
    ///
    /// One or several files containing environment variables. These will override the environment
    /// variables inherited from the operating system.
    #[clap(short, long)]
    env: Vec<PathBuf>,

    /// Request timeout
    ///
    /// Max time to wait, in seconds, before request times out
    #[clap(short = 'T', long = "timeout", default_value = "30")]
    timeout: usize,
}

impl Args {
    pub fn use_colors(&self) -> ColorChoice {
        match (self.enable_colors, self.disable_colors) {
            (true, false) => ColorChoice::Always,
            (false, true) => ColorChoice::Never,
            (false, false) => ColorChoice::Auto,
            (true, true) => {
                panic!("Flags --colors (-c) and --no-colors (-C) are mutually exclusive")
            }
        }
    }

    pub fn file(&self) -> &std::path::Path {
        &self.file
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout as u64)
    }

    pub fn env(&self) -> Vec<PathBuf> {
        self.env.clone()
    }
}
