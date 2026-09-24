//! Serde serialisation and deserialization for `UsJurisdiction`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::table::UsJurisdiction;

impl Serialize for UsJurisdiction {
    /// Serializes as the two-letter code, delegating to the [`std::fmt::Display`] impl the tests pin.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for UsJurisdiction {
    /// Deserialization is a domain boundary: an unknown or territorial code is a decode error, not
    /// a silently retained string.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "jurisdiction {raw:?} is not one of the 50 states or the District of Columbia"
            ))
        })
    }
}
