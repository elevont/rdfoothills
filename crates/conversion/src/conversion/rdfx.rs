// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;

#[cfg(feature = "async")]
use async_trait::async_trait;

use super::OntFile;
use rdfoothills_mime as mime;

#[derive(Debug, Default)]
pub struct Converter;

const CLI_CMD: &str = "rdfx";
const CLI_CMD_DESC: &str = "RDF format conversion";

impl Converter {
    fn rdfx<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd(CLI_CMD, CLI_CMD_DESC, args)
    }

    #[cfg(feature = "async")]
    async fn rdfx_async<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd_async(CLI_CMD, CLI_CMD_DESC, args).await
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

macro_rules! convert_args {
    ($from:expr, $to:expr) => {
        &[
            OsStr::new("convert"),
            OsStr::new("--format"),
            OsStr::new(
                super::to_rdflib_format($to.mime_type)
                    .expect("rdfx called with an invalid (-> unsupported by RDFlib) target type"),
            ),
            OsStr::new("--output"),
            $to.file.as_os_str(),
            $from.file.as_os_str(),
        ]
    };
}

#[cfg_attr(feature = "async", async_trait)]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Data,
            priority: super::Priority::Low,
            typ: super::Type::Cli,
            name: "rdfx",
        }
    }

    fn is_available(&self) -> bool {
        super::is_cli_cmd_available(CLI_CMD)
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        Self::supports_format(from) && Self::supports_format(to)
    }

    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdfx(convert_args!(from, to))
    }

    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdfx_async(convert_args!(from, to)).await
    }
}
