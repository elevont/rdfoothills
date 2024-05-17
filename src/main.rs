// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
use regex::Regex;
use reqwest::header::CONTENT_DISPOSITION;
use std::{collections::HashMap, path::Path as StdPath, path::PathBuf};
use std::{ffi::OsStr, net::SocketAddr};
use std::{io, str::FromStr};
use tokio_util::io::ReaderStream;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

mod hasher;
mod mime;

pub static CACHE_ROOT: Lazy<PathBuf> = Lazy::new(|| dirs::cache_dir().unwrap().join("ont-serv"));
pub static ONTS_CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| CACHE_ROOT.join("ontologies"));
pub const ONT_FILE_PREFIX: &str = "ontology";

const UPLOADS_DIRECTORY: &str = "uploads";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ont_serv=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    create_dir(UPLOADS_DIRECTORY).await;
    create_dir(ONTS_CACHE_DIR.as_path()).await;

    // build our application with some routes
    let app = Router::new().route("/rdf-ont", get(handler_rdf));

    // run it
    tokio::join!(serve(app, 3000),);
}

async fn create_dir<P: AsRef<StdPath> + Send>(dir: P) {
    create_dir_res(dir.as_ref())
        .await
        .map_err(|err| {
            panic!(
                "failed to create directory `{}`: {err}",
                dir.as_ref().display()
            )
        })
        .unwrap();
}

async fn create_dir_res<P: AsRef<StdPath> + Send>(dir: P) -> io::Result<()> {
    tokio::fs::create_dir_all(dir).await.or_else(|err| {
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
        }
    })
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
fn path_is_valid(path: &str) -> bool {
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return false;
        }
    }

    components.count() == 1
}

// -----------------------------

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

#[async_trait]
impl<S> FromRequestParts<S> for OntRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // let path_params: Path<HashMap<String, String>> =
        //     parts.extract().await.map_err(IntoResponse::into_response)?;
        let query_params: Query<HashMap<String, String>> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        let headers: HeaderMap = parts.extract().await.map_err(IntoResponse::into_response)?;

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
        let content_type_str = headers
            .get(ACCEPT)
            .map(|ctype| {
                HeaderValue::to_str(ctype).map_err(|err| {
                    (
                        StatusCode::NOT_FOUND,
                        format!(
                            "Failed to convert header value for 'content-type' to string: {err}"
                        ),
                    )
                        .into_response()
                })
            })
            .transpose()?;
        dbg!(&content_type_str);
        // .unwrap_or_else(|err| (StatusCode::NOT_FOUND, format!("Failed to convert header value for 'content-type' to string: {err}")).into_response())?;
        let mime_type = content_type_str
            .map(|ctype_str| mime::Type::from_str(ctype_str).map_err(|err|
                (StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    format!("Failed to parse requested content-type '{ctype_str}' to an RDF MIME type: {err}")
                ).into_response()))
            .transpose()?
            .unwrap_or_default();
        dbg!(&mime_type);

        let query_mime_type = query_params
            .get("query-accept")
            .map(|ctype_str| mime::Type::from_str(ctype_str).map_err(|err|
                (StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    format!("Failed to parse content-type to be requested '{ctype_str}' to an RDF MIME type: {err}")
                ).into_response()))
            .transpose()?;

        let pref = DlOrConv::Download; // TODO Maybe we want to allow setting this with a query parameter as well?
        Ok(Self {
            uri,
            query_mime_type,
            mime_type,
            pref,
        })
    }
}

pub static NON_BASIC_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]").unwrap());

fn url2fname(url: &Url) -> String {
    let url_str = url.as_str();
    let url_nameified = NON_BASIC_CHARS.replace_all(url_str, "_");
    url_nameified.into()
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

async fn look_for_file(file_path: &StdPath) -> io::Result<bool> {
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

async fn cli_cmd(cmd: &str, task: &str, args: &[&str]) -> Result<(), (StatusCode, String)> {
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

async fn pylode(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd("pylode", "RDF to HTML conversion", args).await
}

async fn rdf_tools(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd(
        "rdf-convert",
        "RDF format conversion (from/with pkg: 'rdftools')",
        args,
    )
    .await
}

async fn rdfx(args: &[&str]) -> Result<(), (StatusCode, String)> {
    cli_cmd("rdfx", "RDF format conversion", args).await
}

async fn to_html_conversion(
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

async fn rdf_convert(
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

type ResponseBody = axum::http::Response<axum::body::Body>;

fn respond_with_body(
    file: &StdPath,
    mime_type: mime::Type,
    body: Body,
    // ) -> impl IntoResponse + Sized {
) -> (HeaderMap, Body) {
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

// async fn body_from_file(file: &StdPath) -> Result<Body, (StatusCode, String)> {
async fn body_from_file(file: &StdPath) -> Result<Body, (StatusCode, String)> {
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

fn body_from_content(ont_content: Vec<u8>) -> Body {
    Body::from(ont_content)
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

/// Ensures the provided dir exists.
/// Returns whether it was created.
async fn ensure_dir_exists(dir_path: &StdPath) -> io::Result<bool> {
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

// async fn dl_ont(ont_request: OntRequest, ont_cache_dir: &StdPath) -> Result<OntCacheFile, impl IntoResponse> {
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
    // {
    //     let ont_cache_dir_path_exists = tokio::fs::try_exists(&ont_cache_dir)
    //         .await
    //         .expect("Failed to check if ontology cache directory path exists");
    //     if ont_cache_dir_path_exists {
    //         if !tokio::fs::metadata(&ont_cache_dir)
    //             .await
    //             .expect("Failed to check if ontology cache path is a directory")
    //             .is_dir()
    //         {
    //             panic!("Should be an ontology cache directory but is not a directory: '{}' - possible solution: delete it", ont_cache_dir.display());
    //         }
    //         ont_might_be_cached = true;
    //     } else {
    //         tokio::fs::create_dir_all(&ont_cache_dir)
    //             .await
    //             .expect("Failed to create ontology cache directory");
    //     }
    // }

    if ont_might_be_cached {
        let ont_file_required_exists = look_for_file(&ont_file_required).await.unwrap();
        // let ont_file_required_exists = tokio::fs::try_exists(&ont_file_required)
        //     .await
        //     .expect("Failed to check if ontology cache file path exists");
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
            // TODO FIXME cache if no ontology found, otherwise always use cache
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

                    // let ont_cache_file_content = tokio::fs::read(&mr_ont_cache_file.file)
                    //     .await
                    //     .map_err(|err| {
                    //         (
                    //             StatusCode::INTERNAL_SERVER_ERROR,
                    //             format!(
                    //                 "Failed to read ontology cache file '{}': {err}",
                    //                 mr_ont_cache_file.file.display()
                    //             ),
                    //         )
                    //     })
                    //     .unwrap();
                    // OntCacheFile {
                    //     file: ont_cache_file_found,
                    //     mime_type: mtype,
                    //     content: ont_cache_file_content,
                    // }
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
            // .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!(
            //     "We were unable/failed to conver the format returned by the server ({}) into the requested format ({}) - {err}.",
            //     &dled_ont_file.mime_type, ont_request.mime_type
            // )).into_response());
            conversion_res.map_err(IntoResponse::into_response)
            // if !cached_ont.mime_type.is_machine_readable() {
            //     Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
            //         "As the cached format of this ontology ({}) is not machine-readable, it cannot be converted into the requested format.",
            //         cached_ont.mime_type
            //     )).into_response())
            // } else {
            //     match (
            //         to_rdflib_format(cached_ont.mime_type),
            //         to_rdflib_format(ont_request.mime_type),
            //     ) {
            //         (Some(cached_rdflib_type), Some(requested_rdflib_type)) => {
            //             let ont_requested_file = ont_cache(&ont_cache_dir, ont_request.mime_type);
            //             rdf_convert(
            //                 cached_rdflib_type,
            //                 requested_rdflib_type,
            //                 &cached_ont.file,
            //                 &ont_requested_file,
            //             )
            //             .await
            //             .map_err(IntoResponse::into_response)?;
            //             Ok(respond_with_body(
            //                 &ont_requested_file,
            //                 ont_request.mime_type,
            //                 body_from_file(&ont_requested_file)
            //                     .await
            //                     .map_err(IntoResponse::into_response)?,
            //             )
            //             .await)
            //         }
            //         _ => Err((
            //             StatusCode::INTERNAL_SERVER_ERROR,
            //             format!(
            //                 "Can not convert {} to {}",
            //                 cached_ont.mime_type, ont_request.mime_type
            //             ),
            //         )
            //             .into_response()),
            //     }
            // }
        } else {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!(
                "As the format returned by the server ({}) is not machine-readable, it cannot be converted into the requested format.",
                dled_ont.mime_type
            )).into_response())
        }
    }
}

const fn to_rdflib_format(mime_type: mime::Type) -> Option<&'static str> {
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

fn extract_file_ext(file: &StdPath) -> Option<&str> {
    file.extension().and_then(OsStr::to_str)
}

// -----------------------------

async fn serve(app: Router, port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
