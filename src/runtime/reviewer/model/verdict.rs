use crate::domain::identity::AthleteId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision", deny_unknown_fields)]
pub enum AssistantVerdict {
    Select {
        athlete_id: AthleteId,
        reason: String,
        evidence: Vec<AssistantEvidenceRef>,
    },
    Unresolved {
        reason: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssistantEvidenceRef {
    pub document: crate::domain::identity::EvidenceDigest,
    pub locator: String,
}
