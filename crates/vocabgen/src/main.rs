// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_crate_dependencies)]

mod cli;

use cli_utils::logging;
use cli_utils::BoxResult;
pub use rdfoothills_vocabgen as vocabgen;
use tracing::metadata::LevelFilter;
pub use vocabgen::config;

pub use vocabgen::VERSION;

fn main() -> BoxResult<()> {
    let log_reload_handle = logging::setup(clap::crate_name!())?;

    let cli_args = cli::parse();

    let log_level = if cli_args.verbose {
        LevelFilter::DEBUG
    } else if cli_args.quiet {
        LevelFilter::WARN
    } else {
        LevelFilter::INFO
    };
    logging::set_log_level_tracing(&log_reload_handle, log_level)?;

    vocabgen::generate(&cli_args.config)?;

    Ok(())
}
