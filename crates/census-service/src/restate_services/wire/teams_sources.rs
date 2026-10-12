use census_domain::model::{CoachLaneError, CoachLaneRefusal, CoachLaneStage, SourceRef};
use serde::{Deserialize, Serialize};

use super::{JurisdictionRequest, StageOutcome};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsSourceRequest {
    pub jurisdiction: JurisdictionRequest,
    pub source: String,
    pub observed_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TeamsSourceInspection {
    Settled { outcome: TeamsSourceOutcome },
    Unsettled { progress: Vec<TeamsAttemptProgress> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TeamsSourceOutcome {
    Completed {
        outcome: StageOutcome,
        progress: Vec<TeamsAttemptProgress>,
    },
    Terminal {
        message: String,
        progress: Vec<TeamsAttemptProgress>,
    },
    Exhausted {
        attempts: u8,
        last_failure: String,
        progress: Vec<TeamsAttemptProgress>,
    },
    Interrupted {
        attempts: Option<u8>,
        message: String,
        progress: Vec<TeamsAttemptProgress>,
    },
}

impl TeamsSourceOutcome {
    pub fn coach_lane_stage(&self) -> CoachLaneStage {
        match self {
            Self::Completed { outcome, .. } if outcome.records == 0 => {
                CoachLaneStage::SuccessfulEmpty
            }
            Self::Completed { .. } => CoachLaneStage::CompleteWithGaps,
            Self::Terminal { .. } | Self::Interrupted { .. } => CoachLaneStage::Blocked,
            Self::Exhausted { .. } => CoachLaneStage::Exhausted,
        }
    }

    pub fn coach_lane_refusal(
        &self,
        source: &str,
        observed_on: &str,
    ) -> Result<Option<CoachLaneRefusal>, CoachLaneError> {
        let reason = match self {
            Self::Completed { outcome, .. } => Some(format!(
                "source run retained {} records without qualified coach role evidence",
                outcome.records
            )),
            Self::Terminal { message, .. } => {
                non_empty(message).map(|message| format!("source terminal: {message}"))
            }
            Self::Exhausted {
                attempts,
                last_failure,
                ..
            } => Some(match non_empty(last_failure) {
                Some(failure) => format!("retries exhausted after {attempts} attempts: {failure}"),
                None => format!("retries exhausted after {attempts} attempts"),
            }),
            Self::Interrupted { message, .. } => {
                non_empty(message).map(|message| format!("source run interrupted: {message}"))
            }
        };
        reason
            .map(|reason| CoachLaneRefusal::new(SourceRef::id(source), reason, observed_on))
            .transpose()
    }
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TeamsAttemptProgress {
    Unknown {
        attempt: u8,
    },
    Completed {
        attempt: u8,
        outcome: StageOutcome,
    },
    Transient {
        attempt: u8,
        outcome: Option<StageOutcome>,
        message: String,
    },
    Terminal {
        attempt: u8,
        outcome: Option<StageOutcome>,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsSourceFailure {
    pub source: String,
    pub outcome: TeamsSourceOutcome,
}
