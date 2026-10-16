use super::{limit, EvidenceSubjectAttachmentError, ReplayLimits, Result};
use crate::participation_candidates::MAX_PARTICIPATION_FACTS;
use serde::de::{Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AttachmentTextV1(String);

impl AttachmentTextV1 {
    pub fn parse(value: &str) -> Result<Self> {
        limit("text bytes", value.len(), ReplayLimits::SMALL.text_bytes)?;
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(EvidenceSubjectAttachmentError::Invalid("text"));
        }
        let mut text = String::new();
        text.try_reserve_exact(value.len())
            .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
        text.push_str(value);
        Ok(Self(text))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AttachmentTextV1 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> std::result::Result<Self, D::Error> {
        struct Text;
        impl Visitor<'_> for Text {
            type Value = AttachmentTextV1;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded nonempty attachment text")
            }
            fn visit_str<E: Error>(self, value: &str) -> std::result::Result<Self::Value, E> {
                AttachmentTextV1::parse(value).map_err(E::custom)
            }
        }
        parser.deserialize_str(Text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AttachmentDigestV1(String);

impl AttachmentDigestV1 {
    pub fn parse(value: &str) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(EvidenceSubjectAttachmentError::Invalid("SHA256"));
        }
        Ok(Self(AttachmentTextV1::parse(value)?.0))
    }
    pub fn of(value: &(impl Serialize + ?Sized)) -> Result<Self> {
        Self::parse(&crate::model::serialized_digest(value)?)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AttachmentDigestV1 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> std::result::Result<Self, D::Error> {
        let text = AttachmentTextV1::deserialize(parser)?;
        Self::parse(text.as_str()).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AttachmentListV1<T>(Vec<T>);

impl<T> AttachmentListV1<T> {
    pub fn new(values: Vec<T>) -> Result<Self> {
        limit("references", values.len(), MAX_PARTICIPATION_FACTS)?;
        Ok(Self(values))
    }
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    pub fn into_values(self) -> Vec<T> {
        self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for AttachmentListV1<T> {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> std::result::Result<Self, D::Error> {
        struct List<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for List<T> {
            type Value = AttachmentListV1<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("at most 64 attachment references")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                for _ in 0..MAX_PARTICIPATION_FACTS {
                    let Some(value) = seq.next_element::<T>()? else {
                        return Ok(AttachmentListV1(values));
                    };
                    values.try_reserve_exact(1).map_err(A::Error::custom)?;
                    values.push(value);
                }
                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(A::Error::custom("attachment reference capacity"));
                }
                Ok(AttachmentListV1(values))
            }
        }
        parser.deserialize_seq(List(std::marker::PhantomData))
    }
}

pub(super) fn public_id<'de, D: Deserializer<'de>>(
    parser: D,
) -> std::result::Result<crate::public_identity_history::PublicAthleteId, D::Error> {
    let text = AttachmentTextV1::deserialize(parser)?;
    crate::public_identity_history::PublicAthleteId::parse(text.as_str()).map_err(D::Error::custom)
}

pub(super) fn candidate<'de, D: Deserializer<'de>>(
    parser: D,
) -> std::result::Result<crate::model::AthleteCandidateId, D::Error> {
    let text = AttachmentTextV1::deserialize(parser)?;
    crate::model::AthleteCandidateId::deserialize(
        serde::de::value::StrDeserializer::<D::Error>::new(text.as_str()),
    )
}

pub(super) fn store<'de, D: Deserializer<'de>>(
    parser: D,
) -> std::result::Result<crate::model::StoreIdentity, D::Error> {
    let text = AttachmentTextV1::deserialize(parser)?;
    crate::model::StoreIdentity::parse(text.as_str()).map_err(D::Error::custom)
}
