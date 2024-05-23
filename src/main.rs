// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod cache;
mod cli;
mod constants;
mod conversion;
mod hasher;
mod logger;
mod mime;
mod ont_request;
mod util;

use crate::conversion::convert;
use crate::ont_request::DlOrConv;
use crate::ont_request::OntRequest;
use axum::extract::State;
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use cache::*;
use cli_utils::BoxResult;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::trace::TraceLayer;
use tracing_subscriber::filter::LevelFilter;
use util::*;

use git_version::git_version;

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentaton.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub const VERSION: &str = git_version!(cargo_prefix = "", fallback = "unknown");

#[derive(Clone, Debug)]
pub struct Config {
    addr: SocketAddr,
    cache_root: PathBuf,
    prefere_conversion: DlOrConv,
}

fn main() -> BoxResult<()> {
    let log_reload_handle = logger::setup()?;

    let cli_args = cli::parse()?;

    if cli_args.verbose {
        logger::set_log_level(&log_reload_handle, LevelFilter::DEBUG)?;
    } else if cli_args.quiet {
        logger::set_log_level(&log_reload_handle, LevelFilter::WARN)?;
    } else {
        logger::set_log_level(&log_reload_handle, LevelFilter::INFO)?;
    }

    run_proxy(&cli_args.proxy_conf);

    Ok(())
}

#[tokio::main]
async fn run_proxy(config: &Config) {
    create_dir(config.cache_root.as_path()).await;

    // build our application
    let route = Router::new().route("/", get(handler_rdf).with_state(config.clone()));

    // run it
    tokio::join!(serve(route, config.addr));
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| {
            let addition_opt = if addr.port() < 1024 {
                format!(" - You might need root priviledges to listen on port {}, because it is smaller then 1024", addr.port())
            } else {
                String::new()
            };
            format!("Failed to listen on {addr}{addition_opt}: {err}")
        })
        .unwrap();
    tracing::info!("listening on {addr}");
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

async fn handler_rdf(
    State(config): State<Config>,
    ont_request: OntRequest,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let ont_cache_dir = ont_dir(&config.cache_root, &ont_request.uri);
    let ont_file_required = ont_file(&ont_cache_dir, ont_request.mime_type);

    let ont_might_be_cached = ensure_dir_exists(&ont_cache_dir)
        .await
        .map_err(|err| format!("Failed to ensure directory path exists - '{err}'"))
        .unwrap();

    if ont_might_be_cached {
        let ont_file_required_exists = look_for_file(&ont_file_required).await.unwrap();
        if ont_file_required_exists {
            return Ok(respond_with_body(
                &ont_file_required,
                ont_request.mime_type,
                body_from_file(&ont_file_required).await?,
            ));
        }
    }

    // NOTE From here on we know, that the format requested by the client is not cached yet

    match ont_request.pref {
        DlOrConv::Download => {}
        DlOrConv::Convert => {
            let ont_cache_files_found = if ont_might_be_cached {
                search_ont_files(&ont_cache_dir, true).await.unwrap()
            } else {
                vec![]
            };
            if !ont_cache_files_found.is_empty() {
                let annotated_ont_cache_file_found = annotate_ont_files(ont_cache_files_found)
                    .await
                    .map_err(|err| format!("Failed to parse MIME types from cache files - '{err}'"))
                    .unwrap();
                let machine_readable_cached_ont_files: Vec<_> = annotated_ont_cache_file_found
                    .iter()
                    .filter(|ont_cache| mime::Type::is_machine_readable(ont_cache.mime_type))
                    .collect();
                for mr_ont_cache_file in machine_readable_cached_ont_files {
                    // let mtype = mr_ont_cache_file.mime_type;
                    let conversion_res =
                        convert(&ont_request, &ont_cache_dir, mr_ont_cache_file).await;
                    if let Ok(converted) = conversion_res {
                        return body_response(&converted).await;
                    }
                }
            }
        }
    }

    // NOTE At this point we know, that the format requested by the client is producible by convverting from any of the already cached formats (if any)

    let dled_ont = dl_ont(&ont_request, &ont_cache_dir).await?;

    if dled_ont.mime_type == ont_request.mime_type {
        // This is possilbe if we just downloaded the ontology
        Ok(respond_with_body(
            &dled_ont.file,
            ont_request.mime_type,
            body_from_content(dled_ont.content),
        ))
    } else {
        // This is possible, if the ontology server returned a different format then the one we requested
        if dled_ont.mime_type.is_machine_readable() {
            let dled_ont_file = dled_ont.into_ont_file();
            let conversion_res = convert(&ont_request, &ont_cache_dir, &dled_ont_file).await;
            let output_ont_file = conversion_res.map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to convert the downloaded ontology: {err}"),
                )
            })?;
            body_response(&output_ont_file).await
        } else {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
                "As the format returned by the server ({}) is not machine-readable, it cannot be converted into the requested format.",
                dled_ont.mime_type
            )))
        }
    }
}
