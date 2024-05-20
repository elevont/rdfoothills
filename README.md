<!--
SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# `ontprox` - **Ont**ology **Prox**y

[![License: AGPL-3.0-or-later](
    https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](
    LICENSE.txt)
[![REUSE status](
    https://api.reuse.software/badge/github.com/hoijui/ontprox)](
    https://api.reuse.software/info/github.com/hoijui/ontprox)
[![Repo](
    https://img.shields.io/badge/Repo-GitHub-555555&logo=github.svg)](
    https://github.com/hoijui/ontprox)
[![Package Releases](
    https://img.shields.io/crates/v/ontprox.svg)](
    https://crates.io/crates/ontprox)
[![Documentation Releases](
    https://docs.rs/ontprox/badge.svg)](
    https://docs.rs/ontprox)
[![Dependency Status](
    https://deps.rs/repo/github/hoijui/ontprox/status.svg)](
    https://deps.rs/repo/github/hoijui/ontprox)
[![Build Status](
    https://github.com/hoijui/ontprox/workflows/build/badge.svg)](
    https://github.com/hoijui/ontprox/actions)

A tiny HTTP service that allows to fetch an RDF ontology
that is available in one/a few format(s),
in others.

For example, an ontology is served under IRI/URI
<https://w3id.org/someorg/ont/thisone>
as Turtle.
This service can then be used as a proxy,
given this URI plus a target MIME-type
(through the HTTP `Accept` header) -
for example `application/ld+json` or `text/html`;
this MIME-type will then be served,
if the conversion is possible.

Internally we use [pyLODE] for conversion to HTML,
and the python [RDFlib] (through a thin CLI wrapper)
for all other conversion.

**NOTE**
Caching is involved!

## Usage

### Prerequisites

You need to have [`pylode`][pyLODE]
and [`rdf-convert` (from the _rdftools_ tool set][rdftools]
installed and available on your [`PATH`][PATH].

### Install

As for now, you have two choices:

1. [Compile it](#how-to-compile) yourself
1. Download a Linux x86\_64 statically linked binary from
   [the latest release](https://github.com/hoijui/ontprox/releases/latest)

### Run

(see [Install](#install) and [Prerequisites](#prerequisites) first)

```shell
$ cargo run
...
listening on 127.0.0.1:3000
```

### Fetches

(see [Run](#run) first)

When the service is running on `http://127.0.0.1:3000`,
you can fetch the HTML version
of e.g. the [ValueFlows] (VF) ontology as HTML,
either by opening <http://127.0.0.1:3000?uri=https://w3id.org/valueflows/ont/vf.TTL>
in your browser, or on the command line with [CURL]:

```shell
curl "http://127.0.0.1:3000?uri=https://w3id.org/valueflows/ont/vf.TTL" \
    -H "Accept: text/html" \
    > ont_vf.html
```

To give an other example,
this fetches the [Open Know-How] (OKH) ontology as JSON-LD:

```shell
curl "http://127.0.0.1:3000?uri=https://w3id.org/oseg/ont/okh" \
    -H "Accept: application/ld+json" \
    > ont_okh.jsonld
```

## How to compile

You need to install Rust(lang) and Cargo.
On most platforms, the best way to do this is with [RustUp].

Then get the sources with:

```bash
git clone --recurse-submodules https://github.com/hoijui/ontprox.git
cd ontprox
```

Then you can compile with:

```bash
cargo build
```

If all goes well,
the executable can be found at `target/debug/ontprox`.

[RDFlib]: https://rdflib.readthedocs.io
[pyLODE]: https://github.com/RDFLib/pyLODE
[RustUp]: https://rustup.rs/
[ValueFlows]: https://valueflo.ws/
[Open Know-How]: https://github.com/iop-alliance/OpenKnowHow
[CURL]: https://curl.se/
[rdftools]: https://github.com/hoijui/rdftools
[PATH]: https://en.wikipedia.org/wiki/PATH_(variable)
