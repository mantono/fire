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

    /// Enable/disable colors
    ///
    /// Enable or disable output with colors. By default colors will be used if the terminal seems
    /// to support colors.
    #[clap(short = 'C', long = "colors")]
    use_colors: Option<Colors>,
}

impl Args {
    pub fn use_colors(&self) -> ColorChoice {
        match self.use_colors {
            None => ColorChoice::Auto,
            Some(Colors::Always) => ColorChoice::Always,
            Some(Colors::AlwaysAnsi) => ColorChoice::AlwaysAnsi,
            Some(Colors::Auto) => ColorChoice::Auto,
            Some(Colors::Never) => ColorChoice::Never,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Colors {
    Always,
    AlwaysAnsi,
    Auto,
    Never,
}

impl std::str::FromStr for Colors {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "always" => Ok(Colors::Always),
            "ansi" => Ok(Colors::AlwaysAnsi),
            "auto" => Ok(Colors::Auto),
            "never" => Ok(Colors::Never),
            _ => Err("Invalid color choice"),
        }
    }
}
