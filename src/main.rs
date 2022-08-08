mod args;
mod dbg;
mod fmt;
mod logger;

use crate::args::Args;
use crate::dbg::dbg_info;
use crate::fmt::write;
use crate::logger::setup_logging;
use clap::Parser;
use std::process;
use termcolor::{Color, StandardStream};

fn main() {
    let args: Args = Args::parse();
    setup_logging(args.verbosity_level);
    log::debug!("Config: {:?}", args);
    log::info!("This is a log message");

    if args.print_dbg {
        println!("{}", dbg_info());
        process::exit(0);
    }

    let mut stdout = StandardStream::stdout(args.use_colors());
    write(&mut stdout, "Hello, world!\n", Some(Color::Red));
}
