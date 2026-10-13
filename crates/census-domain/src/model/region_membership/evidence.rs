use super::RegionMembershipError;
use crate::model::{Evidence, EvidenceMethod};
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;

mod wire;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionEvidenceInputV1 {
    pub evidence: Evidence,
    pub capture_sha256: String,
    pub locator: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RegionEvidenceV1(RegionEvidenceInputV1);

impl RegionEvidenceV1 {
    pub fn new(input: RegionEvidenceInputV1) -> Result<Self, RegionMembershipError> {
        super::boundary::validate_evidence(&input)?;
        Ok(Self(input))
    }

    pub fn input(&self) -> &RegionEvidenceInputV1 {
        &self.0
    }

    pub fn evidence(&self) -> &Evidence {
        &self.0.evidence
    }

    pub fn capture_sha256(&self) -> &str {
        &self.0.capture_sha256
    }

    pub fn locator(&self) -> &str {
        &self.0.locator
    }

    pub fn statement(&self) -> &str {
        &self.0.statement
    }
}

impl Ord for RegionEvidenceV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = self.input();
        let right = other.input();
        (
            &left.evidence.source,
            method_key(left.evidence.method),
            &left.evidence.observed_on,
            &left.evidence.note,
            &left.capture_sha256,
            &left.locator,
            &left.statement,
        )
            .cmp(&(
                &right.evidence.source,
                method_key(right.evidence.method),
                &right.evidence.observed_on,
                &right.evidence.note,
                &right.capture_sha256,
                &right.locator,
                &right.statement,
            ))
    }
}

impl PartialOrd for RegionEvidenceV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn method_key(method: EvidenceMethod) -> u8 {
    match method {
        EvidenceMethod::Fetched => 0,
        EvidenceMethod::Parsed => 1,
        EvidenceMethod::Derived => 2,
    }
}

impl<'de> Deserialize<'de> for RegionEvidenceV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(RegionEvidenceInputV1::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}
