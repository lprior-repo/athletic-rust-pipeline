use super::{acquisition::SOURCE_PARSER_REVISION, identity::scoped_key, protocol::ReviewOutcome};
use crate::domain::{
    identity::{AthleteId, EvidenceDigest, SourceRowKey, WorkbookDigest},
    name::CanonicalName,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ROW_PROTOCOL_REVISION: &str = "athlete-row-golden-context-v7";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowJob {
    pub workbook: WorkbookDigest,
    pub snapshot: EvidenceDigest,
    pub source: SourceRowKey,
}

impl RowJob {
    pub fn key(&self) -> anyhow::Result<String> {
        scoped_key(
            &self.snapshot,
            &(
                ROW_PROTOCOL_REVISION,
                SOURCE_PARSER_REVISION,
                &self.workbook,
                &self.source,
            ),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySummary {
    pub job: RowJob,
    pub candidate_ids: BTreeSet<AthleteId>,
    pub query_artifacts: Vec<EvidenceDigest>,
    pub complete: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CandidateCoverage {
    Complete {
        athlete_id: AthleteId,
        probe: EvidenceDigest,
        profile: EvidenceDigest,
    },
    Incomplete {
        athlete_id: AthleteId,
        probe: Option<EvidenceDigest>,
        profile: Option<EvidenceDigest>,
        issues: Vec<String>,
    },
    NameExcluded {
        athlete_id: AthleteId,
        probe: EvidenceDigest,
        source_name: CanonicalName,
    },
}

impl CandidateCoverage {
    #[must_use]
    pub fn athlete_id(&self) -> AthleteId {
        match self {
            Self::Complete { athlete_id, .. }
            | Self::Incomplete { athlete_id, .. }
            | Self::NameExcluded { athlete_id, .. } => *athlete_id,
        }
    }
    #[must_use]
    pub fn probe(&self) -> Option<&EvidenceDigest> {
        match self {
            Self::Complete { probe, .. } | Self::NameExcluded { probe, .. } => Some(probe),
            Self::Incomplete { probe, .. } => probe.as_ref(),
        }
    }
    #[must_use]
    pub fn profile(&self) -> Option<&EvidenceDigest> {
        match self {
            Self::Complete { profile, .. } => Some(profile),
            Self::Incomplete { profile, .. } => profile.as_ref(),
            Self::NameExcluded { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceMethod {
    Deterministic,
    LocalReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum RowResolution {
    Accepted {
        athlete_id: AthleteId,
        method: AcceptanceMethod,
    },
    CompleteSearchNoMatch,
    ReviewRequired,
}

/// Immutable result data; Restate, not this artifact, owns execution and recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowReport {
    pub revision: String,
    pub job: RowJob,
    pub resolution: RowResolution,
    pub discovery: Option<EvidenceDigest>,
    pub candidates: Vec<CandidateCoverage>,
    pub assessment: Option<EvidenceDigest>,
    pub query_evidence: Vec<EvidenceDigest>,
    pub review: Option<ReviewOutcome>,
    pub issues: Vec<String>,
}
