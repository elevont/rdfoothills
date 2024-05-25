// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::OsStr;

use async_trait::async_trait;
use once_cell::sync::Lazy;

use super::OntFile;
use crate::mime;

#[derive(Debug, Default)]
pub struct Converter;

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
    async fn pylode<I, S>(args: I) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = S> + Send,
        S: AsRef<OsStr>,
    {
        super::cli_cmd("pylode", "RDF to HTML conversion", args).await
    }
}

#[async_trait]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Data,
            priority: super::Priority::Mid,
            typ: super::Type::Cli,
            name: "pyLODE",
        }
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        to == mime::Type::Html && super::to_rdflib_format(from).is_some()
    }

    async fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        Self::pylode(
            PYLODE_ARGS_BEGIN
                .iter()
                .chain(&[to.file.as_os_str(), from.file.as_os_str()]),
        )
        .await
    }
}
