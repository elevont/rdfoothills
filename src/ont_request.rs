// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{mime, Config};
use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::{header::ACCEPT, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

/// If the requested format is not present yet,
/// which action to preffer to get it.
/// The other action will be tried if the first one fails.
#[derive(Debug, Copy, Clone)]
pub enum DlOrConv {
    Download,
    Convert,
}

#[derive(Debug)]
pub struct OntRequest {
    /// The original ontologies URI.
    pub uri: Url,
    /// The MIME type to be requested.
    /// It will be sent in the HTTP header `Accept`,
    /// when downloading from the supplied URI.
    pub query_mime_type: Option<mime::Type>,
    /// The MIME type requested.
    /// This is what our client wants,
    /// and what we try to sent to it.
    pub mime_type: mime::Type,
    pub pref: DlOrConv,
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
impl FromRequestParts<Config> for OntRequest {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Config,
    ) -> Result<Self, Self::Rejection> {
        let query_params: Query<HashMap<String, String>> =
            parts.extract().await.map_err(IntoResponse::into_response)?;
        let headers: HeaderMap = parts.extract().await.map_err(IntoResponse::into_response)?;
        // let config: State<Config> = state.into().await.map_err(IntoResponse::into_response)?;

        let mime_type = extract_requested_content_type(&headers)?;
        let uri = extract_uri(&query_params)?;
        let query_mime_type = extract_query_accept(&query_params)?;

        let pref = state.prefere_conversion; // TODO Maybe we want to allow setting this with a query parameter as well?
        Ok(Self {
            uri,
            query_mime_type,
            mime_type,
            pref,
        })
    }
}
