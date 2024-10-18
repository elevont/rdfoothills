// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use clap::{command, value_parser, Arg, ArgAction, Command, ValueHint};
use const_format::formatcp;

use crate::config::Config;

pub const A_S_VERSION: char = 'V';
pub const A_L_VERSION: &str = "version";
pub const A_S_QUIET: char = 'q';
pub const A_L_QUIET: &str = "quiet";
pub const A_S_VERBOSE: char = 'v';
pub const A_L_VERBOSE: &str = "verbose";
pub const A_S_FORCE: char = 'f';
pub const A_L_FORCE: &str = "force";
pub const A_S_HEADER: char = 'H';
pub const A_L_HEADER: &str = "header";
pub const A_S_OUT_DIR: char = 'O';
pub const A_L_OUT_DIR: &str = "output-directory";
// pub const A_S_IN_FILE: char = 'I';
pub const A_L_IN_FILE: &str = "ontology-file";

fn arg_version() -> Arg {
    Arg::new(A_L_VERSION)
        .help(formatcp!(
            "Print version information and exit. \
May be combined with -{A_S_QUIET},--{A_L_QUIET}, \
to really only output the version string."
        ))
        .short(A_S_VERSION)
        .long(A_L_VERSION)
        .action(ArgAction::SetTrue)
}

fn arg_quiet() -> Arg {
    Arg::new(A_L_QUIET)
        .help("Minimize or suppress output to stderr")
        .long_help("Minimize or suppress output to stderr; stdout is never used by this program, with or without this option set.")
        .action(ArgAction::SetTrue)
        .short(A_S_QUIET)
        .long(A_L_QUIET)
        .conflicts_with(A_L_VERBOSE)
}

fn arg_verbose() -> Arg {
    Arg::new(A_L_VERBOSE)
        .help("more verbose output (useful for debugging)")
        .short(A_S_VERBOSE)
        .long(A_L_VERBOSE)
        .action(ArgAction::SetTrue)
}

fn arg_force() -> Arg {
    Arg::new(A_L_FORCE)
        .help("forces overwriting potentially already existing output files")
        .short(A_S_FORCE)
        .long(A_L_FORCE)
        .action(ArgAction::SetTrue)
}

fn arg_header() -> Arg {
    Arg::new(A_L_HEADER)
        .help("The text to insert on top of all output files (generated Rust source code)")
        .short(A_S_HEADER)
        .long(A_L_HEADER)
        .action(ArgAction::Set)
        .value_hint(ValueHint::Other)
        .value_name("TEXT")
}

fn arg_out_dir() -> Arg {
    Arg::new(A_L_OUT_DIR)
        .help("The output directory, where Rust source files get written to")
        .short(A_S_OUT_DIR)
        .long(A_L_OUT_DIR)
        .action(ArgAction::Set)
        .value_parser(value_parser!(std::path::PathBuf))
        .value_hint(ValueHint::DirPath)
        .value_name("OUT_DIR")
        .required_unless_present(A_L_VERSION)
}

fn arg_in_file() -> Arg {
    Arg::new(A_L_IN_FILE)
        .help("The input OWL input file(s)")
        // .short(A_S_IN_FILE)
        // .long(A_L_IN_FILE)
        .action(ArgAction::Set)
        .value_parser(value_parser!(std::path::PathBuf))
        .value_hint(ValueHint::FilePath)
        .value_name("OWL_FILE")
        .required_unless_present(A_L_VERSION)
        .num_args(1..)
}

#[must_use]
pub fn args_matcher() -> Command {
    command!()
        .about(clap::crate_description!())
        .bin_name(clap::crate_name!())
        .help_expected(true)
        .disable_version_flag(true)
        .arg(arg_version())
        .arg(arg_quiet())
        .arg(arg_verbose())
        .arg(arg_force())
        .arg(arg_header())
        .arg(arg_out_dir())
        .arg(arg_in_file())
}

#[allow(clippy::print_stdout)]
fn print_version_and_exit(quiet: bool) {
    if !quiet {
        print!("{} ", clap::crate_name!());
    }
    println!("{}", crate::VERSION);
    std::process::exit(0);
}

#[derive(Clone, Debug)]
pub struct Args {
    pub quiet: bool,
    pub verbose: bool,
    pub config: Config,
}

/// Parses the command line arguments,
/// including verification.
///
/// # Panics
///
/// - The output directory was not supplied
/// - No input file/ontology was supplied
#[must_use]
pub fn parse() -> Args {
    let args = args_matcher().get_matches();

    let quiet = args.get_flag(A_L_QUIET);
    let version = args.get_flag(A_L_VERSION);
    if version {
        print_version_and_exit(quiet);
    }

    let verbose = args.get_flag(A_L_VERBOSE);
    let force = args.get_flag(A_L_FORCE);
    let header = args.get_one::<String>(A_L_HEADER).cloned();
    let out_dir = args
        .get_one::<PathBuf>(A_L_OUT_DIR)
        .cloned()
        .expect("The output directory is required");
    let in_files: Vec<PathBuf> = args
        .get_many(A_L_IN_FILE)
        .expect("At least one OWL input file (in RDF/Turtle format) is required")
        .cloned()
        .collect();

    let config = Config {
        ontologies: in_files,
        out_dir,
        force,
        header,
    };

    Args {
        quiet,
        verbose,
        config,
    }
}
