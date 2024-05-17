// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod cache;
mod hasher;
mod mime;
mod ont_request;
mod util;

use crate::ont_request::DlOrConv;
use crate::ont_request::OntRequest;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use cache::*;
use std::net::SocketAddr;
use std::path::Path as StdPath;
use tower_http::trace::TraceLayer;
use util::*;

#[tokio::main]
async fn main() {
    init_tracing();

    create_dir(ONTS_CACHE_DIR.as_path()).await;

    // build our application
    let route = Router::new().route("/", get(handler_rdf));

    let addr = [127, 0, 0, 1]; // TODO Make configurable
    let port: u16 = 3000; // TODO Make configurable
    let serving_addr = SocketAddr::from((addr, port));

    // run it
    tokio::join!(serve(route, serving_addr));
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", addr);
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

async fn try_convert(
    ont_request: &OntRequest,
    ont_cache_dir: &StdPath,
    cached_ont: &OntFile,
) -> Result<(HeaderMap, Body), (StatusCode, String)> {
    if cached_ont.mime_type.is_machine_readable() {
        let ont_requested_file = ont_file(ont_cache_dir, ont_request.mime_type);
        if ont_request.mime_type == mime::Type::Html {
            to_html_conversion(
                cached_ont.mime_type,
                ont_request.mime_type,
                &cached_ont.file,
                &ont_requested_file,
            )
            .await?;
            return Ok(respond_with_body(
                &ont_requested_file,
                ont_request.mime_type,
                body_from_file(&ont_requested_file).await?,
            ));
        }

        match (
            to_rdflib_format(cached_ont.mime_type),
            to_rdflib_format(ont_request.mime_type),
        ) {
            (Some(cached_rdflib_type), Some(requested_rdflib_type)) => {
                rdf_convert(
                    cached_rdflib_type,
                    requested_rdflib_type,
                    &cached_ont.file,
                    &ont_requested_file,
                )
                .await?;
                Ok(respond_with_body(
                    &ont_requested_file,
                    ont_request.mime_type,
                    body_from_file(&ont_requested_file).await?,
                ))
            }
            _ => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Can not convert {} to {}",
                    cached_ont.mime_type, ont_request.mime_type
                ),
            )),
        }
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
            "As the cached format of this ontology ({}) is not machine-readable, it cannot be converted into the requested format.",
            cached_ont.mime_type
        )))
    }
}

async fn handler_rdf(ont_request: OntRequest) -> Result<impl IntoResponse, impl IntoResponse> {
    let ont_cache_dir = ont_dir(&ont_request.uri);
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
                body_from_file(&ont_file_required)
                    .await
                    .map_err(IntoResponse::into_response)?,
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
                    .map_err(|err| format!("Failed to parse MIME types from cache files - '{err}'"))
                    .unwrap();
                let machine_readable_cached_ont_files: Vec<_> = annotated_ont_cache_file_found
                    .iter()
                    .filter(|ont_cache| mime::Type::is_machine_readable(ont_cache.mime_type))
                    .collect();
                for mr_ont_cache_file in machine_readable_cached_ont_files {
                    // let mtype = mr_ont_cache_file.mime_type;
                    let conversion_res =
                        try_convert(&ont_request, &ont_cache_dir, mr_ont_cache_file).await;
                    if let Ok(converted) = conversion_res {
                        return Ok(converted);
                    }
                }
            }
        }
    }

    // NOTE At this point we know, that the format requested by the client is producible by convverting from any of the already cached formats (if any)

    let dled_ont = dl_ont(&ont_request, &ont_cache_dir)
        .await
        .map_err(IntoResponse::into_response)?;

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
            let conversion_res = try_convert(&ont_request, &ont_cache_dir, &dled_ont_file).await;
            conversion_res.map_err(IntoResponse::into_response)
        } else {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
                "As the format returned by the server ({}) is not machine-readable, it cannot be converted into the requested format.",
                dled_ont.mime_type
            )).into_response())
        }
    }
}
