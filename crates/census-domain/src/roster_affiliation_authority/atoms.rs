use super::{charge, size, Result, RosterAuthorityError};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RosterAuthorityTextV1(String);

impl RosterAuthorityTextV1 {
    pub fn parse(value: &str) -> Result<Self> {
        charge("text bytes", size(value.len())?, 4096)?;
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
            return Err(RosterAuthorityError::Invalid("text"));
        }
        let mut owned = String::new();
        owned.try_reserve_exact(value.len()).map_err(|_| RosterAuthorityError::Allocation)?;
        owned.push_str(value);
        Ok(Self(owned))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl<'de> Deserialize<'de> for RosterAuthorityTextV1 {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        struct Text;
        impl Visitor<'_> for Text {
            type Value = RosterAuthorityTextV1;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("nonempty bounded roster authority text")
            }
            fn visit_str<E: Error>(self, value: &str) -> std::result::Result<Self::Value, E> {
                RosterAuthorityTextV1::parse(value).map_err(E::custom)
            }
        }
        decoder.deserialize_str(Text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct RosterAuthorityDigestV1(String);

impl RosterAuthorityDigestV1 {
    pub fn parse(value: &str) -> Result<Self> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err(RosterAuthorityError::Invalid("SHA256"));
        }
        Ok(Self(RosterAuthorityTextV1::parse(value)?.0))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl<'de> Deserialize<'de> for RosterAuthorityDigestV1 {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> std::result::Result<Self, D::Error> {
        let value = RosterAuthorityTextV1::deserialize(decoder)?;
        Self::parse(value.as_str()).map_err(D::Error::custom)
    }
}
