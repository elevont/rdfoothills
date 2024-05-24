// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::async_trait;

use crate::cache::OntFile;
use crate::mime;

#[derive(Debug, Default)]
pub struct Converter;

impl Converter {
    async fn rdf_tools(args: &[&str]) -> Result<(), super::Error> {
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
            "--input",
            super::to_str(&from.file),
            "--output",
            super::to_str(&to.file),
            "--read",
            super::to_rdflib_format(from.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) source type",
            ),
            "--write",
            super::to_rdflib_format(to.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) target type",
            ),
        ])
        .await
    }
}
