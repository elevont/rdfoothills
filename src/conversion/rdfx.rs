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
    async fn rdfx<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send,
        S: AsRef<OsStr>,
    {
        super::cli_cmd("rdfx", "RDF format conversion", args).await
    }

    const fn supports_format(fmt: mime::Type) -> bool {
        match fmt {
            mime::Type::N3
            | mime::Type::JsonLd
            | mime::Type::NTriples
            | mime::Type::OwlXml
            | mime::Type::RdfXml
            | mime::Type::Turtle => true,
            mime::Type::BinaryRdf
            | mime::Type::Csvw
            | mime::Type::Hdt
            | mime::Type::HexTuples
            | mime::Type::Html
            | mime::Type::Microdata
            | mime::Type::NdJsonLd
            | mime::Type::NQuads
            | mime::Type::NQuadsStar
            | mime::Type::NTriplesStar
            | mime::Type::OwlFunctional
            | mime::Type::RdfA
            | mime::Type::RdfJson
            | mime::Type::TriG
            | mime::Type::TriGStar
            | mime::Type::TriX
            | mime::Type::Tsvw
            | mime::Type::TurtleStar
            | mime::Type::YamlLd => false,
        }
    }
}

#[async_trait]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Data,
            priority: super::Priority::Low,
            typ: super::Type::Cli,
            name: "rdfx",
        }
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        Self::supports_format(from) && Self::supports_format(to)
    }

    async fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdfx(&[
            OsStr::new("convert"),
            OsStr::new("--format"),
            OsStr::new(
                super::to_rdflib_format(to.mime_type)
                    .expect("rdfx called with an invalid (-> unsupported by RDFlib) target type"),
            ),
            OsStr::new("--output"),
            to.file.as_os_str(),
            from.file.as_os_str(),
        ])
        .await
    }
}
