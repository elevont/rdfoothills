// SPDX-FileCopyrightText: 2024 Robin Vobruba <hoijui.quaero@gmail.com>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "async")]
use async_trait::async_trait;
use oxrdfio::{RdfFormat, RdfParseError, RdfParser, RdfSerializer};
#[cfg(feature = "async")]
use tokio::fs;

use super::OntFile;
use rdfoothills_mime as mime;

#[derive(Debug, Default)]
pub struct Converter;

impl Converter {
    const fn to_oxrdf_format(fmt: mime::Type) -> Option<RdfFormat> {
        match fmt {
            mime::Type::N3 => Some(RdfFormat::N3),
            mime::Type::NQuads | mime::Type::NQuadsStar => Some(RdfFormat::NQuads),
            mime::Type::NTriples | mime::Type::NTriplesStar => Some(RdfFormat::NTriples),
            mime::Type::OwlXml | mime::Type::RdfXml => Some(RdfFormat::RdfXml),
            mime::Type::TriG | mime::Type::TriGStar => Some(RdfFormat::TriG),
            mime::Type::Turtle | mime::Type::TurtleStar => Some(RdfFormat::Turtle),
            mime::Type::BinaryRdf
            | mime::Type::Csvw
            | mime::Type::Hdt
            | mime::Type::HexTuples
            | mime::Type::Html
            | mime::Type::JsonLd
            | mime::Type::Microdata
            | mime::Type::NdJsonLd
            | mime::Type::OwlFunctional
            | mime::Type::RdfA
            | mime::Type::RdfJson
            | mime::Type::TriX
            | mime::Type::Tsvw
            | mime::Type::YamlLd => None,
        }
    }

    const fn supports_format(fmt: mime::Type) -> bool {
        Self::to_oxrdf_format(fmt).is_some()
    }
}

fn map_rdf_parse_error(parse_err: RdfParseError) -> super::Error {
    match parse_err {
        RdfParseError::Io(io_err) => super::Error::Io(io_err),
        RdfParseError::Syntax(syntax_err) => super::Error::Syntax(syntax_err.to_string()),
    }
}

#[cfg_attr(feature = "async", async_trait)]
impl super::Converter for Converter {
    fn info(&self) -> super::Info {
        super::Info {
            quality: super::Quality::Data,
            priority: super::Priority::High,
            typ: super::Type::Native,
            name: "OxRDF I/O",
        }
    }

    fn is_available(&self) -> bool {
        true
    }

    fn supports(&self, from: mime::Type, to: mime::Type) -> bool {
        Self::supports_format(from) && Self::supports_format(to)
    }

    fn convert(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        let from_fmt = Self::to_oxrdf_format(from.mime_type)
            .expect("convert called with an invalid (-> unsupported by OxRDF) input format");
        let to_fmt = Self::to_oxrdf_format(to.mime_type)
            .expect("convert called with an invalid (-> unsupported by OxRDF) output format");

        let in_file = std::fs::File::open(&from.file);
        let reader = RdfParser::from_format(from_fmt).parse_read(in_file.unwrap());
        let out_file = std::fs::File::create(&to.file);
        let mut writer = RdfSerializer::from_format(to_fmt).serialize_to_write(out_file.unwrap());
        for quad_res in reader {
            let quad = quad_res.map_err(map_rdf_parse_error)?;
            writer.write_quad(&quad)?;
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn convert_async(&self, from: &OntFile, to: &OntFile) -> Result<(), super::Error> {
        let from_fmt = Self::to_oxrdf_format(from.mime_type)
            .expect("convert called with an invalid (-> unsupported by OxRDF) input format");
        let to_fmt = Self::to_oxrdf_format(to.mime_type)
            .expect("convert called with an invalid (-> unsupported by OxRDF) output format");

        let in_file = fs::File::open(&from.file).await;
        let mut reader = RdfParser::from_format(from_fmt).parse_tokio_async_read(in_file.unwrap());
        let out_file = fs::File::create(&to.file).await;
        let mut writer =
            RdfSerializer::from_format(to_fmt).serialize_to_tokio_async_write(out_file.unwrap());
        while let Some(quad_res) = reader.next().await {
            let quad = quad_res.map_err(map_rdf_parse_error)?;
            writer.write_quad(&quad).await?;
        }

        Ok(())
    }
}
