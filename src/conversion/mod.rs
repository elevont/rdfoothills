// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod pylode;
mod rdfconvert;
mod rdfx;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use tokio::process;

use crate::mime;

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

pub struct OntFile {
    pub file: PathBuf,
    pub mime_type: mime::Type,
}

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

    #[error("None of the supported and available converters can convert from {from} to {to}. ")]
    NoConverter { from: mime::Type, to: mime::Type },

    #[error("Failed to run {cmd} for {task}: {from}")]
    ExtCmdFailedToInvoke {
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

    #[error(
        "Input and output formats are the same. Try to just copy the file, if really required"
    )]
    NoConversionRequired,
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
    fn is_available(&self) -> bool;
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

#[must_use]
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

/// Checks if an external command is available
/// and we have the rights to execute it.
#[must_use]
pub fn is_cli_cmd_available(cmd: &str) -> bool {
    process::Command::new(cmd).spawn().is_ok()
}

/// Executes an external command, more or less as if on the CLI.
/// @param cmd The command to execute
/// @param task The human oriented description of the task/goal of this command execution
/// @param args The arguments to pass to the command, as if on the CLI
///
/// # Errors
///
/// Returns `Error::ExtCmdFailedToInvoke` if the command was not found,
/// or we do not have the permission to execute it.
/// Returns `Error::ExtCmdUnsuccessfull` if the command was executed,
/// but somethign went wrong/failed (exit state != 0).
// pub async fn cli_cmd(cmd: &str, task: &str, args: &[&str]) -> Result<(), Error> {
pub async fn cli_cmd<I, S>(cmd: &str, task: &str, args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = S> + Send,
    S: AsRef<OsStr>,
{
    let output = process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|from| Error::ExtCmdFailedToInvoke {
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

/// Converts from one RDF format to another.
///
/// # Errors
///
/// Returns `Error::NonMachineReadableSource` if conversion would be necessary,
/// but the source is not machine readable.
/// Returns `Error::NoConverter` if the conversion is not supported.
pub async fn convert(from: &OntFile, to: &OntFile) -> Result<(), Error> {
    if !from.mime_type.is_machine_readable() {
        return Err(Error::NonMachineReadableSource {
            from: from.mime_type,
        });
    }

    if from.mime_type == to.mime_type {
        return Err(Error::NoConversionRequired);
    }

    for converter in CONVERTERS.iter() {
        if converter.supports(from.mime_type, to.mime_type) && converter.is_available() {
            return converter.convert(from, to).await;
        }
    }

    Err(Error::NoConverter {
        from: from.mime_type,
        to: to.mime_type,
    })
}
