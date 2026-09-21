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
    Rankings {
        collection: EvidenceDigest,
        list_id: u64,
        gender: String,
        grade: Option<u8>,
        event_short: String,
        page: u32,
        capture: RankingsCapture,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentReceipt {
    pub digest: EvidenceDigest,
    pub source_url: String,
    pub http_status: u16,
    pub media_type: String,
    pub bytes: u64,
    pub fetched_at_unix_ms: u64,
    pub elapsed_ms: u64,
    pub rankings: Option<RankingPageObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RankingsCapture {
    Navigation,
    Results,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingPageObservation {
    pub capture: RankingsCapture,
    pub request_method: String,
    pub request_url: String,
    pub request_body: Option<String>,
    pub next_page: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    InvalidInput,
    Transport,
    AccessDenied,
    BrowserChallenge,
    BrowserUnavailable,
    RateLimited,
    HttpFailure,
    PayloadLimit,
    ArtifactFailure,
    MalformedResponse,
    RetryExhausted,
    UncertainEffect,
}

/// Restate owns scheduling. Source workflows and model SDK policies have distinct
/// retry budgets. Unacknowledged effects may repeat during crash recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    WorkflowControlled {
        operation: EvidenceDigest,
        maximum_retries: RetryCount,
        observed_attempts: u32,
        attempts: Vec<EvidenceDigest>,
    },
    WorkflowEvidenceUnavailable {
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
        response: Box<DocumentReceipt>,
        previous_responses: Vec<DocumentReceipt>,
        retries: RetryEvidence,
    },
    Failed {
        lane: ModelLane,
        request: Option<EvidenceDigest>,
        failure: OperationFailure,
    },
}
