// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;

use async_trait::async_trait;

use super::OntFile;
use crate::mime;

#[derive(Debug, Default)]
pub struct Converter;

impl Converter {
    async fn rdf_tools<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send,
        S: AsRef<OsStr>,
    {
        super::cli_cmd(
            "rdf-convert",
            "RDF format conversion (from/with pkg: 'rdftools')",
            args,
        )
        .await
    }
}

#[async_trait]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Prefixes,
            priority: super::Priority::Mid,
            typ: super::Type::Cli,
            name: "rdf-convert",
        }
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        super::to_rdflib_format(from).is_some() && super::to_rdflib_format(to).is_some()
    }

    async fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdf_tools(&[
            OsStr::new("--input"),
            from.file.as_os_str(),
            OsStr::new("--output"),
            to.file.as_os_str(),
            OsStr::new("--read"),
            OsStr::new(super::to_rdflib_format(from.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) source type",
            )),
            OsStr::new("--write"),
            OsStr::new(super::to_rdflib_format(to.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) target type",
            )),
        ])
        .await
    }
}
