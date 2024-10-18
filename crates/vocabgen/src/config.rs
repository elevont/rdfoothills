// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct Config {
    /**
     * Paths to locally stored ontology files in the RDF/Turtle format,
     * to be converted to Rust source files representing them.
     */
    pub ontologies: Vec<PathBuf>,
    /**
     * Where to write the output Rust source files to.
     */
    pub out_dir: PathBuf,
    /**
     * The text to insert on top of all output files
     * (generated Rust source code).
     */
    pub header: Option<String>,
    /**
     * Whether to overwrite potentially already existing output files.
     */
    pub force: bool,
}
