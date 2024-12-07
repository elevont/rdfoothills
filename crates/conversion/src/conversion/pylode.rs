// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;

#[cfg(feature = "async")]
use async_trait::async_trait;
use once_cell::sync::Lazy;

use super::OntFile;
use rdfoothills_mime as mime;

#[derive(Debug, Default)]
pub struct Converter;

const CLI_CMD: &str = "pylode";
const CLI_CMD_DESC: &str = "RDF to HTML conversion";

static PYLODE_ARGS_BEGIN: Lazy<Vec<&'static OsStr>> = Lazy::new(|| {
    vec![
        OsStr::new("--sort"),
        OsStr::new("--css"),
        OsStr::new("true"),
        OsStr::new("--profile"),
        OsStr::new("ontpub"),
        OsStr::new("--outputfile"),
    ]
});

impl Converter {
    fn pylode<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd(CLI_CMD, CLI_CMD_DESC, args)
    }

    #[cfg(feature = "async")]
    async fn pylode_async<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send + Clone,
        S: AsRef<OsStr>,
    {
        super::cli_cmd_async(CLI_CMD, CLI_CMD_DESC, args).await
    }
}

macro_rules! convert_args {
    ($from:expr, $to:expr) => {
        PYLODE_ARGS_BEGIN
            .iter()
            .chain(&[$to.file.as_os_str(), $from.file.as_os_str()])
    };
}

#[cfg_attr(feature = "async", async_trait)]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Data,
            priority: super::Priority::Mid,
            typ: super::Type::Cli,
            name: "pyLODE",
        }
    }

    fn is_available(&self) -> bool {
        super::is_cli_cmd_available(CLI_CMD)
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        to == mime::Type::Html && super::to_rdflib_format(from).is_some()
    }

    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::pylode(convert_args!(from, to))
    }

    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::pylode_async(convert_args!(from, to)).await
    }
}
