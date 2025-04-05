// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "oxrdfio")]
mod oxrdfio;
mod pylode;
mod rdfconvert;
mod rdfx;

#[cfg(feature = "async")]
use async_trait::async_trait;
use once_cell::sync::Lazy;
#[cfg(not(feature = "async"))]
use std::process;
#[cfg(feature = "async")]
use tokio::process;

use rdfoothills_mime as mime;

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
    #[cfg(feature = "oxrdfio")]
    converters.push(Box::new(oxrdfio::Converter));
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
    ExtCmdUnsuccessful {
        cmd: String,
        task: String,
        exit_code: i32,
        stderr: String,
    },

    #[error(
        "Input and output formats are the same. Try to just copy the file, if really required"
    )]
    NoConversionRequired,

    #[error("The input file was not syntactically valid:\n{0}")]
    Syntax(String),

    /// Represents all cases of `std::io::Error`.
    #[error(transparent)]
    Io(#[from] std::io::Error),
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
    pub quality: Quality,
    pub priority: Priority,
    pub typ: Type,
    pub name: &'static str,
}

#[cfg_attr(feature = "async", async_trait)]
pub trait Converter: Send + Sync {
    fn info(&self) -> Info;
    fn is_available(&self) -> bool;
    fn supports(&self, from: mime::Type, to: mime::Type) -> bool;

    /// Converts from one RDF format to another - non-async version.
    ///
    /// # Errors
    ///
    /// - if the conversion is not supported (see `Converter::supports`)
    /// - if the conversion fails
    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), Error>;

    /// Converts from one RDF format to another - async version.
    ///
    /// # Errors
    ///
    /// - if the conversion is not supported (see `Converter::supports`)
    /// - if the conversion fails
    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), Error>;
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
        | mime::Type::Manchester
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

fn handle_cli_cmd_output(
    cmd: &str,
    task: &str,
    output_res: io::Result<std::process::Output>,
) -> Result<(), Error> {
    let output = output_res.map_err(|from| Error::ExtCmdFailedToInvoke {
        from,
        cmd: cmd.to_owned(),
        task: task.to_owned(),
    })?;
    if !output.status.success() {
        return Err(Error::ExtCmdUnsuccessful {
            cmd: cmd.to_owned(),
            task: task.to_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

macro_rules! trace_cmd {
    ($specifier:expr, $cmd:expr, $args:expr) => {
        tracing::trace!(
            "Running CLI command{}:\n{} {}",
            $specifier,
            $cmd,
            $args
                .clone()
                .into_iter()
                .map(|arg| arg.as_ref().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
    };
}

/// Executes an external command, more or less as if on the CLI.
///
/// * `cmd` - The command to execute
/// * `task` - The human oriented description of the task/goal
///   of this command execution
/// * `args` - The arguments to pass to the command, as if on the CLI
///
/// # Errors
///
/// Returns `Error::ExtCmdFailedToInvoke` if the command was not found,
/// or we do not have the permission to execute it.
/// Returns `Error::ExtCmdUnsuccessful` if the command was executed,
/// but something went wrong/failed (exit state != 0).
pub fn cli_cmd<I, S>(cmd: &str, task: &str, args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = S> + Send + Clone,
    S: AsRef<OsStr>,
{
    trace_cmd!("", cmd, args);
    handle_cli_cmd_output(
        cmd,
        task,
        std::process::Command::new(cmd).args(args).output(),
    )
}

/// Executes an external command, more or less as if on the CLI.
///
/// * `cmd` - The command to execute
/// * `task` - The human oriented description of the task/goal
///   of this command execution
/// * `args` - The arguments to pass to the command, as if on the CLI
///
/// # Errors
///
/// Returns `Error::ExtCmdFailedToInvoke` if the command was not found,
/// or we do not have the permission to execute it.
/// Returns `Error::ExtCmdUnsuccessful` if the command was executed,
/// but something went wrong/failed (exit state != 0).
#[cfg(feature = "async")]
pub async fn cli_cmd_async<I, S>(cmd: &str, task: &str, args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = S> + Send + Clone,
    S: AsRef<OsStr>,
{
    trace_cmd!(" (async)", cmd, args);
    handle_cli_cmd_output(
        cmd,
        task,
        process::Command::new(cmd).args(args).output().await,
    )
}

/// Converts from one RDF format to another.
///
/// # Errors
///
/// Returns `Error::NonMachineReadableSource` if conversion would be necessary,
/// but the source is not machine readable.
/// Returns `Error::NoConverter` if the conversion is not supported.
pub fn select_converter(from: &OntFile, to: &OntFile) -> Result<&'static dyn Converter, Error> {
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
            return Ok(converter.as_ref());
        }
    }

    Err(Error::NoConverter {
        from: from.mime_type,
        to: to.mime_type,
    })
}

/// Converts from one RDF format to another.
///
/// # Errors
///
/// Returns `Error::NonMachineReadableSource` if conversion would be necessary,
/// but the source is not machine readable.
/// Returns `Error::NoConverter` if the conversion is not supported.
/// Returns `Error::*` if conversion failed.
pub fn convert(from: &OntFile, to: &OntFile) -> Result<Info, Error> {
    let converter = select_converter(from, to)?;
    converter.convert(from, to).map(|()| converter.info())
}

/// Converts from one RDF format to another.
///
/// # Errors
///
/// Returns `Error::NonMachineReadableSource` if conversion would be necessary,
/// but the source is not machine readable.
/// Returns `Error::NoConverter` if the conversion is not supported.
/// Returns `Error::*` if conversion failed.
#[cfg(feature = "async")]
pub async fn convert_async(from: &OntFile, to: &OntFile) -> Result<Info, Error> {
    let converter = select_converter(from, to)?;
    converter
        .convert_async(from, to)
        .await
        .map(|()| converter.info())
}
