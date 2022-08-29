use std::{path::PathBuf, time::Duration};

use clap::Parser;
use termcolor::ColorChoice;

use crate::prop::{self, ParsePropertyError, Property};

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
    /// Print headers
    #[clap(short = 'H', long)]
    pub headers: bool,

    /// Print request
    ///
    /// Print the content of the request as it is sent to the remote host. To also see request
    /// headers, use thea `--headers` flag (`-H`).
    #[clap(short, long)]
    request: bool,

    /// Environment file(s)
    ///
    /// One or several files containing environment variables. These will override the environment
    /// variables inherited from the operating system.
    #[clap(short, long)]
    env: Vec<PathBuf>,

    /// Set environment variable
    ///
    /// Override or set a specific environment variable in KEY=VALUE format. Would have same effect
    /// as setting this in an environment file provided to the --env command, this is just more
    /// convenient when a variable should be changed often. A value given to this flag will take
    /// precendence over an environment variable from the system and an environment variable found
    /// in and environment variables file.
    #[clap(short = 'E', long = "variable")]
    env_vars: Vec<Property>,

    /// Request timeout
    ///
    /// Max time to wait, in seconds, before request times out
    #[clap(short = 'T', long = "timeout", default_value = "30")]
    timeout: usize,

    /// Request file
    ///
    /// Request template file which contains the request that should be executed
    #[clap(value_parser)]
    file: PathBuf,
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

    pub fn print_request(&self) -> bool {
        self.request
    }

    pub fn env(&self) -> Result<Vec<Property>, ParsePropertyError> {
        let sys_envs: Vec<Property> = std::env::vars()
            .into_iter()
            .map(Property::try_from)
            .filter_map(|p| p.ok())
            .collect();

        let file_envs: Vec<Property> = self
            .env
            .clone()
            .into_iter()
            .map(|file| prop::from_file(&file).unwrap())
            .flatten()
            .collect();

        let alloc_size: usize = sys_envs.len() + file_envs.len() + self.env_vars.len();

        let mut props: Vec<Property> = Vec::with_capacity(alloc_size);

        props.extend(sys_envs);
        props.extend(file_envs);
        props.extend(self.env_vars.clone());

        Ok(props)
    }
}
