// SPDX-FileCopyrightText: 2023 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! [Web Ontology Language (OWL)](
//! http://www.w3.org/2002/07/owl)
//! vocabulary.

use crate::named_node;

pub const NS_BASE: &str = "http://www.w3.org/2002/07/owl#";
pub const NS_PREFERRED_PREFIX: &str = "owl";

named_node!(
    DATATYPE_PROPERTY,
    NS_BASE,
    "DatatypeProperty",
    "The class of data properties."
);
named_node!(CLASS, NS_BASE, "Class", "TODO"); // TODO Fill in description
named_node!(OBJECT_PROPERTY, NS_BASE, "ObjectProperty", "TODO"); // TODO Fill in description
