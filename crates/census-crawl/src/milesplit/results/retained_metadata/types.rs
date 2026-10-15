use crate::milesplit::{RawPage, ResultSetRef, VerifiedRetainedOwnedResult};
use crate::net::FetchOutcome;
use census_domain::model::{ProfileDate, StoreIdentity};
use chrono::{DateTime, Utc};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMetadataArchiveRef(String);

impl RawMetadataArchiveRef {
    pub fn parse(key: &str) -> Result<Self, super::RetainedMetadataError> {
        let digest = key
            .strip_prefix("metadata/")
            .and_then(|value| value.strip_suffix("/manifest"));
        if !digest.is_some_and(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Err(super::RetainedMetadataError::InvalidArchiveReference);
        }
        Ok(Self(key.to_owned()))
    }

    pub fn manifest_key(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainedPerformanceDate {
    Exact(ProfileDate),
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedResultDivision {
    HighSchool,
    MiddleSchool,
    Collegiate,
    PostGraduation,
    Club,
    Open,
    Unattached,
    Unknown,
}

#[derive(Debug)]
pub struct VerifiedRetainedResultMetadata {
    pub(super) store_identity: StoreIdentity,
    pub(super) snapshot_sequence: u64,
    pub(super) parsed: Arc<ParsedMetadata>,
    pub(super) result: VerifiedRetainedOwnedResult,
    pub(super) division: RetainedResultDivision,
}

#[derive(Debug)]
pub(super) struct ParsedMetadata {
    pub(super) reference: ResultSetRef,
    pub(super) archive: RawMetadataArchiveRef,
    pub(super) capture: FetchOutcome,
    pub(super) acquired_at: DateTime<Utc>,
    pub(super) page: RawPage,
    pub(super) date: RetainedPerformanceDate,
}

impl VerifiedRetainedResultMetadata {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }
    pub fn archive(&self) -> &RawMetadataArchiveRef {
        &self.parsed.archive
    }
    pub fn capture(&self) -> &FetchOutcome {
        &self.parsed.capture
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.parsed.acquired_at
    }
    pub fn page(&self) -> &RawPage {
        &self.parsed.page
    }
    pub fn result(&self) -> &VerifiedRetainedOwnedResult {
        &self.result
    }
    pub fn date(&self) -> &RetainedPerformanceDate {
        &self.parsed.date
    }
    pub fn division(&self) -> RetainedResultDivision {
        self.division
    }
}
