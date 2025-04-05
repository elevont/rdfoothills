// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(dead_code)]

use cli_utils as _;

pub mod cli;
pub mod config;
pub mod parse;

use std::fs;
use std::io;

use config::Config;
use git_version::git_version;
use oxrdfio::RdfFormat;

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentation.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub const VERSION: &str = git_version!(cargo_prefix = "", fallback = "unknown");

#[allow(clippy::doc_markdown)]
/// Generates one of more Rust `vocab` files (for OxRDF)
/// from one or more RDF/Turtle files.
///
/// # Errors
///
/// - one of the input files cannot be read
/// - one of the output files cannot be written
/// - one of the input vocabularies does not have a preferred namespace prefix defined internally
/// - one of the input vocabularies does not have a preferred namespace uri defined internally
pub fn generate(config: &Config) -> io::Result<()> {
    let mut written_out_files = Vec::new();
    for ont in &config.ontologies {
        let turtle_content_str = fs::read_to_string(ont)?;
        let turtle_content = turtle_content_str.as_bytes();

        let rdf_cont = parse::rdf(turtle_content, RdfFormat::Turtle); // TODO Allow to parse other formats then Turtle

        let vocab_info = rdf_cont.into_vocab_info().map_err(io::Error::other)?;
        let ont_namespace = vocab_info
            .preferred_namespace_prefix
            .clone()
            .or_else(|| {
                ont.file_stem()
                    .map(|stem_os_str| stem_os_str.to_string_lossy().to_string())
            })
            .ok_or_else(|| io::Error::other(format!(
                "For input file '{ont}', we were unable to find a preferred namespace prefix; we checked within the ontology data, and considered the input file-name.",
                ont = ont.display())))?;
        let rust_vocab_src = vocab_info.to_str().map_err(io::Error::other)?;
        let out_file = config.out_dir.join(format!("{ont_namespace}.rs"));
        if config.force || !out_file.exists() {
            if written_out_files.contains(&out_file) {
                return Err(io::Error::other(format!(
                    "Two (or more) input ontologies result in the same output file name: {}; please change that.", out_file.display())));
            }
            fs::write(&out_file, rust_vocab_src)?;
            written_out_files.push(out_file);
        }
    }

    Ok(())
}
