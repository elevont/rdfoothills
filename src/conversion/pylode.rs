// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cache::OntFile;
use crate::mime;

pub struct Converter;

impl Converter {
    async fn pylode(args: &[&str]) -> Result<(), super::Error> {
        super::cli_cmd("pylode", "RDF to HTML conversion", args).await
    }
}

impl super::Converter for Converter {
    fn info() -> super::Info {
        super::Info {
            typ: super::Type::Cli,
            priority: 50,
            quality: super::Quality::Data,
        }
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        to == mime::Type::Html && super::to_rdflib_format(from).is_some()
    }

    async fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::pylode(&[
            "--sort",
            "--css",
            "true",
            "--profile",
            "ontpub",
            "--outputfile",
            super::to_str(&to.file),
            super::to_str(&from.file),
        ])
        .await
    }
}
