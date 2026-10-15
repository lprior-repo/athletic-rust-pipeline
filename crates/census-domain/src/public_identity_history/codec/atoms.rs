use std::fmt;

use crate::model::{AthleteCandidateId, StoreIdentity};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use super::super::{record, CandidateEvidenceKind, PublicAthleteId};
use super::containers::{copy_text, Meter, Node, NodeSeed};

pub(super) struct TextNode;
pub(super) struct CandidateNode;
pub(super) struct PublicIdNode;
pub(super) struct InstallationNode;
pub(super) struct DigestNode;
pub(super) struct EvidenceDigestNode;
pub(super) struct NumberNode;
pub(super) struct VersionNode;
pub(super) struct EvidenceKindNode;
pub(super) struct OptionalTextNode;

pub(super) trait Text {
    type Value;

    fn parse<E: Error>(meter: &mut Meter, value: &str) -> Result<Self::Value, E>;
}

impl Node for TextNode {
    type Value = String;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl Node for CandidateNode {
    type Value = AthleteCandidateId;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl Node for PublicIdNode {
    type Value = PublicAthleteId;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl Node for InstallationNode {
    type Value = StoreIdentity;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl Node for DigestNode {
    type Value = String;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl Node for EvidenceDigestNode {
    type Value = String;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(NodeSeed::<Self>::new(meter))
    }
}

impl<'de, N: Text> Visitor<'de> for NodeSeed<'_, N> {
    type Value = N::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("validated bounded history text")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        N::parse(self.meter, value)
    }
}

impl Text for TextNode {
    type Value = String;

    fn parse<E: Error>(meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        meter.text(value)
    }
}

impl Text for CandidateNode {
    type Value = AthleteCandidateId;

    fn parse<E: Error>(meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        let value = meter.text(value)?;
        AthleteCandidateId::deserialize(serde::de::value::StringDeserializer::<E>::new(value))
    }
}

impl Text for PublicIdNode {
    type Value = PublicAthleteId;

    fn parse<E: Error>(meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        meter.charge(value)?;
        PublicAthleteId::parse(value).map_err(E::custom)
    }
}

impl Text for InstallationNode {
    type Value = StoreIdentity;

    fn parse<E: Error>(_meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        StoreIdentity::parse(value).map_err(E::custom)
    }
}

impl Text for DigestNode {
    type Value = String;

    fn parse<E: Error>(_meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        if !record::is_digest(value) {
            return Err(E::custom("invalid history digest"));
        }
        copy_text(value)
    }
}

impl Text for EvidenceDigestNode {
    type Value = String;

    fn parse<E: Error>(meter: &mut Meter, value: &str) -> Result<Self::Value, E> {
        if !record::is_digest(value) {
            return Err(E::custom("invalid history evidence digest"));
        }
        meter.text(value)
    }
}

impl Node for NumberNode {
    type Value = u64;

    fn decode<'de, D: Deserializer<'de>>(
        _meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        u64::deserialize(deserializer)
    }
}

impl Node for VersionNode {
    type Value = u32;

    fn decode<'de, D: Deserializer<'de>>(
        _meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let version = u32::deserialize(deserializer)?;
        if version != record::HISTORY_VERSION {
            return Err(D::Error::custom("unsupported history version"));
        }
        Ok(version)
    }
}

impl Node for EvidenceKindNode {
    type Value = CandidateEvidenceKind;

    fn decode<'de, D: Deserializer<'de>>(
        _meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        CandidateEvidenceKind::deserialize(deserializer)
    }
}

impl Node for OptionalTextNode {
    type Value = Option<String>;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_option(NodeSeed::<Self>::new(meter))
    }
}

impl<'de> Visitor<'de> for NodeSeed<'_, OptionalTextNode> {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("null or bounded superseded binding text")
    }

    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        TextNode::decode(self.meter, deserializer).map(Some)
    }
}
