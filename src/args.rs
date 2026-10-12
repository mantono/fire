use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;
use termcolor::ColorChoice;
use walkdir::WalkDir;

use crate::prop::{self, ParsePropertyError, Property};

const BANNER: &str = include_str!("../resources/banner");
const ABOUT: &str = include_str!("../resources/about");

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
    headers: bool,

    /// Print request
    ///
    /// Print the content of the request as it is sent to the remote host. To also see request
    /// headers, use the `--headers` flag (`-H`).
    #[clap(short, long)]
    request: bool,

    /// Ask for confirmation
    ///
    /// Ask for confirmation before executing a request. This implies `--request` and `--headers`
    /// since the request will have to be displayed before it can be confirmed.
    #[clap(short, long)]
    ask: bool,

    /// Interactive request
    ///
    /// Make request in "interactive mode". When doing a request in interactive mode, an absent
    /// value for a template variable will not cause an error, instead the user will be propmpted to
    /// input a value. This will done in turn for each of the missing values (if several) until
    /// there are values present for all template variables, so a proper request can be done.
    ///
    /// This implies `--ask` since an interactive request will always ask for confirmation before it
    /// is executed.
    #[clap(short, long)]
    interactive: bool,

    /// Trim user input
    ///
    /// Trim user input from leading or trailing whitespace. So the input ' foo bar  ' becomes
    /// just 'foo bar'.
    #[clap(short, long)]
    pub trim: bool,

    /// Environments
    ///
    /// One or several environments which containins environment variables. If the environment is
    /// `development`, the application will search for any occurence of `development.env` or
    /// `development.sec` in the current directory and parent directories, as long as the search is
    /// confined to the Git repository where the request resides. If the command is not executed
    /// inside a Git repository, no traversing to parental directories will be done.
    ///
    /// Varaibles found in *.env or *.sec files will override the environment variables inherited
    /// from the operating system and in the special `.env`/`.sec` which is a "global" environment
    /// that will be always be included regardless of environment.
    #[clap(short, long)]
    env: Vec<String>,

    /// Disable shell assignments in environment files
    ///
    /// Skip $(command) assignments in all .env and .sec files without executing them.
    /// Other sources can still supply the variable; otherwise it is treated as missing.
    #[clap(long = "no-shell")]
    no_shell: bool,

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

    pub fn try_colors(&self) -> bool {
        match self.use_colors() {
            ColorChoice::Never => false,
            _ => true,
        }
    }

    pub fn file(&self) -> &std::path::Path {
        &self.file
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout as u64)
    }

    pub fn print_request(&self) -> bool {
        self.request || self.ask()
    }

    pub fn print_headers(&self) -> bool {
        self.headers || self.ask()
    }

    pub fn ask(&self) -> bool {
        self.ask || self.interactive()
    }

    pub fn interactive(&self) -> bool {
        self.interactive
    }

    pub fn env(&self) -> Result<Vec<Property>, ParsePropertyError> {
        let sys_envs: Vec<Property> = Self::read_sys_envs()?;
        let file_envs: Vec<Property> = self.read_file_envs()?;
        let arg_vars: Vec<Property> = self.read_arg_vars();

        let alloc_size: usize = sys_envs.len() + file_envs.len() + arg_vars.len();
        let mut props: Vec<Property> = Vec::with_capacity(alloc_size);

        props.extend(sys_envs);
        props.extend(file_envs);
        props.extend(arg_vars);

        Ok(props)
    }

    fn read_sys_envs() -> Result<Vec<Property>, ParsePropertyError> {
        std::env::vars()
            .map(Property::try_from)
            .map(|res| res.map(|prop| prop.with_source(prop::Source::EnvVar)))
            .collect()
    }

    fn read_file_envs(&self) -> Result<Vec<Property>, ParsePropertyError> {
        let file_envs: Result<Vec<Vec<Property>>, ParsePropertyError> =
            Self::find_env_files(&self.file, self.env.clone())
                .into_iter()
                .map(|file| prop::from_file(&file, !self.no_shell))
                .collect();

        file_envs.map(|vec| vec.into_iter().flatten().collect())
    }

    fn read_arg_vars(&self) -> Vec<Property> {
        self.arg_vars
            .clone()
            .into_iter()
            .map(|prop| prop.with_source(prop::Source::Arg))
            .collect()
    }

    fn find_env_files(request_file: &Path, environments: Vec<String>) -> Vec<PathBuf> {
        let mut files: Vec<String> = environments
            .into_iter()
            .flat_map(|env| vec![env.clone() + ".env", env + ".sec"])
            .collect();

        files.push(String::from(".env"));
        files.push(String::from(".sec"));

        let end: PathBuf = request_file.canonicalize().unwrap().parent().unwrap().to_path_buf();

        let start: PathBuf = match giro::git_root(&end).expect("Resolve git root") {
            Some(root) => root,
            None => end.clone(),
        };

        log::info!("Start is {:?}", start);
        log::info!("End is {:?}", end);
        WalkDir::new(start)
            .follow_links(false)
            .contents_first(false)
            .into_iter()
            .filter_entry(|entry| end.starts_with(entry.path()) || entry.file_type().is_file())
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let ftype = entry.file_type();
                if ftype.is_file() {
                    let name: String = entry.file_name().to_str().unwrap().to_string();
                    files.contains(&name)
                } else {
                    false
                }
            })
            .inspect(|e| log::debug!("Found environments file {:?}", e))
            .map(|e| e.into_path())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Args;
    use crate::prop::Property;
    use clap::Parser;

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(16))]

        #[test]
        fn environment_loading_stays_within_repository(depth in 0usize..5) {
            let directory: tempfile::TempDir = tempfile::tempdir().unwrap();
            let root: PathBuf = directory.path().join("repository");
            let status: std::process::ExitStatus = std::process::Command::new("git")
                .args(["init", "--quiet"])
                .arg(&root)
                .status()
                .unwrap();
            proptest::prop_assert!(status.success());
            let request_directory: PathBuf = (0..depth)
                .fold(root.clone(), |path: PathBuf, index: usize| path.join(format!("level_{index}")));
            std::fs::create_dir_all(&request_directory).unwrap();
            let request: PathBuf = request_directory.join("request.yml");
            std::fs::write(&request, "https://example.com").unwrap();
            std::fs::write(root.join(".env"), "root_value=kept\nroot_command=$(printf generated)").unwrap();
            std::fs::write(request_directory.join(".sec"), "request_value=kept").unwrap();
            let filenames: [&str; 4] = [".env", ".sec", "development.env", "development.sec"];
            filenames.iter().enumerate().try_for_each(|(index, filename)| -> std::io::Result<()> {
                let marker: PathBuf = directory.path().join(format!("executed_{index}"));
                std::fs::write(directory.path().join(filename), format!(
                    "FIRE_PARENT_STATIC_32=outside\nFIRE_PARENT_COMMAND_32=$(printf x > '{}')",
                    marker.display()
                ))
            }).unwrap();
            let args: Args = Args::try_parse_from([
                "fire", "--env", "development", request.to_str().unwrap()
            ]).unwrap();
            let properties: Vec<Property> = args.env().unwrap();
            proptest::prop_assert!(filenames.iter().enumerate().all(|(index, _)| {
                !directory.path().join(format!("executed_{index}")).exists()
            }), "Parent commands must not execute");
            proptest::prop_assert!(!properties.iter().any(|property| {
                matches!(property.key(), "FIRE_PARENT_STATIC_32" | "FIRE_PARENT_COMMAND_32")
            }), "Parent assignments must not load");
            proptest::prop_assert!(properties.iter().any(|property| {
                property.key() == "root_value" && property.value() == "kept"
            }), "Repository root assignments must load");
            proptest::prop_assert!(properties.iter().any(|property| {
                property.key() == "root_command" && property.value() == "generated"
            }), "Repository root commands must execute");
            proptest::prop_assert!(properties.iter().any(|property| {
                property.key() == "request_value" && property.value() == "kept"
            }), "Request directory assignments must load");
        }
    }

    #[test]
    fn no_shell_skips_commands_in_all_loaded_files() -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let request: PathBuf = directory.path().join("request.yml");
        let marker: PathBuf = directory.path().join("executed");
        std::fs::write(&request, "https://example.com")?;
        [".env", ".sec", "development.env", "development.sec"]
            .into_iter()
            .try_for_each(|filename: &str| -> std::io::Result<()> {
                std::fs::write(
                    directory.path().join(filename),
                    format!(
                        "static_value=kept\ndynamic_value=\"$(printf x >> '{}'; exit 7)\"",
                        marker.display()
                    ),
                )
            })?;
        let args: Args = Args::try_parse_from([
            "fire",
            "--no-shell",
            "--env",
            "development",
            request.to_str().unwrap(),
        ])?;
        let properties: Vec<Property> = args.env()?;
        assert_eq!(
            properties.iter().filter(|property| property.key() == "static_value").count(),
            4
        );
        assert!(!properties.iter().any(|property| property.key() == "dynamic_value"));
        assert!(!marker.exists());
        Ok(())
    }

    #[test]
    fn shell_assignments_remain_enabled_by_default() -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let request: PathBuf = directory.path().join("request.yml");
        std::fs::write(&request, "https://example.com")?;
        std::fs::write(directory.path().join(".env"), "dynamic_value=$(printf generated)")?;
        let args: Args = Args::try_parse_from(["fire", request.to_str().unwrap()])?;
        let properties: Vec<Property> = args.env()?;
        assert!(properties.iter().any(|property| {
            property.key() == "dynamic_value" && property.value() == "generated"
        }));
        Ok(())
    }

    #[test]
    fn no_shell_keeps_fallback_values_and_reports_missing_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory: tempfile::TempDir = tempfile::tempdir()?;
        let request: PathBuf = directory.path().join("request.yml");
        let system_key: &str = "PATH";
        let system_value: String = std::env::var(system_key)?;
        std::fs::write(&request, "https://example.com")?;
        std::fs::write(
            directory.path().join(".env"),
            format!("{system_key}=$(exit 7)\nstatic_value=fallback\nstatic_value=$(exit 7)\nargument_value=$(exit 7)\nFIRE_NO_SHELL_MISSING_VALUE=$(exit 7)"),
        )?;
        let args: Args = Args::try_parse_from([
            "fire",
            "--no-shell",
            "--variable",
            "argument_value=$(printf literal)",
            request.to_str().unwrap(),
        ])?;
        let properties: Vec<Property> = args.env()?;
        let rendered: String = crate::template::substitution(
            format!("{{{{{system_key}}}}}|{{{{static_value}}}}|{{{{argument_value}}}}"),
            properties.clone(),
            false,
            false,
            false,
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(rendered, format!("{system_value}|fallback|$(printf literal)"));
        let error: crate::template::SubstitutionError = crate::template::substitution(
            "{{FIRE_NO_SHELL_MISSING_VALUE}}".to_string(),
            properties,
            false,
            false,
            false,
        )
        .unwrap_err();
        assert!(
            matches!(error, crate::template::SubstitutionError::MissingValue(key) if key == "FIRE_NO_SHELL_MISSING_VALUE")
        );
        Ok(())
    }
}
