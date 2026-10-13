use super::RegionMembershipError;
use crate::model::{Gender, SchoolYear, Sport};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionKeyV1(String);

impl RegionKeyV1 {
    pub fn parse(value: &str) -> Result<Self, RegionMembershipError> {
        if value.is_empty() || value.len() > 96 {
            return Err(RegionMembershipError::InvalidField { field: "region" });
        }
        let qualified = value.split_once(':').is_some_and(|(authority, region)| {
            !authority.is_empty() && !region.is_empty() && !region.contains(':')
        });
        if !qualified
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b':')
            })
        {
            return Err(RegionMembershipError::InvalidField { field: "region" });
        }
        let mut retained = String::new();
        retained
            .try_reserve_exact(value.len())
            .map_err(|_| RegionMembershipError::AllocationFailed)?;
        retained.push_str(value);
        Ok(Self(retained))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for RegionKeyV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

struct KeyVisitor;

impl Visitor<'_> for KeyVisitor {
    type Value = RegionKeyV1;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded lowercase source-qualified region key")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        RegionKeyV1::parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for RegionKeyV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(KeyVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionContextV1 {
    pub sport: Sport,
    pub category: Gender,
    pub school_year: SchoolYear,
}

impl RegionContextV1 {
    pub(super) fn require_query_context(self) -> Result<(), RegionMembershipError> {
        if self.sport == Sport::Unknown {
            return Err(RegionMembershipError::InvalidField {
                field: "query.sport",
            });
        }
        if self.category == Gender::Unknown {
            return Err(RegionMembershipError::InvalidField {
                field: "query.category",
            });
        }
        Ok(())
    }
}
