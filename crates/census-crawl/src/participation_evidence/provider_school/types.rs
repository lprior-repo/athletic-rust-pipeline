use super::super::invalid;
use crate::milesplit::{RosterVerdict, SourceRowLocator};
use crate::CollectionDisposition;
use census_domain::model::{CanonicalSchool, SourceSchoolObservation};
use census_store::StoreResult;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ProviderSchoolPacketRefV1(String);

impl ProviderSchoolPacketRefV1 {
    pub fn parse(value: &str) -> StoreResult<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid("invalid provider school packet reference"));
        }
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSchoolDirectoryProofV1 {
    pub(super) capture_sha256: String,
    pub(super) acquired_at: DateTime<Utc>,
    pub(super) disposition: CollectionDisposition,
    pub(super) parsed_teams: usize,
    pub(super) errors: usize,
    pub(super) unfinished: Vec<String>,
    pub(super) row: SourceRowLocator,
    pub(super) row_sha256: String,
    pub(super) name: SourceRowLocator,
    pub(super) city: SourceRowLocator,
    pub(super) anchor: SourceRowLocator,
}

impl ProviderSchoolDirectoryProofV1 {
    pub fn capture_sha256(&self) -> &str {
        &self.capture_sha256
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
    pub fn disposition(&self) -> CollectionDisposition {
        self.disposition
    }
    pub fn parsed_teams(&self) -> usize {
        self.parsed_teams
    }
    pub fn errors(&self) -> usize {
        self.errors
    }
    pub fn unfinished(&self) -> &[String] {
        &self.unfinished
    }
    pub fn row(&self) -> SourceRowLocator {
        self.row
    }
    pub fn row_sha256(&self) -> &str {
        &self.row_sha256
    }
    pub fn name(&self) -> SourceRowLocator {
        self.name
    }
    pub fn city(&self) -> SourceRowLocator {
        self.city
    }
    pub fn anchor(&self) -> SourceRowLocator {
        self.anchor
    }
}

#[derive(Debug)]
pub struct CheckedProviderSchoolPacketV1 {
    pub(super) reference: ProviderSchoolPacketRefV1,
    pub(super) school: CanonicalSchool,
    pub(super) observation: SourceSchoolObservation,
    pub(super) roster_verdict: RosterVerdict,
    pub(super) directory_proof: ProviderSchoolDirectoryProofV1,
}

impl CheckedProviderSchoolPacketV1 {
    pub fn reference(&self) -> &ProviderSchoolPacketRefV1 {
        &self.reference
    }
    pub fn school(&self) -> &CanonicalSchool {
        &self.school
    }
    pub fn observation(&self) -> &SourceSchoolObservation {
        &self.observation
    }
    pub fn roster_verdict(&self) -> &RosterVerdict {
        &self.roster_verdict
    }
    pub fn directory_proof(&self) -> &ProviderSchoolDirectoryProofV1 {
        &self.directory_proof
    }
}
