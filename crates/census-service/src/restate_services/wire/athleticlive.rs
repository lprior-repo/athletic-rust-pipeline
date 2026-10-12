use super::{JurisdictionRequest, StageOutcome};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AthleticLiveObligation {
    Refused {
        reason: String,
    },
    Armed {
        digest: String,
    },
    Running {
        digest: String,
        attempt: u8,
    },
    Finished {
        digest: String,
        attempt: u8,
        outcome: StageOutcome,
    },
}

impl AthleticLiveObligation {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Finished { outcome, .. } if outcome.disposition.is_complete() && outcome.errors.is_empty() && outcome.unfinished.is_empty())
    }

    pub fn reason(&self) -> String {
        match self {
            Self::Refused { reason } => format!("athleticlive_manifest: refused: {reason}"),
            Self::Armed { digest } => format!("athleticlive_manifest: armed manifest sha256={digest}; no attempt recorded"),
            Self::Running { digest, attempt } => format!("athleticlive_manifest: manifest sha256={digest} attempt={attempt} reserved; completion unknown"),
            Self::Finished { digest, attempt, outcome } => format!("athleticlive_manifest: manifest sha256={digest} attempt={attempt} records={} disposition={:?} unfinished={:?} errors={:?} notes={:?}", outcome.records, outcome.disposition, outcome.unfinished, outcome.errors, outcome.notes),
        }
    }

    pub(crate) fn digest(&self) -> Option<&str> {
        match self {
            Self::Refused { .. } => None,
            Self::Armed { digest }
            | Self::Running { digest, .. }
            | Self::Finished { digest, .. } => Some(digest),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleticLiveManifestRequest {
    pub jurisdiction: JurisdictionRequest,
    pub input: String,
}
