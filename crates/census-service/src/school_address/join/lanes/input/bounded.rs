use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Error, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::{value_error, Acquisition, JoinError, MAX_EVIDENCE_RECORDS, MAX_EVIDENCE_TEXT_BYTES};

pub(super) struct BoundedText(pub(super) String);

pub(super) fn copy_text(value: &str) -> Result<String, JoinError> {
    if value.len() > MAX_EVIDENCE_TEXT_BYTES {
        return Err(value_error(
            "bytes",
            "evidence field",
            "field exceeds 4194304 text bytes",
        ));
    }
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|error| value_error("allocation", "evidence field", &error.to_string()))?;
    owned.push_str(value);
    Ok(owned)
}

struct TextVisitor;

impl<'de> Visitor<'de> for TextVisitor {
    type Value = BoundedText;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string of at most 4194304 UTF-8 bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        copy_text(value).map(BoundedText).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if value.len() > MAX_EVIDENCE_TEXT_BYTES {
            return Err(E::custom("field exceeds 4194304 text bytes"));
        }
        Ok(BoundedText(value))
    }
}

impl<'de> Deserialize<'de> for BoundedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(TextVisitor)
    }
}

pub(super) struct AcquisitionsField(pub(super) Vec<Acquisition>);

struct AcquisitionsVisitor;

impl<'de> Visitor<'de> for AcquisitionsVisitor {
    type Value = AcquisitionsField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an array containing at most 256 acquisition records")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        let mut budget = super::validation::TextBudget::default();
        for _ in 0..MAX_EVIDENCE_RECORDS {
            let Some(value) = sequence.next_element::<Acquisition>()? else {
                return Ok(AcquisitionsField(values));
            };
            budget.charge(&value.provider).map_err(A::Error::custom)?;
            budget.charge(&value.path).map_err(A::Error::custom)?;
            budget.charge(&value.url).map_err(A::Error::custom)?;
            if let Some(date) = &value.retrieved_at {
                budget.charge(date).map_err(A::Error::custom)?;
            }
            values.try_reserve(1).map_err(A::Error::custom)?;
            values.push(value);
        }
        if sequence.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::custom("expected at most 256 acquisition records"));
        }
        Ok(AcquisitionsField(values))
    }
}

impl<'de> Deserialize<'de> for AcquisitionsField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(AcquisitionsVisitor)
    }
}

pub(super) struct SourcePairsField(pub(super) BTreeMap<String, String>);

struct SourcePairsVisitor;

impl<'de> Visitor<'de> for SourcePairsVisitor {
    type Value = SourcePairsField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object containing at most 256 unique source selectors")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        let mut budget = super::validation::TextBudget::default();
        for _ in 0..MAX_EVIDENCE_RECORDS {
            let Some(key) = map.next_key::<BoundedText>()? else {
                return Ok(SourcePairsField(values));
            };
            if values.contains_key(&key.0) {
                return Err(A::Error::custom(format!(
                    "duplicate source selector {:?}",
                    key.0
                )));
            }
            budget.charge(&key.0).map_err(A::Error::custom)?;
            super::validation::require_selector(&key.0).map_err(A::Error::custom)?;
            let value = map.next_value::<BoundedText>()?;
            budget.charge(&value.0).map_err(A::Error::custom)?;
            values.insert(key.0, value.0);
        }
        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(A::Error::custom("expected at most 256 source selectors"));
        }
        Ok(SourcePairsField(values))
    }
}

impl<'de> Deserialize<'de> for SourcePairsField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SourcePairsVisitor)
    }
}
