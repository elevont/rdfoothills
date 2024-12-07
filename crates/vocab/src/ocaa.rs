// SPDX-FileCopyrightText: 2023 - 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! [Ontologies Cache and Analytics (OCAA)](
//! https://w3id.org/oseg/ont/ocaa)
//! vocabulary.

use crate::named_node;

pub const NS_BASE: &str = "https://w3id.org/oseg/ont/ocaa#";
pub const NS_PREFERRED_PREFIX: &str = "ocaa";

named_node!(
    ONTOLOGY_ANALYSIS,
    NS_BASE,
    "OntologyAnalysis",
    "Holds all results of the analysis of an ontology, including sub-analysis"
);
named_node!(
    IRI_ANALYSIS,
    NS_BASE,
    "IriAnalysis",
    "Holds all results of the analysis of an IRI of an Ontology/Vocabulary"
);
named_node!(
    CACHE_ANALYSIS,
    NS_BASE,
    "IriAnalysis",
    "Holds all results of the analysis of the (local) cache content related to an ontology or other RDF data file"
);
named_node!(
    CONTENT_FORMAT,
    NS_BASE,
    "ContentFormat",
    "A specific content format/serialized content instance of an ontology (might be missing (-> provided=false)!)"
);
// named_node!(
//     CONTENT,
//     NS_BASE,
//     "Content",
//     "A specific content format/serialized content instance of an ontology (might have state: missing)"
// );

named_node!(
    HAS_ANALYSIS,
    NS_BASE,
    "hasAnalysis",
    "Links a sub-analysis to its parent"
);
named_node!(
    HAS_CONTENT,
    NS_BASE,
    "hasContent",
    "Links a content format analysis to its ontology"
);
named_node!(
    HAS_NAMESPACE_IRI,
    NS_BASE,
    "hasNamespaceIri",
    "Links an IRI analysis to its ontology"
);
named_node!(
    MEDIA_TYPE,
    NS_BASE,
    "mediaType",
    "Links a concrete MIME-type to its content format analysis"
);

named_node!(
    HAS_MACHINE_READABLE,
    NS_BASE,
    "hasMachineReadable",
    "Whether the subject Ontology has machine-readable content cached"
);
named_node!(
    HAS_HUMAN_ORIENTED,
    NS_BASE,
    "hasHumanOriented",
    "Whether the subject Ontology has human oriented content cached"
);
named_node!(
    HAS_ANY,
    NS_BASE,
    "hasAny",
    "Whether the subject Ontology has any content cached"
);
// named_node!(
//     STATE,
//     NS_BASE,
//     "state",
//     "Indicates the state of a content(-format) within an ontology"
// );
// named_node!(
//     MISSING,
//     NS_BASE,
//     "missing",
//     "Indicates whether the subject is a content-format missing for and within the ontology that links to it (bool)"
// );
named_node!(
    PROVIDED,
    NS_BASE,
    "provided",
    "Indicates whether the subject is a content-format provided for and within the ontology that links to it (bool)"
);
// named_node!(
//     STATE_PROVIDED,
//     NS_BASE,
//     "missing",
//     "Indicates that the subject is a missing content-format within the ontology that links to it"
// );
// named_node!(
//     PROVIDES_MIME_TYPE,
//     NS_BASE,
//     "providesMimeType",
//     "Links a subject Ontology to a mime-type it is provided in by the original authors (usually either human-oriented HTML or one of the many RDF serialization formats)"
// );
named_node!(
    PROVIDED_BY_NAMESPACE_IRI,
    NS_BASE,
    "providedByNamespaceIri",
    "Indicates whether the subject is a content-format provided by the original namespace IRI for and within the ontology that links to it (bool)"
    // "Indicates that the object mime-type is provided by the original IRI vs an alternative one or none at all."
);
// named_node!(
//     PROVIDES_MIME_TYPE_BY_ORIG_IRI,
//     NS_BASE,
//     "providesMimeTypeByOrigIri",
//     "Indicates that the object mime-type is provided by the original IRI vs an alternative one or none at all."
// );
named_node!(
    URI_COMPATIBLE,
    NS_BASE,
    "uriCompatible",
    "Whether the subject IRI is compatible with the URI specification as well (vs only with the IRI specification)"
);
named_node!(
    USES_HTTP,
    NS_BASE,
    "usesHttp",
    "Whether the subject IRI uses HTTP or HTTPS as its scheme"
);
named_node!(
    USES_PURL,
    NS_BASE,
    "usesPurl",
    "Whether the subject IRI uses PURL (Permanent URL), for example `https://w3id.org/...` or `https://purl.org/...`"
);
named_node!(
    ENDS_WELL,
    NS_BASE,
    "endsWell",
    "Whether the subject Ontologies IRI/namespace ends with `/` or `#`, as is best-practice"
);
named_node!(
    PATH_FOLLOWS_BEST_PRACTICE,
    NS_BASE,
    "pathFollowsBestPractice",
    "Whether the subject Ontologies IRI/namespace path part conforms to best practice (see <https://more.metadatacenter.org/recommended-iri-patterns-ontologies-and-their-terms>)"
);
named_node!(
    HAS_NO_QUERY,
    NS_BASE,
    "hasNoQuery",
    "Whether the subject Ontologies IRI/namespace query part is empty, as is best-practice"
);
