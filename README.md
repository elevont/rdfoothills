<!--
SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# rdfoothillls = RDF Utils

[![License: AGPL-3.0-or-later](
    https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](
    LICENSE.txt)
[![REUSE status](
    https://api.reuse.software/badge/codeberg.org/elevont/rdfoothills)](
    https://api.reuse.software/info/codeberg.org/elevont/rdfoothills)
[![Repo](
    https://img.shields.io/badge/CodeBerg.org-green?style=flat&label=Repo)](
    https://codeberg.org/elevont/rdfoothills)
[![Package Releases](
    https://img.shields.io/crates/v/rdfoothills.svg)](
    https://crates.io/crates/rdfoothills)
[![Documentation Releases](
    https://docs.rs/rdfoothills/badge.svg)](
    https://docs.rs/rdfoothills)
[![Dependency Status](
    https://deps.rs/repo/github/elevont/rdfoothills/status.svg)](
    https://deps.rs/repo/github/elevont/rdfoothills)
[![Build Status](
    https://github.com/elevont/rdfoothills/workflows/build/badge.svg)](
    https://github.com/elevont/rdfoothills/actions)

A collection of mostly small, [RDF] related utilities,
including conversatino between different serialization formats,
using external tools.

Currently includes:

- conversion with Python CLI tools from within Rust,
  including conversion between different [RDF serialization formats]
  and to HTML
- meta data about a list of known RDF serialization formats, including:
  - MIME type
  - file extension

Projects using this library:

- [`ontprox`]
- [`onts-depot`]

Behind the scenes we use [pyLODE] for conversion to HTML,
and the python [RDFlib] (through a thin CLI wrapper:
`rdf-convert` from the [rdftools] tool-set)
for all other conversion.

[`ontprox`]: https://codeberg.org/elevont/ontprox
[`onts-depot`]: https://codeberg.org/elevont/onts-depot
[pyLODE]: https://github.com/RDFLib/pyLODE
[RDF serialization formats]: https://ontola.io/blog/rdf-serialization-formats
[RDF]: https://en.wikipedia.org/wiki/Resource_Description_Framework
[RDFlib]: https://rdflib.readthedocs.io/en/stable/
[rdftools]: https://github.com/elevont/rdftools
