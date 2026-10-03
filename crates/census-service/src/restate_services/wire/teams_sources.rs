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
