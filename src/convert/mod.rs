// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cache::OntFile;
use crate::mime;
use crate::respond_with_body;

use crate::cache::ont_file;
use crate::ont_request::OntRequest;
use crate::util::body_from_file;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode},
};
use std::path::Path as StdPath;

pub const fn to_rdflib_format(mime_type: mime::Type) -> Option<&'static str> {
    match mime_type {
        mime::Type::BinaryRdf
        | mime::Type::Csvw
        | mime::Type::Hdt
        | mime::Type::Html
        | mime::Type::Microdata
        | mime::Type::NdJsonLd
        | mime::Type::NQuadsStar
        | mime::Type::NTriplesStar
        | mime::Type::RdfA
        | mime::Type::RdfJson
        | mime::Type::TriGStar
        | mime::Type::OwlFunctional
        | mime::Type::OwlXml
        | mime::Type::Tsvw
        | mime::Type::TurtleStar
        | mime::Type::YamlLd => None,
        mime::Type::HexTuples => Some("hext"),
        // mime::Type::Html => Some("rdfa"),
        mime::Type::JsonLd => Some("json-ld"),
        mime::Type::N3 => Some("n3"),
        mime::Type::NQuads => Some("nquads"),
        mime::Type::NTriples => Some("nt"),
        // mime::Type::RdfA => Some("rdfa"),
        mime::Type::TriG => Some("trig"),
        mime::Type::RdfXml => Some("xml"),
        // mime::Type::RdfXml => Some("pretty-xml"),
        mime::Type::TriX => Some("trix"),
        mime::Type::Turtle => Some("turtle"),
    }
}

pub async fn cli_cmd(cmd: &str, task: &str, args: &[&str]) -> Result<(), (StatusCode, String)> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to run {cmd} for {task}: {err}"),
            )
        })?;
    if !output.status.success() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Running {cmd} for {task} returned with non-zero exit status '{}', indicating an error. stderr:\n{}",
                output.status.code().map_or("<none>".to_string(), |code| i32::to_string(&code)),
                String::from_utf8_lossy(&output.stderr),
        )));
    }

    Ok(())
}

pub async fn pylode(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd("pylode", "RDF to HTML conversion", args).await
}

pub async fn rdf_tools(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd(
        "rdf-convert",
        "RDF format conversion (from/with pkg: 'rdftools')",
        args,
    )
    .await
}

pub async fn rdfx(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd("rdfx", "RDF format conversion", args).await
}

pub async fn to_html_conversion(
    _cached_type: mime::Type,
    _requested_type: mime::Type,
    cached_file: &StdPath,
    requested_file: &StdPath,
) -> Result<(), (StatusCode, String)> {
    pylode(&[
        "--sort",
        "--css",
        "true",
        "--profile",
        "ontpub",
        "--outputfile",
        requested_file.as_os_str().to_str().unwrap(),
        cached_file.as_os_str().to_str().unwrap(),
    ])
    .await
}

pub async fn rdf_convert(
    cached_type: &str,
    requested_type: &str,
    cached_file: &StdPath,
    requested_file: &StdPath,
) -> Result<(), (StatusCode, String)> {
    let use_rdfx = false;
    if use_rdfx {
        rdfx(&[
            "convert",
            "--format",
            requested_type,
            "--output",
            requested_file.as_os_str().to_str().unwrap(),
            cached_file.as_os_str().to_str().unwrap(),
        ])
        .await
    } else {
        rdf_tools(&[
            "--input",
            cached_file.as_os_str().to_str().unwrap(),
            "--output",
            requested_file.as_os_str().to_str().unwrap(),
            "--read",
            cached_type,
            "--write",
            requested_type,
        ])
        .await
    }
}

pub async fn try_convert(
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
