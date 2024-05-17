// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod hasher;
mod mime;
mod util;

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Query},
    http::{
        header::ACCEPT, header::CONTENT_TYPE, request::Parts, HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::get,
    RequestPartsExt, Router,
};
use mediatype::MediaType;
use once_cell::sync::Lazy;
use std::{collections::HashMap, path::Path as StdPath, path::PathBuf};
use std::{ffi::OsStr, net::SocketAddr};
use std::{io, str::FromStr};
use tower_http::trace::TraceLayer;
use url::Url;
use util::*;

pub static CACHE_ROOT: Lazy<PathBuf> = Lazy::new(|| dirs::cache_dir().unwrap().join("ont-serv"));
pub static ONTS_CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| CACHE_ROOT.join("ontologies"));
pub const ONT_FILE_PREFIX: &str = "ontology";

#[tokio::main]
async fn main() {
    init_tracing();

    create_dir(ONTS_CACHE_DIR.as_path()).await;

    // build our application
    let route = Router::new().route("", get(handler_rdf));

    let addr = [127, 0, 0, 1]; // TODO Make configurable
    let port: u16 = 3000; // TODO Make configurable
    let serving_addr = SocketAddr::from((addr, port));

    // run it
    tokio::join!(serve(route, serving_addr));
}

async fn serve(app: Router, addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

#[derive(Debug, Copy, Clone)]
enum DlOrConv {
    Download,
    Convert,
}

#[derive(Debug)]
struct OntRequest {
    /// The original ontologies URI.
    uri: Url,
    /// The MIME type to be requested.
    /// It will be sent in the HTTP header `Accept`,
    /// when downloading from the supplied URI.
    query_mime_type: Option<mime::Type>,
    /// The MIME type requested.
    /// This is what our client wants,
    /// and what we try to sent to it.
    mime_type: mime::Type,
    /// If the requested format is not present yet,
    /// which action to preffer to get it.
    /// The other action will be tried if the first one fails.
    pref: DlOrConv,
}

fn extract_requested_content_type(headers: &HeaderMap) -> Result<mime::Type, Response> {
    let content_type_str = headers
        .get(ACCEPT)
        .map(|ctype| {
            HeaderValue::to_str(ctype).map_err(|err| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Failed to convert header value for 'content-type' to string: {err}"),
                )
                    .into_response()
            })
        })
        .transpose()?;
    let mime_type = content_type_str
        .map(|ctype_str| mime::Type::from_str(ctype_str).map_err(|err|
            (StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!("Failed to parse requested content-type '{ctype_str}' to an RDF MIME type: {err}")
            ).into_response()))
        .transpose()?
        .unwrap_or_default();

    Ok(mime_type)
}

fn extract_uri(query_params: &Query<HashMap<String, String>>) -> Result<Url, Response> {
    let uri_str = query_params
        .get("uri")
        .ok_or_else(|| (StatusCode::NOT_FOUND, "'uri' param missing").into_response())?;
    let uri = Url::parse(uri_str).map_err(|err| {
        (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("'uri' is invalid: {err}"),
        )
            .into_response()
    })?;

    Ok(uri)
}

fn extract_query_accept(
    query_params: &Query<HashMap<String, String>>,
) -> Result<Option<mime::Type>, Response> {
    let query_mime_type = query_params
        .get("query-accept")
        .map(|ctype_str| mime::Type::from_str(ctype_str).map_err(|err|
            (StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!("Failed to parse content-type to be requested '{ctype_str}' to an RDF MIME type: {err}")
            ).into_response()))
        .transpose()?;

    Ok(query_mime_type)
}

#[async_trait]
impl<S> FromRequestParts<S> for OntRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query_params: Query<HashMap<String, String>> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        let headers: HeaderMap = parts.extract().await.map_err(IntoResponse::into_response)?;

        let mime_type = extract_requested_content_type(&headers)?;
        let uri = extract_uri(&query_params)?;
        let query_mime_type = extract_query_accept(&query_params)?;

        let pref = DlOrConv::Download; // TODO Maybe we want to allow setting this with a query parameter as well?
        Ok(Self {
            uri,
            query_mime_type,
            mime_type,
            pref,
        })
    }
}

async fn search_ont_files(ont_cache_dir: &StdPath, all: bool) -> io::Result<Vec<PathBuf>> {
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

fn annotate_ont_files(ont_files: Vec<PathBuf>) -> Result<Vec<OntFile>, mime::ParseError> {
    ont_files
        .into_iter()
        .map(|file| {
            let mime_type = mime::Type::from_path(&file)?;
            Ok::<OntFile, mime::ParseError>(OntFile { file, mime_type })
        })
        .collect::<Result<Vec<_>, _>>()
}

async fn look_for_ont_file(
    ont_cache_dir: &StdPath,
    mime_type: mime::Type,
) -> io::Result<Option<PathBuf>> {
    let ont_file_path = ont_cache(ont_cache_dir, mime_type);
    look_for_file(&ont_file_path)
        .await
        .map(|exists| if exists { Some(ont_file_path) } else { None })
}

fn ont_cache(ont_cache_dir: &StdPath, mime_type: mime::Type) -> PathBuf {
    ont_cache_dir.join(format!("{ONT_FILE_PREFIX}.{}", mime_type.file_ext()))
}

struct OntFile {
    file: PathBuf,
    mime_type: mime::Type,
}

struct OntCacheFile {
    file: PathBuf,
    mime_type: mime::Type,
    content: Vec<u8>,
}

impl OntCacheFile {
    pub fn into_ont_file(self) -> OntFile {
        OntFile {
            file: self.file,
            mime_type: self.mime_type,
        }
    }
}

async fn dl_ont(
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
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse content-type returned when downloading from the supplied URI as MIME type: {err}")))?
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
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse content-type returned when downloading from the supplied URI as RDF MIME type: {err}")))?
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
    let ont_file_dl = ont_cache(ont_cache_dir, resp_rdf_mime_type);
    tokio::fs::write(&ont_file_dl, &rdf_bytes)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed writing content downloaded from the supplied URI to the cache on disc: {err}")))?;
    Ok(OntCacheFile {
        file: ont_file_dl,
        mime_type: resp_rdf_mime_type,
        content: rdf_bytes.as_ref().to_owned(),
    })
}

async fn try_convert(
    ont_request: &OntRequest,
    ont_cache_dir: &StdPath,
    cached_ont: &OntFile,
) -> Result<(HeaderMap, Body), (StatusCode, String)> {
    if cached_ont.mime_type.is_machine_readable() {
        let ont_requested_file = ont_cache(ont_cache_dir, ont_request.mime_type);
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
    let url_nameified = url2fname(&ont_request.uri);
    // NOTE Because the nameified version of the URL could be equal
    //      for different URLs, we append its hash.
    let url_hash = hasher::hash_num(&ont_request.uri);
    let url_dir_name = format!("{url_nameified}-{url_hash}");

    let ont_cache_dir = ONTS_CACHE_DIR.join(url_dir_name);
    let ont_file_required = ont_cache(&ont_cache_dir, ont_request.mime_type);

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
