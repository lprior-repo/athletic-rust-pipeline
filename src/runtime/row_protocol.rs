use super::{identity::scoped_key, protocol::ReviewOutcome};
use crate::domain::identity::{AthleteId, EvidenceDigest, SourceRowKey, WorkbookDigest};
use serde::{Deserialize, Serialize};

pub const ROW_PROTOCOL_REVISION: &str = "athlete-row-golden-context-v4";

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
                super::acquisition::SOURCE_PARSER_REVISION,
                &self.workbook,
                &self.source,
            ),
        )
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
    pub assessment: Option<EvidenceDigest>,
    pub query_evidence: Vec<EvidenceDigest>,
    pub profile_evidence: Vec<EvidenceDigest>,
    pub review: Option<ReviewOutcome>,
    pub issues: Vec<String>,
}
