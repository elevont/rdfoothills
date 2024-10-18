// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use oxiri::{IriParseError, IriRef};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub type Iri = IriRef<String>;

pub const PREFIX_EMPTY: &str = "";
pub const PREFIX_EMPTY_ID: &str = "__NO_PREFIX_ID__";

// TODO Find a better name then Prefix; maybe: OntId, OntIdPair, NamespaceId, RefixAndIri, PrefIri, ...
/// Represents a kind of basic ID for a set of RDF triples
/// that could be though of as being in one namespace,
/// most commonly this is used for ontologies within a file.
///
/// # Examples (Turtle format):
///
/// ```turtle
/// @prefix owl:      <http://www.w3.org/2002/07/owl#> .
/// @prefix rdf:      <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
/// @prefix rdfs:     <http://www.w3.org/2000/01/rdf-schema#> .
/// @prefix xsd:      <http://www.w3.org/2001/XMLSchema#> .
/// @prefix schema:   <http://schema.org/> .
/// ```
// #[derive(Debug, Clone)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Prefix {
    /// The short form, e.g. `xsd` or `schema`.
    /// See `::PREFIX_EMPTY`
    pub prefix: String,
    /// The extended/full form,
    /// e.g. `http://www.w3.org/2001/XMLSchema#`
    /// or `http://schema.org/`.
    pub iri: Iri,
}

impl Prefix {
    /// Creates a new instance of `Prefix`.
    ///
    /// # Errors
    ///
    /// Returns an `IriParseError` if the given `iri` is invalid.
    pub fn new(prefix_id: String, iri: String) -> Result<Self, IriParseError> {
        Ok(Self {
            prefix: prefix_id,
            iri: IriRef::parse(iri)?,
        })
    }

    /// Returns the `@base` of the IRI.
    /// This is simply the IRI without the last character.
    ///
    /// # Examples
    ///
    /// - `http://www.w3.org/2001/XMLSchema#` -> \
    ///   `http://www.w3.org/2001/XMLSchema`
    /// - `http://schema.org/` -> \
    ///   `http://schema.org`
    ///
    /// # Panics
    ///
    /// If the IRI does not end with a common delimiter, e.g. `#` or `/`.
    #[must_use]
    pub fn base(&self) -> &str {
        let iri_str = self.iri.as_str();
        if iri_str.ends_with('#') || iri_str.ends_with('/') {
            #[allow(clippy::indexing_slicing)]
            #[allow(clippy::string_slice)]
            &iri_str[0..self.iri.as_str().len() - 1]
        } else {
            panic!(
                "IRI {} is not a base plus common delimiter suffix ('/' or '#')",
                self.iri
            );
        }
    }

    /// Returns a _non empty_ "version" of the prefix-ID.
    /// This is either `self.prefix` or `::PREFIX_EMPTY_ID`.
    ///
    /// # Examples
    ///
    /// - `"xsd"` -> \
    ///   `"xsd"`
    /// - `"schema"` -> \
    ///   `"schema"`
    /// - `""` -> \
    ///   `"__NO_PREFIX_ID__"`
    #[must_use]
    pub fn prefix_id(&self) -> &str {
        if self.prefix == PREFIX_EMPTY {
            PREFIX_EMPTY_ID
        } else {
            self.prefix.as_str()
        }
    }
}
