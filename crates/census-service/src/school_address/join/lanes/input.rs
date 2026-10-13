use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::super::JoinError;

mod bounded;
mod fields;
mod validation;

pub(crate) use fields::{EvidenceField, EvidenceFields};

pub(super) const MAX_EVIDENCE_RECORDS: usize = 256;
pub(super) const MAX_EVIDENCE_TEXT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct Acquisition {
    pub entry: u32,
    pub provider: String,
    pub path: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcquisitionWire {
    entry: u32,
    provider: bounded::BoundedText,
    path: bounded::BoundedText,
    url: bounded::BoundedText,
    #[serde(default)]
    retrieved_at: Option<bounded::BoundedText>,
}

impl<'de> Deserialize<'de> for Acquisition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AcquisitionWire::deserialize(deserializer)?;
        let value = Self {
            entry: wire.entry,
            provider: wire.provider.0,
            path: wire.path.0,
            url: wire.url.0,
            retrieved_at: wire.retrieved_at.map(|value| value.0),
        };
        validation::validate_acquisition(&value).map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

#[derive(Clone, Debug)]
pub struct Overrides {
    pub(super) input: EvidenceInput,
}

#[derive(Clone, Debug)]
pub(super) enum EvidenceInput {
    Acquisitions(Vec<Acquisition>),
    LegacySourcePairs(LegacySourcePairs),
}

#[derive(Clone, Debug)]
pub(super) struct LegacySourcePairs {
    pub(super) urls: BTreeMap<String, String>,
    pub(super) dates: BTreeMap<String, String>,
}

impl Default for Overrides {
    fn default() -> Self {
        Self {
            input: EvidenceInput::Acquisitions(Vec::new()),
        }
    }
}

impl Overrides {
    pub fn from_acquisitions(values: Vec<Acquisition>) -> Result<Self, JoinError> {
        Self {
            input: EvidenceInput::Acquisitions(values),
        }
        .validated()
    }

    pub fn from_source_pairs(
        urls: BTreeMap<String, String>,
        dates: BTreeMap<String, String>,
    ) -> Result<Self, JoinError> {
        Self {
            input: EvidenceInput::LegacySourcePairs(LegacySourcePairs { urls, dates }),
        }
        .validated()
    }

    pub fn from_cli_pairs(urls: &[String], dates: &[String]) -> Result<Self, JoinError> {
        Self::from_source_pairs(parse_source_pairs(urls)?, parse_source_pairs(dates)?)
    }

    pub fn acquisitions(&self) -> Option<&[Acquisition]> {
        match &self.input {
            EvidenceInput::Acquisitions(values) => Some(values),
            EvidenceInput::LegacySourcePairs(_) => None,
        }
    }

    pub fn validated(self) -> Result<Self, JoinError> {
        validation::validate_input(&self.input)?;
        Ok(self)
    }

    pub(crate) fn serialize_fields<M>(&self, map: &mut M) -> Result<(), M::Error>
    where
        M: SerializeMap,
    {
        match &self.input {
            EvidenceInput::Acquisitions(values) => map.serialize_entry("acquisitions", values),
            EvidenceInput::LegacySourcePairs(values) => {
                map.serialize_entry("urls", &values.urls)?;
                map.serialize_entry("dates", &values.dates)
            }
        }
    }
}

impl Serialize for Overrides {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        self.serialize_fields(&mut map)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Overrides {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(fields::OverridesVisitor)
    }
}

pub fn parse_source_pairs(values: &[String]) -> Result<BTreeMap<String, String>, JoinError> {
    validation::require_count("source-pairs", values.len())?;
    let mut pairs = BTreeMap::new();
    let mut budget = validation::TextBudget::default();
    for value in values {
        let Some((source, target)) = value.split_once('=') else {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        };
        if source.is_empty() || target.is_empty() {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        }
        budget.charge(source)?;
        budget.charge(target)?;
        validation::require_selector(source)?;
        if pairs.contains_key(source) {
            return Err(value_error("source-pairs", source, "duplicate selector"));
        }
        pairs.insert(bounded::copy_text(source)?, bounded::copy_text(target)?);
    }
    Ok(pairs)
}

pub(super) fn value_error(kind: &'static str, pair: &str, detail: &str) -> JoinError {
    JoinError::EvidenceValue {
        kind,
        pair: pair.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
