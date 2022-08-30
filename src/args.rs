use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use clap::Parser;
use git2::{Repository, RepositoryOpenFlags};
use termcolor::ColorChoice;
use walkdir::{DirEntry, WalkDir};

use crate::prop::{self, ParsePropertyError, Property, DEFAULT_PRIO, HIGHEST_PRIO, LOWEST_PRIO};

const BANNER: &'static str = include_str!("../resources/banner");
const ABOUT: &'static str = include_str!("../resources/about");

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = ABOUT, before_long_help = BANNER)]
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

    /// Environments
    ///
    /// One or several environments which containins environment variables. If the environment is
    /// `development`, the application will search for any occurence of `development.env` in the
    /// current directory and parent directories, as long as the search is confined to the Git
    /// repository where the request resides. If the command is not executed inside a Git
    /// repository, no traversing to parental directories will be done.
    ///
    /// Varaibles found in *.env files will override the environment variables inherited from the
    /// operating system and in the special `.env` which is a "global" environment that will be
    /// always be included regardless of environment.
    #[clap(short, long)]
    env: Vec<String>,

    /// Set environment variable
    ///
    /// Override or set a specific environment variable in KEY=VALUE format. Would have same effect
    /// as setting this in an environment file provided to the --env command, this is just more
    /// convenient when a variable should be changed often. A value given to this flag will take
    /// precendence over an environment variable from the system and an environment variable found
    /// in and environment variables file.
    #[clap(short = 'E', long = "variable")]
    arg_vars: Vec<Property>,

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
            .map(|prop| prop.with_prio(LOWEST_PRIO))
            .collect();
        /*
        let file_envs: Vec<Property> = self
            .env
            .clone()
            .into_iter()
            .map(|file| prop::from_file(&file, DEFAULT_PRIO).unwrap())
            .flatten()
            .collect(); */

        let file_envs: Vec<Property> = Self::find_env_files(&self.file, self.env.clone())
            .into_iter()
            .map(|file| prop::from_file(&file).unwrap())
            .flatten()
            .collect();

        let arg_vars: Vec<Property> = self
            .arg_vars
            .clone()
            .into_iter()
            .map(|prop| prop.with_prio(HIGHEST_PRIO))
            .collect();

        let alloc_size: usize = sys_envs.len() + file_envs.len() + arg_vars.len();

        let mut props: Vec<Property> = Vec::with_capacity(alloc_size);

        props.extend(sys_envs);
        props.extend(file_envs);
        props.extend(arg_vars);

        Ok(props)
    }

    fn find_env_files(request_file: &Path, environments: Vec<String>) -> Vec<PathBuf> {
        let mut files: Vec<String> = environments.into_iter().map(|env| env + ".env").collect();
        files.push(String::from(".env"));

        let end: PathBuf = request_file.parent().unwrap().canonicalize().unwrap();
        let start: PathBuf = match Self::git_root() {
            Some(root) => root.parent().unwrap().to_path_buf(),
            None => end.clone(),
        };

        log::info!("Start is {:?}", start);
        log::info!("End is {:?}", end);
        WalkDir::new(start)
            .follow_links(false)
            .contents_first(false)
            .into_iter()
            //  entry.path().ends_with(&end)
            .filter_entry(|entry| end.starts_with(entry.path()) || entry.file_type().is_file())
            .filter_map(|entry| entry.ok())
            .inspect(|e| log::debug!("Inspecting {:?}", e))
            .filter(|entry| {
                let ftype = entry.file_type();
                if ftype.is_file() {
                    let name: String = entry.file_name().to_str().unwrap().to_string();
                    files.contains(&name)
                } else {
                    false
                }
            })
            .inspect(|e| log::debug!("Found .env file {:?}", e))
            .map(|e| e.into_path())
            .collect()
    }

    fn git_root() -> Option<PathBuf> {
        let ceiling = ["/"];
        Repository::open_ext(".", RepositoryOpenFlags::CROSS_FS, ceiling)
            .map(|p| p.path().to_owned())
            .ok()
    }
}
