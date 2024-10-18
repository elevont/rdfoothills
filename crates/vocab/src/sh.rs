// SPDX-FileCopyrightText: 2023 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! [SHACL](http://www.w3.org/ns/shacl) vocabulary.

use crate::named_node;

pub const NS_BASE: &str = "http://www.w3.org/ns/shacl#";
pub const NS_PREFERRED_PREFIX: &str = "sh";

named_node!(NODE_SHAPE, NS_BASE, "NodeShape", "A node shape is a shape that specifies constraint that need to be met with respect to focus nodes.");
named_node!(PROPERTY_SHAPE, NS_BASE, "PropertyShape","A property shape is a shape that specifies constraints on the values of a focus node for a given property or path.");
named_node!(TARGET_CLASS, NS_BASE, "targetClass", "Links a shape to a class, indicating that all instances of the class must conform to the shape.");
named_node!(
    CLOSED,
    NS_BASE,
    "closed",
    "If set to true then the shape is closed."
);
named_node!(
    PROPERTY,
    NS_BASE,
    "property",
    "Links a shape to its property shapes."
);
named_node!(
    PATH,
    NS_BASE,
    "path",
    "Specifies the property path of a property shape."
);
named_node!(
    MAX_COUNT,
    NS_BASE,
    "maxCount",
    "Specifies the maximum number of values in the set of value nodes."
);
named_node!(
    MIN_COUNT,
    NS_BASE,
    "minCount",
    "Specifies the minimum number of values in the set of value nodes."
);
named_node!(
    CLASS,
    NS_BASE,
    "class",
    "The type that all value nodes must have."
);
named_node!(
    DATA_TYPE,
    NS_BASE,
    "datatype",
    "Specifies an RDF datatype that all value nodes must have."
);
named_node!(
    OR,
    NS_BASE,
    "or",
    "Specifies the condition that each value node conforms to at least one of the provided shapes."
);
named_node!(
    NODE,
    NS_BASE,
    "node",
    "Specifies the node shape that all value nodes must conform to."
);
named_node!(PATTERN, NS_BASE, "pattern", "Specifies a regular expression pattern that the string representations of the value nodes must match.");
named_node!(
    NODE_KIND,
    NS_BASE,
    "nodeKind",
    "Specifies the node kind (e.g. IRI or literal) each value node."
);
named_node!(TARGET_OBJECTS_OF, NS_BASE, "targetObjectsOf", "Links a shape to a property, indicating that all all objects of triples that have the given property as their predicate must conform to the shape.");
named_node!(TARGET_SUBJECTS_OF, NS_BASE, "targetSubjectsOf", "Links a shape to a property, indicating that all subjects of triples that have the given property as their predicate must conform to the shape.");
named_node!(
    NAME,
    NS_BASE,
    "name",
    "Human-readable labels for the property in the context of the surrounding shape."
);
named_node!(
    DESCRIPTION,
    NS_BASE,
    "description",
    "Human-readable descriptions for the property in the context of the surrounding shape."
);
