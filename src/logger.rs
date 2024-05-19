// SPDX-FileCopyrightText: 2021 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use cli_utils::BoxResult;
use std::io;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::Layered,
    prelude::*,
    reload::{self, Handle},
    EnvFilter, Registry,
};

type ReloadHandle = Handle<LevelFilter, Layered<EnvFilter, Registry, Registry>>;

/// Sets up logging, with a way to change the log level later on,
/// and with all output going to stderr,
/// as suggested by <https://clig.dev/>.
///
/// # Errors
///
/// If initializing the registry (logger) failed.
pub fn setup() -> BoxResult<ReloadHandle> {
    // NOTE It is crucial to first set the lowest log level,
    //      as apparently, any level that is lower then this one
    //      will be ignored when trying to set it later on.
    //      Later though, the level can be changed up and down as desired.
    let level_filter = LevelFilter::TRACE;
    let (filter, reload_handle_filter) = reload::Layer::new(level_filter);

    let l_stderr = fmt::layer().map_writer(move |_| io::stderr);

    let registry = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ontprox=debug,tower_http=debug".into()),
        )
        .with(filter)
        .with(l_stderr);
    registry.try_init()?;

    Ok(reload_handle_filter)
}

pub fn set_log_level(reload_handle: &ReloadHandle, level: LevelFilter) -> BoxResult<()> {
    reload_handle.modify(|filter| *filter = level)?;
    Ok(())
}
