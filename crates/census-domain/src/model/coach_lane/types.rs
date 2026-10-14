use serde::{Deserialize, Serialize};

use crate::jurisdiction::UsJurisdiction;
use crate::model::cohort::SchoolYear;
use crate::model::provenance::SourceRef;

pub const COACH_LANE_ID_PREFIX: &str = "coach_lane";
pub const COACH_LANE_MAX_ARM_BYTES: usize = 128;
pub const COACH_LANE_MAX_FRESHNESS_DAYS: u32 = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneStatus {
    Pending,
    Armed,
    AcceptedRefusal,
    Blocked,
    Failed,
    Exhausted,
    SuccessfulEmpty,
    CompleteWithGaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneStage {
    Pending,
    SuccessfulEmpty,
    CompleteWithGaps,
    Blocked,
    Failed,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneRole {
    Published,
    Derived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneTenure {
    Current,
    Former,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneMailbox {
    Published,
    Unpublished,
    DirectoryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneResolution {
    pub role: CoachLaneRole,
    pub tenure: CoachLaneTenure,
    pub mailbox: CoachLaneMailbox,
    pub retrieved_on: String,
    #[serde(default)]
    pub freshness_days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneRefusal {
    pub(super) source: SourceRef,
    pub(super) reason: String,
    pub(super) observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneAcceptance {
    pub(super) actor: String,
    pub(super) event: String,
    pub(super) observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneEvidence {
    pub(super) schools: u32,
    pub(super) contacts: u32,
    pub(super) digest: String,
    pub(super) observed_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCoachLane {
    pub jurisdiction: UsJurisdiction,
    pub season: SchoolYear,
    pub revision: u32,
    pub arm: String,
    pub arm_basis: String,
    pub observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoachLaneObligation {
    pub(super) id: String,
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) season: SchoolYear,
    pub(super) revision: u32,
    pub(super) arm: String,
    pub(super) arm_basis: String,
    pub(super) status: CoachLaneStatus,
    pub(super) refusal: Option<CoachLaneRefusal>,
    pub(super) acceptance: Option<CoachLaneAcceptance>,
    pub(super) evidence: Option<CoachLaneEvidence>,
    pub(super) resolution: CoachLaneResolution,
    pub(super) updated_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachLaneMatrix {
    pub(super) rows: Vec<CoachLaneObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoachLaneError {
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    #[error("{field} is not permitted in this state")]
    Unsupported { field: &'static str },
    #[error("an armed coach lane requires per-school output evidence")]
    MissingEvidence,
    #[error("an accepted refusal requires a recorded refusal first")]
    MissingRefusal,
    #[error("a recorded refusal cannot close a coach lane without an explicit acceptance event")]
    UnacceptedRefusal,
    #[error("jurisdiction {jurisdiction} has no coach-lane row")]
    MissingJurisdiction { jurisdiction: UsJurisdiction },
    #[error("jurisdiction {jurisdiction} has more than one coach-lane row")]
    DuplicateJurisdiction { jurisdiction: UsJurisdiction },
    #[error("jurisdiction {jurisdiction} is outside the census scope")]
    OutsideCensusScope { jurisdiction: UsJurisdiction },
    #[error("the coach lane for {jurisdiction} is not closed: {status}")]
    LaneNotClosed {
        jurisdiction: UsJurisdiction,
        status: CoachLaneStatus,
    },
}
