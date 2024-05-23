// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::hasher;
use crate::mime;
use crate::ont_request::OntRequest;
use crate::util::*;
use axum::http::{header::CONTENT_TYPE, StatusCode};
use futures::future::join_all;
use mediatype::MediaType;
use reqwest::Url;
use std::ffi::OsStr;
use std::io;
use std::{path::Path as StdPath, path::PathBuf};

pub const ONT_FILE_PREFIX: &str = "ontology";

pub fn ont_dir(cache_root: &StdPath, uri: &Url) -> PathBuf {
    let url_nameified = url2fname(uri);
    // NOTE Because the nameified version of the URL could be equal
    //      for different URLs, we append its hash.
    let url_hash = hasher::hash_num(uri);
    let url_dir_name = format!("{url_nameified}-{url_hash}");

    cache_root.join("ontologies").join(url_dir_name)
}

pub async fn search_ont_files(ont_cache_dir: &StdPath, all: bool) -> io::Result<Vec<PathBuf>> {
    let mut dir_reader = tokio::fs::read_dir(ont_cache_dir).await?;

    let mut ont_files = vec![];
    while let Some(entry) = dir_reader.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            // let file_prefix_opt = path.file_prefix().and_then(OsStr::to_str); // TODO Use this, once it is in rust stable
            let file_prefix_opt = path.file_stem().and_then(OsStr::to_str);
            if file_prefix_opt.is_some_and(|file_stem| file_stem == ONT_FILE_PREFIX) {
                ont_files.push(path);
                if !all {
                    break;
                }
            }
        }
    }

    Ok(ont_files)
}

/// Given a list of paths to ontologie files,
/// gathers additional info about them, and returns the result,
/// including the paths.
/// This additional info includes the MIME type of the files.
///
/// # Errors
///
/// Will return `ParseError::NoKnownFileExtensionAndReadError` if the file has no extension adn we failed to read the file.
/// Will return `ParseError::UnrecognizedFileExtension` if the extension is not supported.
/// Will return `ParseError::UnidentifiedContent` if the content is not recognized.
/// Will return `ParseError::UnrecognizedContent` if the content is recognized but not supported.
pub async fn annotate_ont_files(ont_files: Vec<PathBuf>) -> Result<Vec<OntFile>, mime::ParseError> {
    join_all(ont_files.into_iter().map(|file| async {
        let mime_type = mime::Type::from_path(&file).await?;
        Ok::<OntFile, mime::ParseError>(OntFile { file, mime_type })
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
}

pub async fn look_for_ont_file(
    ont_cache_dir: &StdPath,
    mime_type: mime::Type,
) -> io::Result<Option<PathBuf>> {
    let ont_file_path = ont_file(ont_cache_dir, mime_type);
    look_for_file(&ont_file_path)
        .await
        .map(|exists| if exists { Some(ont_file_path) } else { None })
}

pub fn ont_file(ont_cache_dir: &StdPath, mime_type: mime::Type) -> PathBuf {
    ont_cache_dir.join(format!("{ONT_FILE_PREFIX}.{}", mime_type.file_ext()))
}

pub struct OntFile {
    pub file: PathBuf,
    pub mime_type: mime::Type,
}

pub struct OntCacheFile {
    pub file: PathBuf,
    pub mime_type: mime::Type,
    pub content: Vec<u8>,
}

impl OntCacheFile {
    pub fn into_ont_file(self) -> OntFile {
        OntFile {
            file: self.file,
            mime_type: self.mime_type,
        }
    }
}

pub async fn dl_ont(
    ont_request: &OntRequest,
    ont_cache_dir: &StdPath,
) -> Result<OntCacheFile, (StatusCode, String)> {
    let mut rdf_dl_req = reqwest::Client::new().get(ont_request.uri.clone());
    if let Some(query_mime_type) = ont_request.query_mime_type {
        rdf_dl_req = rdf_dl_req.header(reqwest::header::ACCEPT, query_mime_type.mime_type());
    }
    let rdf_dl_resp = rdf_dl_req.send().await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed download from the supplied URI: {err}"),
        )
    })?;
    let resp_ctype = rdf_dl_resp
        .headers()
        .get(CONTENT_TYPE)
        .map(|value| value.to_str())
        .transpose()
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Content type contains invisible ASCII chars: {err}"),
            )
        })?;
    let resp_rdf_mime_type_opt = resp_ctype
        .map(MediaType::parse)
        .transpose()
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!(
            "Failed to parse content-type returned when downloading from the supplied URI as MIME type: {err}")))?
        .map(|media_type| {
            let media_type_essence = MediaType::essence(&media_type);
            let mime_type_res = mime::Type::from_media_type(&media_type_essence);
            if let Err(mime::ParseError::CouldBeAny(_)) = mime_type_res {
                Ok(None)
            } else {
                mime_type_res.map(Some)
            }
        })
        .transpose()
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!(
            "Failed to parse content-type returned when downloading from the supplied URI as RDF MIME type: {err}")))?
        .unwrap_or_default();
    let rdf_bytes = rdf_dl_resp.bytes().await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to extract content when downloading from the supplied URI: {err}"),
        )
    })?;
    let resp_rdf_mime_type = if let Some(mtype) = resp_rdf_mime_type_opt {
        mtype
    } else {
        let uri_path = PathBuf::from(ont_request.uri.path());
        let url_file_ext_opt = extract_file_ext(&uri_path);
        let file_ext_mtype_opt =
            url_file_ext_opt.and_then(|url_file_ext| mime::Type::from_file_ext(url_file_ext).ok());
        if let Some(file_ext_mtype) = file_ext_mtype_opt {
            file_ext_mtype
        } else {
            mime::Type::from_content(rdf_bytes.as_ref())
                .or_else(|err| {
                    if let Some(query_mime_type) = ont_request.query_mime_type {
                        Ok(query_mime_type)
                    } else {
                        Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
                            "Generic result content-type supplied by ontology server, and we were unable to determine the actual content-type from the returned content: {err}")))
                    }
                })?
        }
    };
    let ont_file_dl = ont_file(ont_cache_dir, resp_rdf_mime_type);
    tokio::fs::write(&ont_file_dl, &rdf_bytes)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
            "Failed writing content downloaded from the supplied URI to the cache on disc: {err}"),
            )
        })?;
    Ok(OntCacheFile {
        file: ont_file_dl,
        mime_type: resp_rdf_mime_type,
        content: rdf_bytes.as_ref().to_owned(),
    })
}
