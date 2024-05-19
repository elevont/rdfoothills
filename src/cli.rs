// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::IpAddr, path::PathBuf};

use clap::{builder::OsStr, command, Arg, ArgAction, Command, ValueHint};
use cli_utils::BoxResult;
use const_format::formatcp;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::{
    constants::{DEFAULT_ADDRESS, DEFAULT_CACHE_ROOT, DEFAULT_PORT},
    ont_request::DlOrConv,
    Config,
};

pub const A_S_VERSION: char = 'V';
pub const A_L_VERSION: &str = "version";
pub const A_S_QUIET: char = 'q';
pub const A_L_QUIET: &str = "quiet";
pub const A_S_VERBOSE: char = 'v';
pub const A_L_VERBOSE: &str = "verbose";
pub const A_S_PORT: char = 'p';
pub const A_L_PORT: &str = "port";
pub const A_S_ADDR: char = 'a';
pub const A_L_ADDR: &str = "address";
pub const A_S_CACHE_DIR: char = 'c';
pub const A_L_CACHE_DIR: &str = "cache-dir";
pub const A_S_PREFERE_CONVERSION: char = 'C';
pub const A_L_PREFERE_CONVERSION: &str = "prefere-conversion";

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

fn arg_port() -> Arg {
    Arg::new(A_L_PORT)
        .help("the IP port to host this service on")
        .num_args(1)
        .short(A_S_PORT)
        .long(A_L_PORT)
        .action(ArgAction::Set)
        .value_hint(ValueHint::Other)
        .value_name("PORT")
        .default_value(OsStr::from(DEFAULT_PORT.to_string()))
}

fn arg_addr() -> Arg {
    Arg::new(A_L_ADDR)
        .help("the IP address (v4 or v6) to host this service on")
        .num_args(1)
        .short(A_S_ADDR)
        .long(A_L_ADDR)
        .action(ArgAction::Set)
        .value_hint(ValueHint::Other)
        .value_name("IP_ADDRESS")
        .default_value(DEFAULT_ADDRESS)
}

fn arg_cache_dir() -> Arg {
    Arg::new(A_L_CACHE_DIR)
        .help("a variable key-value pair to be used for substitution in the text")
        .num_args(1)
        .short(A_S_CACHE_DIR)
        .long(A_L_CACHE_DIR)
        .action(ArgAction::Set)
        .value_hint(ValueHint::DirPath)
        .value_name("DIR_PATH")
        .default_value(DEFAULT_CACHE_ROOT.as_os_str())
}

fn arg_prefere_conversion() -> Arg {
    Arg::new(A_L_PREFERE_CONVERSION)
        .help("Preffer conversion from a cached format over downloading the requested format directly from the supplied URI.")
        .short(A_S_PREFERE_CONVERSION)
        .long(A_L_PREFERE_CONVERSION)
        .action(ArgAction::SetTrue)
}

pub fn args_matcher() -> Command {
    command!()
        .about(clap::crate_description!())
        .bin_name(clap::crate_name!())
        .help_expected(true)
        .disable_version_flag(true)
        .arg(arg_version())
        .arg(arg_verbose())
        .arg(arg_quiet())
        .arg(arg_port())
        .arg(arg_addr())
        .arg(arg_cache_dir())
        .arg(arg_prefere_conversion())
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
    pub proxy_conf: Config,
}

pub fn parse() -> BoxResult<Args> {
    let args = args_matcher().get_matches();

    let quiet = args.get_flag(A_L_QUIET);
    let version = args.get_flag(A_L_VERSION);
    if version {
        print_version_and_exit(quiet);
    }

    let verbose = args.get_flag(A_L_VERBOSE);
    let port = args
        .get_one::<String>(A_L_PORT)
        .map(|port_str| port_str.parse())
        .transpose()?
        .unwrap_or(DEFAULT_PORT);
    let ip_addr_str = args
        .get_one::<String>(A_L_ADDR)
        .cloned()
        .unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let ip_addr = IpAddr::from_str(&ip_addr_str)?;
    let addr = SocketAddr::from((ip_addr, port));
    let cache_root = args
        .get_one::<String>(A_L_CACHE_DIR)
        .map_or(DEFAULT_CACHE_ROOT.clone(), PathBuf::from);
    let prefere_conversion_bool = args.get_flag(A_L_PREFERE_CONVERSION);
    let prefere_conversion = if prefere_conversion_bool {
        DlOrConv::Convert
    } else {
        DlOrConv::Download
    };

    let parsed_args = Args {
        quiet,
        verbose,
        proxy_conf: Config {
            addr,
            cache_root,
            prefere_conversion,
        },
    };

    Ok(parsed_args)
}
