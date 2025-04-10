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

const CLI_CMD: &str = "rdf-convert";
const CLI_CMD_DESC: &str = "RDF format conversion (from/with pkg: 'rdftools')";

impl Converter {
    fn rdf_tools<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd(CLI_CMD, CLI_CMD_DESC, args)
    }

    #[cfg(feature = "async")]
    async fn rdf_tools_async<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd_async(CLI_CMD, CLI_CMD_DESC, args).await
    }
}

macro_rules! convert_args {
    ($from:expr, $to:expr) => {
        &[
            OsStr::new("--input"),
            $from.file.as_os_str(),
            OsStr::new("--output"),
            $to.file.as_os_str(),
            OsStr::new("--read"),
            OsStr::new(super::to_rdflib_format($from.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) source type",
            )),
            OsStr::new("--write"),
            OsStr::new(super::to_rdflib_format($to.mime_type).expect(
                "rdf-convert called with an invalid (-> unsupported by RDFlib) target type",
            )),
        ]
    };
}

#[cfg_attr(feature = "async", async_trait)]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Prefixes,
            priority: super::Priority::Mid,
            typ: super::Type::Cli,
            name: "rdf-convert",
        }
    }

    fn is_available(&self) -> bool {
        super::is_cli_cmd_available(CLI_CMD)
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        super::to_rdflib_format(from).is_some() && super::to_rdflib_format(to).is_some()
    }

    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdf_tools(convert_args!(from, to))
    }

    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::rdf_tools_async(convert_args!(from, to)).await
    }
}
