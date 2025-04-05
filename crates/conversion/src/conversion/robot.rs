// SPDX-FileCopyrightText: 2025 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This is a general RDF tool from the [OBO](http://obolibrary.org) people,
//! which also develop [LinkML](https://linkml.io/).
//! It is a Java tool.
//! We use it specifically for converting to
//! Functional and Manchester syntaxes.
//!
//! General documentation:
//!
//! - <https://github.com/ontodev/robot>
//! - <http://robot.obolibrary.org/convert>
//!
//! Install instructions:
//!
//! - <http://robot.obolibrary.org/#1-getting-started>

use std::ffi::OsStr;

#[cfg(feature = "async")]
use async_trait::async_trait;

use super::OntFile;
use rdfoothills_mime as mime;

#[derive(Debug, Default)]
pub struct Converter;

const CLI_CMD: &str = "robot";
const CLI_CMD_DESC: &str = "RDF format conversion";

impl Converter {
    fn robot<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd(CLI_CMD, CLI_CMD_DESC, args)
    }

    #[cfg(feature = "async")]
    async fn robot_async<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd_async(CLI_CMD, CLI_CMD_DESC, args).await
    }

    #[must_use]
    pub const fn to_robot_format(mime_type: mime::Type) -> Option<&'static str> {
        match mime_type {
            mime::Type::Manchester => Some("omn"),
            mime::Type::OwlFunctional => Some("ofn"),
            mime::Type::OwlXml => Some("owx"),
            mime::Type::RdfXml => Some("owl"),
            mime::Type::Turtle => Some("ttl"),
            mime::Type::BinaryRdf
            | mime::Type::Csvw
            | mime::Type::Hdt
            | mime::Type::HexTuples
            | mime::Type::Html
            | mime::Type::JsonLd
            | mime::Type::Microdata
            | mime::Type::N3
            | mime::Type::NTriples
            | mime::Type::NdJsonLd
            | mime::Type::NQuads
            | mime::Type::NQuadsStar
            | mime::Type::NTriplesStar
            | mime::Type::RdfA
            | mime::Type::RdfJson
            | mime::Type::TriG
            | mime::Type::TriGStar
            | mime::Type::TriX
            | mime::Type::Tsvw
            | mime::Type::TurtleStar
            | mime::Type::YamlLd => None,
        }
    }

    const fn supports_format(fmt: mime::Type) -> bool {
        Self::to_robot_format(fmt).is_some()
    }
}

macro_rules! convert_args {
    ($from:expr, $to:expr) => {
        &[
            OsStr::new("convert"),
            OsStr::new("--input"),
            $from.file.as_os_str(),
            OsStr::new("--format"),
            OsStr::new(
                Self::to_robot_format($to.mime_type)
                    .expect("robot called with an unsupported target format type"),
            ),
            OsStr::new("--output"),
            $to.file.as_os_str(),
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
            name: "robot",
        }
    }

    fn is_available(&self) -> bool {
        super::is_cli_cmd_available(CLI_CMD)
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        Self::supports_format(from) && Self::supports_format(to)
    }

    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::robot(convert_args!(from, to))
    }

    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::robot_async(convert_args!(from, to)).await
    }
}
