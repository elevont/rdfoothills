// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    io::Read,
    rc::Rc,
};

use const_format::concatcp;
use convert_case::{Case, Casing};
use oxrdf::{NamedNode, Subject, Term};
use oxrdfio::{RdfFormat, RdfParser};
use petgraph::graph::{DefaultIx, DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use thiserror::Error;
use tracing;

const PF_CC: &str = "http://creativecommons.org/ns#";
// const PF_DCAT: &str = "http://www.w3.org/ns/dcat#";
const PF_DCTERMS: &str = "http://purl.org/dc/terms/";
const PF_OWL: &str = "http://www.w3.org/2002/07/owl#";
const PF_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const PF_RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const PF_SCHEMA: &str = "http://schema.org/";
const PF_SH: &str = "http://www.w3.org/ns/shacl#";
const PF_VANN: &str = "http://purl.org/vocab/vann/";
const PF_VS: &str = "http://www.w3.org/2003/06/sw-vocab-status/ns#";
// const PF_XSD: &str = "http://www.w3.org/2001/XMLSchema#";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct PrefixedIri {
    prefix_name: String,
    prefix_value: String,
    postfix: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ParsedNamedNode {
    Prefixed(PrefixedIri),
    BaseRelative(PrefixedIri),
    Full(NamedNode),
}

#[derive(Error, Debug)]
pub enum VocabExtractError {
    #[error("No owl:Ontology subject found!")]
    MissingOntology,
}

#[derive(Error, Debug)]
pub enum RustVocabGenError {
    #[error("The vocabulary property `preferred_namespace_prefix` is required")]
    MissingNamespacePrefix,
    #[error("The vocabulary property `prefix_namespace_uri` is required")]
    MissingNamespaceUri,
}

impl Display for ParsedNamedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefixed(node) => write!(f, "{}:{}", node.prefix_name, node.postfix),
            Self::BaseRelative(node) => write!(f, "<{}>", node.postfix),
            Self::Full(node) => write!(f, "{}", node.as_str()),
        }
    }
}

impl ParsedNamedNode {
    fn raw(&self) -> String {
        match self {
            Self::Prefixed(node) | Self::BaseRelative(node) => {
                format!("{}{}", node.prefix_value, node.postfix)
            }
            Self::Full(node) => node.as_str().to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Node {
    Iri(ParsedNamedNode),
    BlankNode,
    Literal(String),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Deprecation {
    enabled: bool,
    since: String,
    message: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SubjectMeta {
    postfix: String,
    title: String,
    description: String,
    deprecation: Deprecation,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iri(node) => node.fmt(f),
            Self::BlankNode => write!(f, "[]"),
            Self::Literal(lit_str) => {
                if lit_str.contains('\n') {
                    write!(f, r#""""{lit_str}""""#)
                } else {
                    write!(f, r#""{lit_str}""#)
                }
            }
        }
    }
}

type NodeIdx = NodeIndex<DefaultIx>;
pub type Edge = Node;

pub type RdfGraph = DiGraph<Node, Edge>;

#[derive(Debug, Clone)]
pub struct RdfContent {
    pub graph: Rc<RdfGraph>,
    pub subjects: HashSet<NodeIndex<u32>>,
    pub base: Option<String>,
    pub prefixes: Vec<(String, String)>,
}

pub struct VocabInfo {
    pub content: RdfContent,
    pub title: Option<String>,
    pub description: Option<String>,
    pub preferred_namespace_prefix: Option<String>,
    pub preferred_namespace_uri: Option<String>,
    pub subjects: Vec<SubjectMeta>,
}

impl RdfContent {
    /// Serializes the RDF content to RDF/Turtle (*.ttl).
    ///
    /// # Panics
    ///
    /// If one of the subjects node IDs is not within the content;
    /// This is not going to happen,
    /// if one did not directly meddle with the content or the subject IDs.
    #[must_use]
    pub fn to_turtle(&self) -> String {
        let mut turtle = String::new();

        for subj_idx in &self.subjects {
            let subj = self.graph.node_weight(*subj_idx).unwrap();
            turtle.push('\n');
            turtle.push_str(subj.to_string().as_str());
            turtle.push('\n');
            for pred_ref in self.graph.edges(*subj_idx) {
                let pred = pred_ref.weight();
                let obj_idx = pred_ref.target();
                let obj = self.graph.node_weight(obj_idx).unwrap();
                turtle.push_str("  ");
                turtle.push_str(pred.to_string().as_str());
                turtle.push(' ');
                turtle.push_str(obj.to_string().as_str());
                turtle.push_str(" ;\n");
            }
            turtle.push_str("  .\n");
        }

        turtle
    }

    #[must_use]
    pub fn extract_for_subject(&self, subj_idx: NodeIndex<DefaultIx>) -> Self {
        let mut copy = self.clone();
        copy.subjects.clear();
        copy.subjects.insert(subj_idx);
        copy
    }

    /// Extract the literal string of the pointed to node.
    ///
    /// # Panics
    ///
    /// If the given node-ID points to as non-literal node.
    #[must_use]
    pub fn extract_literal_string(&self, node_idx: NodeIndex<DefaultIx>) -> String {
        let obj = self.graph.node_weight(node_idx).unwrap();
        if let Node::Literal(lit) = obj {
            lit.clone()
        } else {
            panic!("Expected literal, got {obj}");
        }
    }

    #[must_use]
    fn find_ontology(&self) -> Option<NodeIdx> {
        let mut ont_subj_idx_opt = None;
        'subj_loop: for subj_idx in &self.subjects {
            for pred_ref in self.graph.edges(*subj_idx) {
                let pred = pred_ref.weight();
                if let Node::Iri(pred_node) = pred {
                    if pred_node.raw() == concatcp!(PF_RDF, "type") {
                        let obj_idx = pred_ref.target();
                        let obj = self.graph.node_weight(obj_idx).unwrap();
                        if let Node::Iri(obj_node) = obj {
                            if [concatcp!(PF_OWL, "Ontology")].contains(&obj_node.raw().as_str()) {
                                // This is the ontology subject!
                                ont_subj_idx_opt = Some(*subj_idx);
                                break 'subj_loop;
                            }
                        }
                    }
                }
            }
        }
        ont_subj_idx_opt
    }

    #[must_use]
    fn extract_subj_metas(&self, ont_subj_idx: NodeIdx) -> Vec<SubjectMeta> {
        let mut subjects = Vec::new();
        for subj_idx in &self.subjects {
            if *subj_idx == ont_subj_idx {
                continue;
            }
            let postfix;
            let mut title = None;
            let mut description = None;
            let mut deprecation_enabled = None;
            let mut deprecation_since = None;
            let mut deprecation_message = None;
            let subj = self.graph.node_weight(*subj_idx).unwrap();
            if let Node::Iri(ParsedNamedNode::Prefixed(ref prefxd)) = subj {
                postfix = prefxd.postfix.clone();
            } else {
                panic!("Expected prefixed node, got {subj}");
            }
            for pred_ref in self.graph.edges(*subj_idx) {
                let pred = pred_ref.weight();
                if let Node::Iri(pred_node) = pred {
                    if [concatcp!(PF_DCTERMS, "title"), concatcp!(PF_RDFS, "label")]
                        .contains(&pred_node.raw().as_str())
                    {
                        title = Some(self.extract_literal_string(pred_ref.target()));
                    } else if [
                        concatcp!(PF_DCTERMS, "description"),
                        concatcp!(PF_RDFS, "comment"),
                    ]
                    .contains(&pred_node.raw().as_str())
                    {
                        description = Some(self.extract_literal_string(pred_ref.target()));
                    } else if pred_node.raw().as_str() == concatcp!(PF_VS, "term_status") {
                        deprecation_enabled = Some(
                            self.extract_literal_string(pred_ref.target())
                                .to_lowercase()
                                == "deprecated",
                        );
                    } else if pred_node.raw().as_str() == concatcp!(PF_OWL, "deprecated") {
                        deprecation_enabled = Some(
                            self.extract_literal_string(pred_ref.target())
                                .to_lowercase()
                                == "true",
                        );
                    } else if pred_node.raw().as_str() == concatcp!(PF_CC, "deprecatedOn") {
                        deprecation_since = Some(self.extract_literal_string(pred_ref.target()));
                    } else if pred_node.raw().as_str() == concatcp!(PF_SCHEMA, "supersededBy") {
                        let obj = self.graph.node_weight(pred_ref.target()).unwrap();
                        deprecation_message = Some(format!("Use this instead: {obj}"));
                    }
                }
            }
            #[allow(clippy::shadow_reuse)]
            let title = title.unwrap_or_else(|| format!("No title found for {subj}"));
            #[allow(clippy::shadow_reuse)]
            let mut description =
                description.map_or_else(String::new, |desc| format!("{desc}\n\n"));
            let rdf_content = self.extract_for_subject(*subj_idx);
            description.push_str(&rdf_content.to_turtle());
            subjects.push(SubjectMeta {
                postfix,
                title,
                description,
                deprecation: Deprecation {
                    enabled: deprecation_enabled.unwrap_or(false),
                    since: deprecation_since.unwrap_or_else(String::new),
                    message: deprecation_message.unwrap_or_else(String::new),
                },
            });
        }

        subjects
    }

    /// Extract vocabulary/ontology meta-data.
    ///
    /// # Errors
    ///
    /// If no `owl:Ontology` subject was found.
    pub fn into_vocab_info(self) -> Result<VocabInfo, VocabExtractError> {
        if let Some(ont_subj_idx) = self.find_ontology() {
            let mut preferred_namespace_prefix = None;
            let mut preferred_namespace_uri = None;
            let mut title = None;
            let mut description = None;
            for pred_ref in self.graph.edges(ont_subj_idx) {
                let pred = pred_ref.weight();
                if let Node::Iri(pred_node) = pred {
                    if pred_node.raw() == concatcp!(PF_VANN, "preferredNamespacePrefix") {
                        preferred_namespace_prefix =
                            Some(self.extract_literal_string(pred_ref.target()));
                    } else if pred_node.raw() == concatcp!(PF_VANN, "preferredNamespaceUri") {
                        preferred_namespace_uri =
                            Some(self.extract_literal_string(pred_ref.target()));
                    } else if [concatcp!(PF_DCTERMS, "title"), concatcp!(PF_RDFS, "label")]
                        .contains(&pred_node.raw().as_str())
                    {
                        title = Some(self.extract_literal_string(pred_ref.target()));
                    } else if [
                        concatcp!(PF_DCTERMS, "description"),
                        concatcp!(PF_RDFS, "comment"),
                    ]
                    .contains(&pred_node.raw().as_str())
                    {
                        description = Some(self.extract_literal_string(pred_ref.target()));
                    }
                }
            }

            let subjects = self.extract_subj_metas(ont_subj_idx);

            return Ok(VocabInfo {
                content: self,
                title,
                description,
                preferred_namespace_prefix,
                preferred_namespace_uri,
                subjects,
            });
        }

        Err(VocabExtractError::MissingOntology)
    }
}

// sh:declare [
//   sh:prefix "cmt" ;
//   sh:namespace "https://w3id.org/oseg/ont/cmt#"^^xsd:anyURI ;
// ] ;
// schema:comment """
// # SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
// # SPDX-License-Identifier: CC-BY-SA-4.0
// """ ;
// dcterms:source "https://codeberg.org/elevont/cmt-ont/master/src/ont/okh.ttl"^^xsd:anyURI ;
// schema:codeRepository "https://codeberg.org/elevont/cmt-ont/"^^xsd:anyURI ;
// dcat:keyword "meta", "comments", "notes" ;

impl VocabInfo {
    /// Convert to Rust vocab code.
    ///
    /// # Errors
    ///
    /// - The `preferred_namespace_prefix` property is set to `None`.
    /// - The `preferred_namespace_uri` property is set to `None`.
    pub fn to_str(&self) -> Result<String, RustVocabGenError> {
        let namespace_prefix = self
            .preferred_namespace_prefix
            .as_ref()
            .ok_or(RustVocabGenError::MissingNamespacePrefix)?;
        let namespace_uri = self
            .preferred_namespace_uri
            .as_ref()
            .ok_or(RustVocabGenError::MissingNamespaceUri)?;
        let title = self.title.as_deref().unwrap_or("NO_TITLE");
        let mut vocab = format!(
            r#"
//! [{title} ({})](
//! {namespace_uri})
//! vocabulary.

use crate::{{named_node, named_node_deprecated}};

pub const NS_BASE: &str = "{namespace_uri}";
pub const NS_PREFERRED_PREFIX: &str = "{namespace_prefix}";

"#,
            namespace_prefix.to_ascii_uppercase(),
        );

        let mut seen_consts = HashSet::new();
        for subj in &self.subjects {
            let subj_postfix_const_base = format!(
                "{}{}",
                if subj.deprecation.enabled {
                    "DEPRECATED_"
                } else {
                    ""
                },
                subj.postfix.to_case(Case::Constant)
            );
            let mut subj_postfix_const = subj_postfix_const_base.clone();
            // Ensure that the chosen constant name is unique within the file
            let mut distinguishing_idx = 1;
            while seen_consts.contains(&subj_postfix_const) {
                distinguishing_idx += 1;
                subj_postfix_const.clear();
                subj_postfix_const.push_str(&subj_postfix_const_base);
                subj_postfix_const.push_str("__");
                subj_postfix_const.push_str(distinguishing_idx.to_string().as_str());
            }
            let deprecation_args = if subj.deprecation.enabled {
                format!(
                    ",
    r#\"{}\"#,
    r#\"{}\"#",
                    subj.deprecation.since, subj.deprecation.message
                )
            } else {
                String::new()
            };
            // NOTE: This prevents triggering a false positive
            #[allow(clippy::needless_raw_string_hashes)]
            let subj_str = format!(
                r###"
named_node{}!(
    {subj_postfix_const},
    NS_BASE,
    "{}",
    r#"{}"#{}
);
"###,
                if subj.deprecation.enabled {
                    "_deprecated"
                } else {
                    ""
                },
                subj.postfix,
                subj.description,
                deprecation_args,
            );
            seen_consts.insert(subj_postfix_const);
            vocab.push_str(&subj_str);
        }

        Ok(vocab)
    }
}

fn parse_iri(
    subj: &NamedNode,
    base: Option<&str>,
    prefixes: &Vec<(&str, &str)>,
) -> ParsedNamedNode {
    for prefix in prefixes {
        if subj.as_str().starts_with(prefix.1) {
            return ParsedNamedNode::Prefixed(PrefixedIri {
                prefix_name: prefix.0.to_string(),
                prefix_value: prefix.1.to_string(),
                // postfix: subj.as_str()[prefix.1.len()..].to_string(),
                postfix: subj.as_str().strip_prefix(prefix.1).unwrap().to_string(),
            });
        }
    }
    if let Some(base_iri) = base {
        if subj.as_str().starts_with(base_iri) {
            return ParsedNamedNode::BaseRelative(PrefixedIri {
                prefix_name: String::new(),
                prefix_value: base_iri.to_owned(),
                // postfix: subj.as_str().[base_iri.len()..].to_string(),
                postfix: subj.as_str().strip_prefix(base_iri).unwrap().to_string(),
            });
        }
    }
    ParsedNamedNode::Full(subj.clone())
}

pub fn rdf<R>(input: R, format: RdfFormat) -> RdfContent
where
    R: Read,
{
    let mut graph = RdfGraph::new();
    let mut subjects = HashSet::new();

    let mut parser = RdfParser::from_format(format).for_reader(input);
    let mut iri_to_graph_idx = HashMap::new();
    while let Some(Ok(quad)) = parser.next() {
        if let Subject::NamedNode(subj) = &quad.subject {
            let prefixes = parser.prefixes().collect::<Vec<_>>();
            let base = parser.base_iri();

            let subj_inner = parse_iri(subj, base, &prefixes);
            // let subj_grp = subj_inner.group();
            let subj_iri = Node::Iri(subj_inner);
            let pred_iri = Node::Iri(parse_iri(&quad.predicate, base, &prefixes));

            let obj_node = match quad.object {
                Term::NamedNode(nn) => Node::Iri(parse_iri(&nn, base, &prefixes)),
                Term::BlankNode(bn) => {
                    tracing::warn!("BlankNode objects are not supported -> ignored! {:?}", bn);
                    continue;
                }
                Term::Literal(lit) => Node::Literal(lit.value().to_string()),
                Term::Triple(tr) => {
                    tracing::warn!("Triple objects are not supported -> ignored! {:?}", tr);
                    continue;
                }
            };

            let subj_idx = *iri_to_graph_idx
                .entry(subj_iri.clone())
                .or_insert_with(|| graph.add_node(subj_iri));
            let obj_idx = *iri_to_graph_idx
                .entry(obj_node.clone())
                .or_insert_with(|| graph.add_node(obj_node));
            subjects.insert(subj_idx);
            graph.add_edge(subj_idx, obj_idx, pred_iri);
        } else {
            tracing::warn!("Ignoring triple with subject: {quad:?}");
        }
    }

    RdfContent {
        graph: Rc::new(graph),
        subjects,
        base: parser.base_iri().map(std::borrow::ToOwned::to_owned),
        prefixes: parser
            .prefixes()
            .map(|p| (p.0.to_owned(), p.1.to_owned()))
            .collect(),
    }
}
