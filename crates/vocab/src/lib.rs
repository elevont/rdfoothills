// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Provides ready to use [`NamedNodeRef`](super::NamedNodeRef)s
//! for basic RDF vocabularies.

#![allow(dead_code)]

pub mod ocaa;
pub mod owl;
pub mod sh;

use git_version::git_version;

// This tests rust code in the README with doc-tests.
// Though, It will not appear in the generated documentation.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

pub const VERSION: &str = git_version!(cargo_prefix = "", fallback = "unknown");

#[macro_export]
macro_rules! named_node {
    ($const:ident, $base:expr, $node:literal, $doc:literal) => {
        #[doc=$doc]
        pub const $const: oxrdf::NamedNodeRef<'_> =
            oxrdf::NamedNodeRef::new_unchecked(const_format::concatcp!($base, $node));
    };
}

#[macro_export]
macro_rules! named_node_deprecated {
    ($const:ident, $base:expr, $node:literal, $doc:literal, $since:literal, $note:literal) => {
        #[allow(clippy::deprecated_semver)]
        #[deprecated(since=$since, note=$note)]
        #[doc=$doc]
        pub const $const: oxrdf::NamedNodeRef<'_> =
            oxrdf::NamedNodeRef::new_unchecked(const_format::concatcp!($base, $node));
    };
}

#[macro_export]
macro_rules! typed_literal {
    ($const:ident, $value:literal, $rdf_type:expr) => {
        pub static $const: LazyLock<TermRef<'_>> =
            LazyLock::new(|| TermRef::Literal(LiteralRef::new_typed_literal($value, $rdf_type)));
    };
}

pub mod basics {
    use oxrdf::{vocab::xsd, LiteralRef, TermRef};
    use std::sync::LazyLock;

    pub const NS_BASE_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    typed_literal!(BOOL_FALSE, "false", xsd::BOOLEAN);
    typed_literal!(BOOL_TRUE, "true", xsd::BOOLEAN);

    #[must_use]
    pub fn rdf_bool(arg: bool) -> TermRef<'static> {
        if arg {
            *BOOL_TRUE
        } else {
            *BOOL_FALSE
        }
    }
}
