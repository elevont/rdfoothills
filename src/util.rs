// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::CONTENT_DISPOSITION;
use std::ffi::OsStr;
use std::io;
use std::path::Path as StdPath;
use tokio_util::io::ReaderStream;
use url::Url;

use crate::mime;

pub async fn create_dir<P: AsRef<StdPath> + Send>(dir: P) {
    create_dir_res(dir.as_ref())
        .await
        .map_err(|err| {
            panic!(
                "Failed to create directory `{}`: {err}",
                dir.as_ref().display()
            )
        })
        .unwrap();
}

pub async fn create_dir_res<P: AsRef<StdPath> + Send>(dir: P) -> io::Result<()> {
    tokio::fs::create_dir_all(dir).await.or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}

pub static NON_BASIC_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]").unwrap());

pub fn url2fname(url: &Url) -> String {
    let url_str = url.as_str();
    let url_nameified = NON_BASIC_CHARS.replace_all(url_str, "_");
    url_nameified.into()
}

pub async fn look_for_file(file_path: &StdPath) -> io::Result<bool> {
    let path_exists = tokio::fs::try_exists(&file_path).await?;
    if path_exists
        && !tokio::fs::metadata(&file_path)
            .await?
            // .map_err(|err| format!("Failed to check if directory path '{}' is a directory - '{err}'", dir_path.display()))
            .is_file()
    {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Should be an file, but is not: '{}' - possible solution: delete it",
                file_path.display()
            ),
        ));
    }
    Ok(path_exists)
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

pub fn respond_with_body(file: &StdPath, mime_type: mime::Type, body: Body) -> (HeaderMap, Body) {
    let mut headers = HeaderMap::new();

    // headers.insert(CONTENT_TYPE, "text/toml; charset=utf-8".parse().unwrap());
    headers.insert(CONTENT_TYPE, mime_type.mime_type().parse().unwrap());
    headers.insert(
        CONTENT_DISPOSITION,
        format!(
            "attachment; filename=\"{}\"",
            file.file_name().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
    );

    (headers, body)
}

pub async fn body_from_file(file: &StdPath) -> Result<Body, (StatusCode, String)> {
    // `File` implements `AsyncRead`
    let file_handl = match tokio::fs::File::open(file).await {
        Ok(file_handl) => file_handl,
        Err(err) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("File '{}' not found: {err}", file.display()),
            ))
        }
    };
    // convert the `AsyncRead` into a `Stream`
    let stream = ReaderStream::new(file_handl);
    // convert the `Stream` into an `axum::body::HttpBody`
    Ok(Body::from_stream(stream))
}

pub fn body_from_content(ont_content: Vec<u8>) -> Body {
    Body::from(ont_content)
}

/// Ensures the provided dir exists.
/// Returns whether it was created.
pub async fn ensure_dir_exists(dir_path: &StdPath) -> io::Result<bool> {
    let dir_path_exists = tokio::fs::try_exists(&dir_path)
        .await
        // .map_err(|err| format!("Failed to check if directory path '{}' exists - '{err}'", dir_path.display()))
        ?;
    if dir_path_exists {
        if !tokio::fs::metadata(&dir_path)
            .await?
            // .map_err(|err| format!("Failed to check if directory path '{}' is a directory - '{err}'", dir_path.display()))
            .is_dir()
        {
            return Err(io::Error::new(
                // io::ErrorKind::NotADirectory,
                io::ErrorKind::Other,
                format!("Should be an ontology cache directory, but is not a directory: '{}' - possible solution: delete it", dir_path.display())));
        }
    } else {
        tokio::fs::create_dir_all(&dir_path)
            .await
            // .map_err(|err| format!("Failed to create directory '{}' is a directory - '{err}'", dir_path.display()))
            ?;
    }
    Ok(!dir_path_exists)
}

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

pub fn extract_file_ext(file: &StdPath) -> Option<&str> {
    file.extension().and_then(OsStr::to_str)
}
