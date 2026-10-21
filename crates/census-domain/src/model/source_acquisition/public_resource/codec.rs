use super::Wire;
use serde::de::{value::MapAccessDeserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::fmt;

pub(super) fn decode<'de, D>(deserializer: D) -> Result<Wire, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_map(ResourceVisitor)
}

struct ResourceVisitor;

impl<'de> Visitor<'de> for ResourceVisitor {
    type Value = Wire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a public source resource object")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        Wire::deserialize(MapAccessDeserializer::new(map))
    }
}
