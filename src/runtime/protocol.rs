use super::ModelLane;
use crate::{
    domain::{
        evidence::{EvidenceIssue, EvidenceRef, Observed, Sport, SportAvailability, TeamEvidence},
        facts::{AthleteName, GraduationYear, RetryCount},
        identity::{AthleteId, EvidenceDigest, ProfileUrl},
    },
    model::SourceRecord,
};
use serde::{Deserialize, Serialize};

pub const MAX_REVIEW_INPUT_BYTES: usize = 65_536;
pub const MAX_REVIEW_RESPONSE_BYTES: usize = 32_768;
pub const MAX_SOURCE_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SourceResource {
    Search {
        query: String,
        sport: Sport,
        start: u32,
    },
    Bio {
        athlete_id: AthleteId,
        sport: Sport,
    },
    ProfileHtml {
        profile_url: ProfileUrl,
    },
    Team {
        team_id: u64,
        sport: Sport,
        season: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentReceipt {
    pub digest: EvidenceDigest,
    pub source_url: String,
    pub http_status: u16,
    pub media_type: String,
    pub bytes: u64,
    pub fetched_at_unix_ms: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    InvalidInput,
    Transport,
    AccessDenied,
    RateLimited,
    HttpFailure,
    PayloadLimit,
    ArtifactFailure,
    MalformedResponse,
    RetryExhausted,
    UncertainEffect,
}

/// Restate owns scheduling. Observed effects are not an exact SDK retry counter:
/// an unacknowledged effect may repeat during recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "ownership")]
pub enum RetryEvidence {
    NotAttempted,
    SdkControlled {
        operation: EvidenceDigest,
        maximum_retries: RetryCount,
        observed_attempts: u32,
        attempts: Vec<EvidenceDigest>,
    },
    SdkEvidenceUnavailable {
        operation: EvidenceDigest,
        maximum_retries: RetryCount,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationFailure {
    pub code: FailureCode,
    pub message: String,
    pub http_status: Option<u16>,
    pub retries: RetryEvidence,
    pub evidence: Vec<DocumentReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum FetchOutcome {
    Retrieved {
        receipt: DocumentReceipt,
        retries: RetryEvidence,
        previous_responses: Vec<DocumentReceipt>,
    },
    Failed {
        failure: OperationFailure,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewCandidate {
    pub athlete_id: AthleteId,
    pub name: AthleteName,
    pub teams: Vec<TeamEvidence>,
    pub graduation_years: Vec<Observed<GraduationYear>>,
    pub sports: Vec<SportAvailability>,
    pub issues: Vec<EvidenceIssue>,
    pub eligibility_reasons: Vec<String>,
    pub documents: Vec<EvidenceDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewInput {
    pub source: SourceRecord,
    pub candidates: Vec<ReviewCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewJob {
    pub input: EvidenceDigest,
    pub lane: ModelLane,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum ReviewVerdict {
    Select {
        athlete_id: AthleteId,
        reason: String,
        evidence: Vec<EvidenceRef>,
    },
    Unresolved {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum ReviewOutcome {
    Reviewed {
        lane: ModelLane,
        verdict: ReviewVerdict,
        request: EvidenceDigest,
        response: DocumentReceipt,
        previous_responses: Vec<DocumentReceipt>,
        retries: RetryEvidence,
    },
    Failed {
        lane: ModelLane,
        request: Option<EvidenceDigest>,
        failure: OperationFailure,
    },
}
