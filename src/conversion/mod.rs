// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod pylode;
mod rdfconvert;
mod rdfx;

use axum::async_trait;
use once_cell::sync::Lazy;

use crate::cache::OntFile;
use crate::mime;

use crate::cache::ont_file;
use crate::ont_request::OntRequest;
use std::io;
use std::path::Path;

static CONVERTERS: Lazy<Vec<Box<dyn Converter>>> = Lazy::new(|| {
    let mut converters: Vec<Box<dyn Converter>> = vec![
        Box::new(rdfx::Converter),
        Box::new(rdfconvert::Converter),
        Box::new(pylode::Converter),
    ];
    converters.sort();
    converters
});

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The source format ({from}) format is not machine-readable, and therefore auto-conversion from it to any other format is impossible. ")]
    NonMachineReadableSource { from: mime::Type },

    #[error("None of the supported converters can convert from {from} to {to}. ")]
    NoConverter { from: mime::Type, to: mime::Type },

    #[error("Failed to run {cmd} for {task}: {from}")]
    ExtCmdFaileToInvoke {
        from: io::Error,
        cmd: String,
        task: String,
    },

    #[error("Running {cmd} for {task} returned with non-zero exit status '{exit_code}', indicating an error. stderr:\n{stderr}")]
    ExtCmdUnsuccessfull {
        cmd: String,
        task: String,
        exit_code: i32,
        stderr: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    PreservesComments,
    PreservesFormatting,
    PreservesOrder,
    Base,
    Prefixes,
    Data,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    High,
    Mid,
    Low,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Native,
    Cli,
    NetworkService,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Info {
    quality: Quality,
    priority: Priority,
    typ: Type,
    name: &'static str,
}

#[async_trait]
pub trait Converter: Send + Sync {
    fn info(&self) -> Info;
    fn supports(&self, from: mime::Type, to: mime::Type) -> bool;
    async fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), Error>;
}

impl PartialEq for dyn Converter {
    fn eq(&self, other: &Self) -> bool {
        self.info().eq(&other.info())
    }
}

impl Eq for dyn Converter {}

impl PartialOrd for dyn Converter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for dyn Converter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.info().cmp(&other.info())
    }
}

pub const fn to_rdflib_format(mime_type: mime::Type) -> Option<&'static str> {
    match mime_type {
        mime::Type::BinaryRdf
        | mime::Type::Csvw
        | mime::Type::Hdt
        | mime::Type::Html
        | mime::Type::Microdata
        | mime::Type::NdJsonLd
        | mime::Type::NQuadsStar
        | mime::Type::NTriplesStar
        | mime::Type::RdfA
        | mime::Type::RdfJson
        | mime::Type::TriGStar
        | mime::Type::OwlFunctional
        | mime::Type::OwlXml
        | mime::Type::Tsvw
        | mime::Type::TurtleStar
        | mime::Type::YamlLd => None,
        mime::Type::HexTuples => Some("hext"),
        // mime::Type::Html => Some("rdfa"),
        mime::Type::JsonLd => Some("json-ld"),
        mime::Type::N3 => Some("n3"),
        mime::Type::NQuads => Some("nquads"),
        mime::Type::NTriples => Some("nt"),
        // mime::Type::RdfA => Some("rdfa"),
        mime::Type::TriG => Some("trig"),
        mime::Type::RdfXml => Some("xml"),
        // mime::Type::RdfXml => Some("pretty-xml"),
        mime::Type::TriX => Some("trix"),
        mime::Type::Turtle => Some("turtle"),
    }
}

pub fn to_str(path: &Path) -> &str {
    path.as_os_str().to_str().unwrap()
}

pub async fn cli_cmd(cmd: &str, task: &str, args: &[&str]) -> Result<(), Error> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|from| Error::ExtCmdFaileToInvoke {
            from,
            cmd: cmd.to_owned(),
            task: task.to_owned(),
        })?;
    if !output.status.success() {
        return Err(Error::ExtCmdUnsuccessfull {
            cmd: cmd.to_owned(),
            task: task.to_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

pub async fn convert(
    ont_request: &OntRequest,
    ont_cache_dir: &Path,
    from: &OntFile,
) -> Result<OntFile, Error> {
    if !from.mime_type.is_machine_readable() {
        return Err(Error::NonMachineReadableSource {
            from: from.mime_type,
        });
    }

    let ont_requested_file = ont_file(ont_cache_dir, ont_request.mime_type);
    let to = OntFile {
        file: ont_requested_file,
        mime_type: ont_request.mime_type,
    };

    if from.mime_type == to.mime_type {
        return Ok(to);
    }

    for converter in CONVERTERS.iter() {
        if converter.supports(from.mime_type, to.mime_type) {
            return converter.convert(from, &to).await.map(|()| to);
        }
    }

    Err(Error::NoConverter {
        from: from.mime_type,
        to: to.mime_type,
    })
}
