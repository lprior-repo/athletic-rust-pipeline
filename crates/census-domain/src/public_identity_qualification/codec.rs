use super::{
    Declaration, IdentityAcceptance, IdentityProtocol, PublicIdentityQualificationV1,
    QualificationLifecycle,
};
use crate::model::StoreIdentity;
use serde::de::{value::MapAccessDeserializer, Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

impl<'de> Deserialize<'de> for PublicIdentityQualificationV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(DeclarationVisitor)
    }
}

struct DeclarationVisitor;

impl<'de> Visitor<'de> for DeclarationVisitor {
    type Value = PublicIdentityQualificationV1;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a public identity qualification declaration object")
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        Declaration::deserialize(MapAccessDeserializer::new(map)).map(PublicIdentityQualificationV1)
    }
}

pub(super) fn store_identity<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<StoreIdentity, D::Error> {
    deserializer.deserialize_str(IdentityVisitor)
}

struct IdentityVisitor;

impl Visitor<'_> for IdentityVisitor {
    type Value = StoreIdentity;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("exactly 64 ASCII hexadecimal bytes identifying a store")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        StoreIdentity::parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for QualificationLifecycle {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        fixed_text(
            deserializer,
            "fresh_qualification",
            Self::FreshQualification,
        )
    }
}

impl<'de> Deserialize<'de> for IdentityProtocol {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        fixed_text(
            deserializer,
            "public_identity_ledger_v1",
            Self::PublicIdentityLedgerV1,
        )
    }
}

impl<'de> Deserialize<'de> for IdentityAcceptance {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        fixed_text(
            deserializer,
            "source_bound_establishment_v1",
            Self::SourceBoundEstablishmentV1,
        )
    }
}

fn fixed_text<'de, D: Deserializer<'de>, T: Copy>(
    deserializer: D,
    expected: &'static str,
    accepted: T,
) -> Result<T, D::Error> {
    deserializer.deserialize_str(FixedText { expected, accepted })
}

struct FixedText<T> {
    expected: &'static str,
    accepted: T,
}

impl<T: Copy> Visitor<'_> for FixedText<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.expected)
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        if value == self.expected {
            Ok(self.accepted)
        } else {
            Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
        }
    }
}
