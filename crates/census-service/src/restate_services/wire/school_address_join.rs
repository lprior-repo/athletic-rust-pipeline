use census_domain::national_census::SchoolAddressJoinReply;
use serde::de::{Error as _, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::school_address::{EvidenceField, EvidenceFields, JoinError, JoinReport, Overrides};

const FIELD_NAMES: [&str; 5] = [
    "generation",
    "urls",
    "dates",
    "acquisitions",
    "expected_digest",
];

#[derive(Debug, Clone, Default)]
pub struct SchoolAddressJoinRequest {
    pub generation: Option<String>,
    pub overrides: Overrides,
    pub expected_digest: Option<String>,
}

impl Serialize for SchoolAddressJoinRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("generation", &self.generation)?;
        self.overrides.serialize_fields(&mut map)?;
        if let Some(expected_digest) = &self.expected_digest {
            map.serialize_entry("expected_digest", expected_digest)?;
        }
        map.end()
    }
}

#[derive(Default)]
struct RequestFields {
    generation: Option<Option<String>>,
    evidence: EvidenceFields,
    expected_digest: Option<Option<String>>,
}

impl RequestFields {
    fn read_field<'de, A: MapAccess<'de>>(
        &mut self,
        key: &str,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match key {
            "generation" => read_optional_string(&mut self.generation, "generation", map),
            "expected_digest" => {
                read_optional_string(&mut self.expected_digest, "expected_digest", map)
            }
            _ => match EvidenceField::parse(key) {
                Some(field) => self.evidence.read_field(field, map),
                None => Err(A::Error::unknown_field(key, &FIELD_NAMES)),
            },
        }
    }

    fn finish(self) -> Result<SchoolAddressJoinRequest, JoinError> {
        Ok(SchoolAddressJoinRequest {
            generation: self.generation.flatten(),
            overrides: self.evidence.finish()?,
            expected_digest: self.expected_digest.flatten(),
        })
    }
}

fn read_optional_string<'de, A: MapAccess<'de>>(
    value: &mut Option<Option<String>>,
    field: &'static str,
    map: &mut A,
) -> Result<(), A::Error> {
    if value.is_some() {
        return Err(A::Error::duplicate_field(field));
    }
    *value = Some(map.next_value()?);
    Ok(())
}

struct RequestVisitor;

impl<'de> Visitor<'de> for RequestVisitor {
    type Value = SchoolAddressJoinRequest;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a school address join request")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = RequestFields::default();
        for _ in 0..FIELD_NAMES.len() {
            let Some(key) = EvidenceFields::next_key(&mut map)? else {
                return fields.finish().map_err(A::Error::custom);
            };
            fields.read_field(&key, &mut map)?;
        }
        if EvidenceFields::next_key(&mut map)?.is_some() {
            return Err(A::Error::custom(
                "school address join request accepts at most five fields",
            ));
        }
        fields.finish().map_err(A::Error::custom)
    }
}

impl<'de> Deserialize<'de> for SchoolAddressJoinRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(RequestVisitor)
    }
}

impl From<JoinReport> for SchoolAddressJoinReply {
    fn from(report: JoinReport) -> Self {
        Self {
            mode: report.mode,
            generation: report.generation,
            report: report.report,
            outcomes: report.outcomes,
            counters: report.counters,
        }
    }
}

#[cfg(test)]
#[path = "school_address_join_tests.rs"]
mod tests;
