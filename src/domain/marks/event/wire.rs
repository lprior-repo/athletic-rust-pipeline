//! Serde wire format for `EventName`: the canonical name out, the retained
//! parse back in.

use super::EventName;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for EventName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for EventName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse_retained(&raw).map_err(serde::de::Error::custom)
    }
}
