use super::RegionEvidenceInputV1;
use crate::model::{Evidence, EvidenceMethod, SourceRef};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInput {
    id: Text<128>,
    #[serde(default)]
    url: Option<Text<2048>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceInput {
    source: SourceInput,
    method: EvidenceMethod,
    observed_on: Text<40>,
    #[serde(default)]
    note: Option<Text<4096>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvenanceInput {
    evidence: EvidenceInput,
    capture_sha256: Text<64>,
    locator: Text<512>,
    statement: Text<512>,
}

impl<'de> Deserialize<'de> for RegionEvidenceInputV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = ProvenanceInput::deserialize(deserializer)?;
        let evidence = Evidence {
            source: SourceRef::new(
                input.evidence.source.id.0,
                input.evidence.source.url.map(|url| url.0),
            ),
            method: input.evidence.method,
            observed_on: input.evidence.observed_on.0,
            note: input.evidence.note.map(|note| note.0),
        };
        Ok(Self {
            evidence,
            capture_sha256: input.capture_sha256.0,
            locator: input.locator.0,
            statement: input.statement.0,
        })
    }
}

struct Text<const MAX: usize>(String);
struct TextVisitor<const MAX: usize>;

impl<const MAX: usize> Visitor<'_> for TextVisitor<MAX> {
    type Value = Text<MAX>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a string of at most {MAX} bytes")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.len() > MAX {
            return Err(E::custom(
                "regional provenance string exceeds its byte ceiling",
            ));
        }
        let mut retained = String::new();
        retained
            .try_reserve_exact(value.len())
            .map_err(|_| E::custom("regional provenance allocation failed"))?;
        retained.push_str(value);
        Ok(Text(retained))
    }
}

impl<'de, const MAX: usize> Deserialize<'de> for Text<MAX> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(TextVisitor::<MAX>)
    }
}
