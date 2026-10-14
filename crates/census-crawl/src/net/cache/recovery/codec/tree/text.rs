use super::Budget;
use serde::de::{DeserializeSeed, Visitor};
use serde::Deserializer;
use std::fmt;

pub(super) fn copy<E: serde::de::Error>(
    text: &str,
    budget: &mut Budget<'_, '_>,
) -> Result<String, E> {
    budget.reserve(text.len())?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| E::custom("metadata string allocation failed"))?;
    owned.push_str(text);
    Ok(owned)
}

pub(super) struct Key<'a, 'io, 'budget>(pub(super) &'a mut Budget<'io, 'budget>);

impl<'de> DeserializeSeed<'de> for Key<'_, '_, '_> {
    type Value = String;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<String, D::Error> {
        decoder.deserialize_str(self)
    }
}

impl Visitor<'_> for Key<'_, '_, '_> {
    type Value = String;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON member name")
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
        copy(value, self.0)
    }
}
