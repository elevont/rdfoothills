// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mediatype::{
    names::{APPLICATION, TEXT},
    MediaType,
};
use once_cell::sync::Lazy;
use std::{
    borrow::Cow,
    ffi::OsStr,
    fmt::Display,
    path::{Path as StdPath, PathBuf},
};
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;

use crate::hasher;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unrecognized ontology content-type (mime-type): '{0}'")]
    UnrecognizedContentType(String),

    #[error("Generic content-type, could be any RDF type or none: '{0}'")]
    CouldBeAny(String),

    #[error("Not a valid/parsable content-type format: '{0}'")]
    InvalidFormat(#[from] mediatype::MediaTypeError),

    #[error("Unrecognized ontology file extension: '{0}'")]
    UnrecognizedFileExtension(String),

    #[error("File has no extension: '{0}'")]
    NoFileExtension(PathBuf),

    #[error("Unrecognized ontology file content")]
    UnrecognizedContent,
}

const MIME_TYPE_BINARY_RDF: &str = "application/x-binary-rdf";
const MIME_TYPE_CSVW: &str = "text/csv";
// const MIME_TYPE_HDT: &str = "NONE"; // See <https://www.w3.org/submissions/2011/SUBM-HDT-20110330/#media>: "The media type of HDT is the media type of their parts"
const MIME_TYPE_HEX_TUPLES: &str = "application/hex+x-ndjson";
const MIME_TYPE_HTML: &str = "text/html";
const MIME_TYPE_HTML_2: &str = "application/xhtml+xml";
const MIME_TYPE_JSON_LD: &str = "application/ld+json";
const MIME_TYPE_JSON_LD_2: &str = "application/json-ld"; // JSON-LD (invalid/inofficial form)
const MIME_TYPE_MICRODATA: &str = "application/x-microdata"; // TODO should this be application/x-microdata+json?
const MIME_TYPE_N3: &str = "text/rdf+n3";
const MIME_TYPE_N3_2: &str = "text/n3";
const MIME_TYPE_ND_JSON_LD: &str = "application/x-ld+ndjson";
const MIME_TYPE_N_QUADS: &str = "application/n-quads";
const MIME_TYPE_N_QUADS_STAR: &str = "application/n-quadsstar"; // TODO This is a pure guess so far
const MIME_TYPE_N_TRIPLES: &str = "application/n-triples";
const MIME_TYPE_N_TRIPLES_STAR: &str = "application/n-triplesstar"; // TODO This is a pure guess so far
const MIME_TYPE_OWL_XML: &str = "application/owl+xml"; // .owx
const MIME_TYPE_OWL_FUNCTIONAL: &str = "text/owl-functional"; // .ofn
const MIME_TYPE_RDF_A: &str = "text/html";
const MIME_TYPE_RDF_JSON: &str = "application/rdf+json";
const MIME_TYPE_RDF_XML: &str = "application/rdf+xml";
const MIME_TYPE_TRIG: &str = "text/trig";
const MIME_TYPE_TRIG_STAR: &str = "application/x-trigstar";
const MIME_TYPE_TRIX: &str = "application/trix";
const MIME_TYPE_TSVW: &str = "text/tab-separated-values";
const MIME_TYPE_TURTLE: &str = "text/turtle";
const MIME_TYPE_TURTLE_STAR: &str = "text/x-turtlestar";
const MIME_TYPE_TURTLE_STAR_2: &str = "application/x-turtlestar";
const MIME_TYPE_YAML_LD: &str = "application/ld+yaml";

const MEDIA_TYPE_BINARY_RDF: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-binary-rdf"));
const MEDIA_TYPE_CSVW: MediaType = MediaType::new(TEXT, mediatype::names::CSV);
// const MEDIA_TYPE_HDT: MediaType =
//     MediaType::new(APPLICATION, mediatype::Name::new_unchecked("hdt")); // See <https://www.w3.org/submissions/2011/SUBM-HDT-20110330/#media>: "The media type of HDT is the media type of their parts"
const MEDIA_TYPE_HEX_TUPLES: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::Name::new_unchecked("hex"),
    Some(mediatype::Name::new_unchecked("x-ndjson")),
    &[],
);
const MEDIA_TYPE_HTML: MediaType = MediaType::new(TEXT, mediatype::names::HTML);
const MEDIA_TYPE_HTML_2: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::names::XHTML,
    Some(mediatype::names::XML),
    &[],
);
const MEDIA_TYPE_JSON_LD: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::names::LD,
    Some(mediatype::names::JSON),
    &[],
);
const MEDIA_TYPE_JSON_LD_2: MediaType =
    MediaType::new(TEXT, mediatype::Name::new_unchecked("json-ld"));
const MEDIA_TYPE_MICRODATA: MediaType = MediaType::new(TEXT, mediatype::names::HTML);
// const MEDIA_TYPE_MICRODATA_2: MediaType = MediaType::from_parts(
//     APPLICATION,
//     mediatype::names::XHTML,
//     Some(mediatype::names::XML),
//     &[],
// );
const MEDIA_TYPE_N3: MediaType = MediaType::new(TEXT, mediatype::names::N3);
const MEDIA_TYPE_N3_2: MediaType =
    MediaType::from_parts(TEXT, mediatype::names::RDF, Some(mediatype::names::N3), &[]);
const MEDIA_TYPE_ND_JSON_LD: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-ld+ndjson"));
const MEDIA_TYPE_N_QUADS: MediaType = MediaType::new(APPLICATION, mediatype::names::N_QUADS);
const MEDIA_TYPE_N_QUADS_2: MediaType =
    MediaType::new(TEXT, mediatype::Name::new_unchecked("x-nquads"));
const MEDIA_TYPE_N_QUADS_3: MediaType = MediaType::new(TEXT, mediatype::names::N_QUADS);
const MEDIA_TYPE_N_QUADS_STAR: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("n-quadsstar")); // TODO This is a pure guess so far
const MEDIA_TYPE_N_TRIPLES: MediaType = MediaType::new(APPLICATION, mediatype::names::N_TRIPLES);
const MEDIA_TYPE_N_TRIPLES_STAR: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("n-triplesstar")); // TODO This is a pure guess so far
const MEDIA_TYPE_RDF_A: MediaType = MediaType::new(TEXT, mediatype::names::HTML);
// const MEDIA_TYPE_RDF_A_2: MediaType = MediaType::from_parts(
//     APPLICATION,
//     mediatype::names::XHTML,
//     Some(mediatype::names::XML),
//     &[],
// );
const MEDIA_TYPE_RDF_JSON: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::names::RDF,
    Some(mediatype::names::JSON),
    &[],
);
const MEDIA_TYPE_RDF_XML: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::names::RDF,
    Some(mediatype::names::XML),
    &[],
);
const MEDIA_TYPE_RDF_XML_2: MediaType = MediaType::new(APPLICATION, mediatype::names::XML);
const MEDIA_TYPE_RDF_XML_3: MediaType = MediaType::new(TEXT, mediatype::names::XML);
const MEDIA_TYPE_TRIG: MediaType = MediaType::new(APPLICATION, mediatype::names::TRIG);
const MEDIA_TYPE_TRIG_2: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-trig"));
const MEDIA_TYPE_TRIG_STAR: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-trigstar"));
const MEDIA_TYPE_TRIX: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("trix"));
const MEDIA_TYPE_TSVW: MediaType = MediaType::new(TEXT, mediatype::names::TAB_SEPARATED_VALUES);
const MEDIA_TYPE_TURTLE: MediaType = MediaType::new(TEXT, mediatype::names::TURTLE);
const MEDIA_TYPE_TURTLE_2: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-turtle"));
const MEDIA_TYPE_TURTLE_STAR: MediaType =
    MediaType::new(TEXT, mediatype::Name::new_unchecked("x-turtlestar"));
const MEDIA_TYPE_TURTLE_STAR_2: MediaType =
    MediaType::new(APPLICATION, mediatype::Name::new_unchecked("x-turtlestar"));
const MEDIA_TYPE_YAML_LD: MediaType = MediaType::from_parts(
    APPLICATION,
    mediatype::names::LD,
    Some(mediatype::Name::new_unchecked("yaml")),
    &[],
);

const MEDIA_TYPE_TEXT_PLAIN: MediaType = MediaType::new(TEXT, mediatype::names::PLAIN);

const FEXT_BINARY_RDF: &str = "brf";
const FEXT_HDT: &str = "hdt"; // TODO This is a pure guess so far
const FEXT_CSVW: &str = "csvw";
const FEXT_CSVW_2: &str = "csv";
const FEXT_HEX_TUPLES: &str = "hext";
const FEXT_HTML: &str = "html";
const FEXT_HTML_2: &str = "xhtml";
const FEXT_HTML_3: &str = "htm";
const FEXT_JSON_LD: &str = "jsonld";
const FEXT_MICRODATA: &str = "html";
const FEXT_MICRODATA_2: &str = "xhtml";
const FEXT_MICRODATA_3: &str = "htm";
const FEXT_N3: &str = "n3";
const FEXT_ND_JSON_LD: &str = ".ndjsonld";
const FEXT_ND_JSON_LD_2: &str = ".jsonl";
const FEXT_ND_JSON_LD_3: &str = ".ndjson";
const FEXT_N_QUADS: &str = "nq";
const FEXT_N_QUADS_STAR: &str = "nqs"; // TODO This is a pure guess so far
const FEXT_N_TRIPLES: &str = "nt";
const FEXT_N_TRIPLES_STAR: &str = "nts"; // TODO This is a pure guess so far
const FEXT_OWL_XML: &str = "owx";
const FEXT_OWL_FUNCTIONAL: &str = "ofn";
const FEXT_RDF_A: &str = "html";
const FEXT_RDF_A_2: &str = "xhtml";
const FEXT_RDF_A_3: &str = "htm";
const FEXT_RDF_JSON: &str = "rj";
const FEXT_RDF_XML: &str = "rdf";
const FEXT_RDF_XML_2: &str = "rdfs";
const FEXT_RDF_XML_3: &str = "owl";
const FEXT_RDF_XML_4: &str = "xml";
const FEXT_TRIG: &str = "trig";
const FEXT_TRIG_STAR: &str = "trigs";
const FEXT_TRIX: &str = "trix";
const FEXT_TRIX_2: &str = "xml";
const FEXT_TSVW: &str = "tsvw";
const FEXT_TSVW_2: &str = "tsv";
const FEXT_TURTLE: &str = "ttl";
const FEXT_TURTLE_STAR: &str = "ttls";
const FEXT_YAML_LD: &str = "yamlld";
const FEXT_YAML_LD_2: &str = "ymlld";

pub fn media_type2type(media_type: &MediaType) -> Option<Type> {
    let search_hash = hasher::hash_num(media_type);
    MEDIA_TYPE_2_MIME.get(&search_hash).copied()
}

pub static MEDIA_TYPE_2_MIME: Lazy<HashMap<u64, Type>> = Lazy::new(|| {
    vec![
        (MEDIA_TYPE_BINARY_RDF, Type::BinaryRdf),
        (MEDIA_TYPE_CSVW, Type::Csvw),
        // (MEDIA_TYPE_HDT, Type::), // NOTE Does not have its own media type
        (MEDIA_TYPE_HEX_TUPLES, Type::HexTuples),
        (MEDIA_TYPE_HTML, Type::Html),
        (MEDIA_TYPE_HTML_2, Type::Html),
        (MEDIA_TYPE_JSON_LD, Type::JsonLd),
        (MEDIA_TYPE_JSON_LD_2, Type::JsonLd),
        // (MEDIA_TYPE_MICRODATA, Type::Microdata),
        // (MEDIA_TYPE_MICRODATA_2, Type::Microdata),
        (MEDIA_TYPE_N3, Type::N3),
        (MEDIA_TYPE_N3_2, Type::N3),
        (MEDIA_TYPE_ND_JSON_LD, Type::NdJsonLd),
        (MEDIA_TYPE_N_QUADS, Type::NQuads),
        (MEDIA_TYPE_N_QUADS_2, Type::NQuads),
        (MEDIA_TYPE_N_QUADS_3, Type::NQuads),
        (MEDIA_TYPE_N_QUADS_STAR, Type::NQuadsStar),
        (MEDIA_TYPE_N_TRIPLES, Type::NTriples),
        (MEDIA_TYPE_N_TRIPLES_STAR, Type::NTriplesStar),
        // (MEDIA_TYPE_RDF_A, Type::RdfA),
        // (MEDIA_TYPE_RDF_A_2, Type::RdfA),
        (MEDIA_TYPE_RDF_JSON, Type::RdfJson),
        (MEDIA_TYPE_RDF_XML, Type::RdfXml),
        (MEDIA_TYPE_RDF_XML_2, Type::RdfXml),
        (MEDIA_TYPE_RDF_XML_3, Type::RdfXml),
        (MEDIA_TYPE_TRIG, Type::TriG),
        (MEDIA_TYPE_TRIG_2, Type::TriG),
        (MEDIA_TYPE_TRIG_STAR, Type::TriGStar),
        (MEDIA_TYPE_TRIX, Type::TriX),
        (MEDIA_TYPE_TSVW, Type::Tsvw),
        (MEDIA_TYPE_TURTLE, Type::Turtle),
        (MEDIA_TYPE_TURTLE_2, Type::Turtle),
        (MEDIA_TYPE_TURTLE_STAR, Type::TurtleStar),
        (MEDIA_TYPE_TURTLE_STAR_2, Type::TurtleStar),
        (MEDIA_TYPE_YAML_LD, Type::YamlLd),
    ]
    .into_iter()
    .map(|(mtype, tpy)| (hasher::hash_num(mtype), tpy))
    .collect()
});

/**
 * The different mime-types of RDF Ontologies.
 */
#[derive(Copy, Clone, Debug, Default, /*DeserializeFromStr,*/ PartialEq, Eq, Hash)]
pub enum Type {
    BinaryRdf,
    Csvw,
    Hdt,
    HexTuples,
    #[default]
    Html,
    JsonLd,
    Microdata,
    N3,
    NdJsonLd,
    NQuads,
    NQuadsStar,
    NTriples,
    NTriplesStar,
    RdfA,
    RdfJson,
    RdfXml,
    TriG,
    TriGStar,
    TriX,
    Tsvw,
    Turtle,
    TurtleStar,
    YamlLd,
}

impl FromStr for Type {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_mime_type(s)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}

impl Type {
    #[must_use]
    pub fn main() -> Vec<Self> {
        vec![Self::Html, Self::JsonLd, Self::RdfXml, Self::Turtle]
    }

    pub fn from_mime_type<'a, T>(mime_type: T) -> Result<Self, ParseError>
    where
        T: Into<Cow<'a, str>>,
    {
        let mime_type_cow = mime_type.into();
        let media_type = MediaType::parse(mime_type_cow.as_ref())?;
        Self::from_media_type(&media_type)
    }

    pub fn from_media_type(media_type: &MediaType) -> Result<Self, ParseError> {
        // let value_opt = media_type2type(media_type);
        if media_type.essence() == MEDIA_TYPE_TEXT_PLAIN {
            return Err(ParseError::CouldBeAny(media_type.to_string()));
        }
        media_type2type(media_type)
            .ok_or_else(|| ParseError::UnrecognizedContentType(media_type.to_string()))
    }

    pub fn from_file_ext(file_ext: &str) -> Result<Self, ParseError> {
        Ok(match file_ext.to_lowercase().as_str() {
            FEXT_BINARY_RDF => Self::BinaryRdf,
            FEXT_CSVW | FEXT_CSVW_2 => Self::Csvw,
            FEXT_HDT => Self::Hdt,
            FEXT_HTML | FEXT_HTML_2 | FEXT_HTML_3 => Self::Html,
            FEXT_JSON_LD => Self::JsonLd,
            FEXT_N3 => Self::N3,
            FEXT_ND_JSON_LD | FEXT_ND_JSON_LD_2 | FEXT_ND_JSON_LD_3 => Self::NdJsonLd,
            FEXT_N_QUADS => Self::NQuads,
            FEXT_N_QUADS_STAR => Self::NQuadsStar,
            FEXT_N_TRIPLES => Self::NTriples,
            FEXT_N_TRIPLES_STAR => Self::NTriplesStar,
            // FEXT_RDF_A | FEXT_RDF_A_2 | FEXT_RDF_A_3 => Self::RdfA,
            FEXT_RDF_JSON => Self::RdfJson,
            FEXT_RDF_XML | FEXT_RDF_XML_2 | FEXT_RDF_XML_3 | FEXT_RDF_XML_4 => Self::RdfXml,
            FEXT_TRIG => Self::TriG,
            FEXT_TRIG_STAR => Self::TriGStar,
            FEXT_TRIX | FEXT_TRIX_2 => Self::TriX,
            FEXT_TSVW | FEXT_TSVW_2 => Self::Tsvw,
            FEXT_TURTLE => Self::Turtle,
            FEXT_TURTLE_STAR => Self::TurtleStar,
            FEXT_YAML_LD | FEXT_YAML_LD_2 => Self::YamlLd,
            _ => return Err(ParseError::UnrecognizedFileExtension(file_ext.to_string())),
        })
    }

    pub fn from_path(file: &StdPath) -> Result<Self, ParseError> {
        file.extension()
            .map(OsStr::to_string_lossy)
            .map(|fext| Self::from_file_ext(fext.as_ref()))
            .ok_or_else(|| ParseError::NoFileExtension(file.to_owned()))?
    }

    pub fn from_content(content: &[u8]) -> Result<Self, ParseError> {
        let infer_typ = infer::get(content).ok_or(ParseError::UnrecognizedContent)?;
        let media_typ = MediaType::parse(infer_typ.mime_type())
            .map_err(|_err| ParseError::UnrecognizedContent)?;
        Self::from_media_type(&media_typ)
    }

    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::BinaryRdf => MIME_TYPE_BINARY_RDF,
            Self::Csvw => MIME_TYPE_CSVW,
            Self::Hdt => MIME_TYPE_RDF_XML, // See <https://www.w3.org/submissions/2011/SUBM-HDT-20110330/#media>: "The media type of HDT is the media type of their parts. The Header SHOULD be represented in an RDF syntax. The normative format of the Header is [RDF/XML]"
            Self::HexTuples => MIME_TYPE_HEX_TUPLES,
            Self::Html => MIME_TYPE_HTML,
            Self::JsonLd => MIME_TYPE_JSON_LD,
            Self::Microdata => MIME_TYPE_MICRODATA,
            Self::N3 => MIME_TYPE_N3,
            Self::NdJsonLd => MIME_TYPE_ND_JSON_LD,
            Self::NQuads => MIME_TYPE_N_QUADS,
            Self::NQuadsStar => MIME_TYPE_N_QUADS_STAR,
            Self::NTriples => MIME_TYPE_N_TRIPLES,
            Self::NTriplesStar => MIME_TYPE_N_TRIPLES_STAR,
            Self::RdfA => MIME_TYPE_RDF_A,
            Self::RdfJson => MIME_TYPE_RDF_JSON,
            Self::RdfXml => MIME_TYPE_RDF_XML,
            Self::TriG => MIME_TYPE_TRIG,
            Self::TriGStar => MIME_TYPE_TRIG_STAR,
            Self::TriX => MIME_TYPE_TRIX,
            Self::Tsvw => MIME_TYPE_TSVW,
            Self::Turtle => MIME_TYPE_TURTLE,
            Self::TurtleStar => MIME_TYPE_TURTLE_STAR,
            Self::YamlLd => MIME_TYPE_YAML_LD,
        }
    }

    #[must_use]
    pub const fn media_type(self) -> MediaType<'static> {
        match self {
            Self::BinaryRdf => MEDIA_TYPE_BINARY_RDF,
            Self::Csvw => MEDIA_TYPE_CSVW,
            Self::Hdt => MEDIA_TYPE_RDF_XML, // See <https://www.w3.org/submissions/2011/SUBM-HDT-20110330/#media>: "The media type of HDT is the media type of their parts. The Header SHOULD be represented in an RDF syntax. The normative format of the Header is [RDF/XML]"
            Self::HexTuples => MEDIA_TYPE_HEX_TUPLES,
            Self::Html => MEDIA_TYPE_HTML,
            Self::JsonLd => MEDIA_TYPE_JSON_LD,
            Self::Microdata => MEDIA_TYPE_MICRODATA,
            Self::N3 => MEDIA_TYPE_N3,
            Self::NdJsonLd => MEDIA_TYPE_ND_JSON_LD,
            Self::NQuads => MEDIA_TYPE_N_QUADS,
            Self::NQuadsStar => MEDIA_TYPE_N_QUADS_STAR,
            Self::NTriples => MEDIA_TYPE_N_TRIPLES,
            Self::NTriplesStar => MEDIA_TYPE_N_TRIPLES_STAR,
            Self::RdfA => MEDIA_TYPE_RDF_A,
            Self::RdfJson => MEDIA_TYPE_RDF_JSON,
            Self::RdfXml => MEDIA_TYPE_RDF_XML,
            Self::TriG => MEDIA_TYPE_TRIG,
            Self::TriGStar => MEDIA_TYPE_TRIG_STAR,
            Self::TriX => MEDIA_TYPE_TRIX,
            Self::Tsvw => MEDIA_TYPE_TSVW,
            Self::Turtle => MEDIA_TYPE_TURTLE,
            Self::TurtleStar => MEDIA_TYPE_TURTLE_STAR,
            Self::YamlLd => MEDIA_TYPE_YAML_LD,
        }
    }

    #[must_use]
    pub const fn file_ext(self) -> &'static str {
        match self {
            Self::BinaryRdf => FEXT_BINARY_RDF,
            Self::Csvw => FEXT_CSVW,
            Self::Hdt => FEXT_HDT,
            Self::HexTuples => FEXT_HEX_TUPLES,
            Self::Html => FEXT_HTML,
            Self::JsonLd => FEXT_JSON_LD,
            Self::Microdata => FEXT_MICRODATA,
            Self::N3 => FEXT_N3,
            Self::NdJsonLd => FEXT_ND_JSON_LD,
            Self::NQuads => FEXT_N_QUADS,
            Self::NQuadsStar => FEXT_N_QUADS_STAR,
            Self::NTriples => FEXT_N_TRIPLES,
            Self::NTriplesStar => FEXT_N_TRIPLES_STAR,
            Self::RdfA => FEXT_RDF_A,
            Self::RdfJson => FEXT_RDF_JSON,
            Self::RdfXml => FEXT_RDF_XML,
            Self::TriG => FEXT_TRIG,
            Self::TriGStar => FEXT_TRIG_STAR,
            Self::TriX => FEXT_TRIX,
            Self::Tsvw => FEXT_TSVW,
            Self::Turtle => FEXT_TURTLE,
            Self::TurtleStar => FEXT_TURTLE_STAR,
            Self::YamlLd => FEXT_YAML_LD,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BinaryRdf => "BinaryRDF",
            Self::Csvw => "CSVW",
            Self::Hdt => "HDT",
            Self::HexTuples => "HexTuples",
            Self::Html => "HTML",
            Self::JsonLd => "JSON-LD",
            Self::Microdata => "Microdata",
            Self::N3 => "N3",
            Self::NdJsonLd => "NDJSON-LD",
            Self::NQuads => "N-Quads",
            Self::NQuadsStar => "N-Quads-star",
            Self::NTriples => "N-Triples",
            Self::NTriplesStar => "N-Triples-star",
            Self::RdfA => "RDFa",
            Self::RdfJson => "RDF/JSON",
            Self::RdfXml => "RDF/XML",
            Self::TriG => "TriG",
            Self::TriGStar => "TriG-star",
            Self::TriX => "TriX",
            Self::Tsvw => "TSVW",
            Self::Turtle => "Turtle",
            Self::TurtleStar => "Turtle-star",
            Self::YamlLd => "YAML-LD",
        }
    }

    #[must_use]
    pub const fn is_machine_readable(self) -> bool {
        match self {
            Self::Html => false,
            Self::BinaryRdf
            | Self::Csvw
            | Self::Hdt
            | Self::HexTuples
            | Self::JsonLd
            | Self::Microdata
            | Self::N3
            | Self::NdJsonLd
            | Self::NQuads
            | Self::NQuadsStar
            | Self::NTriples
            | Self::NTriplesStar
            | Self::RdfA
            | Self::RdfJson
            | Self::RdfXml
            | Self::TriG
            | Self::TriGStar
            | Self::TriX
            | Self::Tsvw
            | Self::Turtle
            | Self::TurtleStar
            | Self::YamlLd => true,
        }
    }

    #[must_use]
    pub const fn standard_definition_url(self) -> &'static str {
        match self {
            Self::BinaryRdf => todo!(), // TODO
            Self::Csvw => "https://w3c.github.io/csvw/syntax/",
            Self::Hdt => "https://www.rdfhdt.org/",
            Self::HexTuples => "https://github.com/ontola/hextuples",
            Self::Html => todo!(), // TODO
            Self::JsonLd => "http://www.w3.org/ns/formats/JSON-LD",
            Self::Microdata => "https://www.w3.org/wiki/Mapping_Microdata_to_RDF",
            Self::N3 => "http://www.w3.org/ns/formats/N3",
            Self::NdJsonLd => "https://github.com/json-ld/ndjson-ld",
            Self::NQuads => "http://www.w3.org/ns/formats/N-Quads",
            Self::NQuadsStar => {
                "https://w3c.github.io/rdf-star/cg-spec/editors_draft.html#n-quads-star"
            }
            Self::NTriples => "http://www.w3.org/ns/formats/N-Triples",
            Self::NTriplesStar => {
                "https://w3c.github.io/rdf-star/cg-spec/editors_draft.html#n-triples-star"
            }
            Self::RdfA => "https://www.w3.org/2001/sw/wiki/RDFa",
            Self::RdfJson => "http://www.w3.org/ns/formats/RDF_JSON",
            Self::RdfXml => "http://www.w3.org/ns/formats/RDF_XML",
            Self::TriG => "http://www.w3.org/ns/formats/TriG",
            Self::TriGStar => "https://w3c.github.io/rdf-star/cg-spec/editors_draft.html#trig-star",
            Self::TriX => todo!(), // TODO
            Self::Tsvw => "https://w3c.github.io/csvw/syntax/",
            Self::Turtle => "http://www.w3.org/ns/formats/Turtle",
            Self::TurtleStar => {
                "https://w3c.github.io/rdf-star/cg-spec/editors_draft.html#turtle-star"
            }
            Self::YamlLd => {
                "https://www.w3.org/community/reports/json-ld/CG-FINAL-yaml-ld-20231206/"
            }
        }
    }

    #[must_use]
    pub const fn star(self) -> bool {
        match self {
            Self::BinaryRdf | Self::NTriplesStar | Self::TriGStar | Self::TurtleStar => true,
            Self::Csvw
            | Self::Hdt
            | Self::HexTuples
            | Self::Html
            | Self::JsonLd
            | Self::Microdata
            | Self::N3
            | Self::NdJsonLd
            | Self::NQuads
            | Self::NQuadsStar
            | Self::NTriples
            | Self::RdfA
            | Self::RdfJson
            | Self::RdfXml
            | Self::TriG
            | Self::TriX
            | Self::Tsvw
            | Self::Turtle
            | Self::YamlLd => false,
        }
    }

    #[must_use]
    pub fn is_default(self) -> bool {
        self == Self::default()
    }

    // pub fn rdf_literal(self) -> Term {
    //     Term::Literal(Literal::new_simple_literal(self.mime_type)),
    // }
}
