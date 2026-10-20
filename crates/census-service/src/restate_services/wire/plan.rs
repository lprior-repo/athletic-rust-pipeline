use census_domain::model::{CoachLaneError, CoachLaneRefusal, SourceRef};
use serde::{Deserialize, Serialize};

use crate::restate_services::plan::{owed, sweepable, UnitDisposition};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePlan {
    #[serde(default)]
    pub sweepable: Vec<String>,
    #[serde(default)]
    pub refused: Vec<RefusedSource>,
    /// Source families whose already-retained inputs satisfy the plan without acquisition.
    /// Added with a default so earlier Restate journals remain readable.
    #[serde(default)]
    pub satisfied: Vec<SatisfiedSource>,
    #[serde(default)]
    pub fingerprint: String,
}

impl SourcePlan {
    pub fn of(dispositions: &[UnitDisposition], fingerprint: String) -> Self {
        Self {
            sweepable: sweepable(dispositions)
                .iter()
                .map(|unit| unit.slug.to_string())
                .collect(),
            refused: owed(dispositions)
                .iter()
                .map(|refusal| RefusedSource {
                    slug: refusal.slug.to_string(),
                    reason: refusal.reason.to_string(),
                    kind: refusal.kind,
                })
                .collect(),
            satisfied: Vec::new(),
            fingerprint,
        }
    }

    pub fn has_engineering_gaps(&self) -> bool {
        self.refused.iter().any(|refusal| {
            matches!(
                refusal.kind,
                crate::restate_services::plan::RefusalKind::Unknown
                    | crate::restate_services::plan::RefusalKind::EngineeringGap
            )
        })
    }

    pub fn coach_lane_refusals(
        &self,
        observed_on: &str,
    ) -> Result<Vec<CoachLaneRefusal>, CoachLaneError> {
        self.refused
            .iter()
            .map(|refused| {
                CoachLaneRefusal::new(
                    SourceRef::id(refused.slug.clone()),
                    refused.reason.clone(),
                    observed_on,
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SatisfiedSource {
    pub slug: String,
    /// Digest of the run-bound receipt that makes this retained source complete.
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusedSource {
    pub slug: String,
    pub reason: String,
    #[serde(default)]
    pub kind: crate::restate_services::plan::RefusalKind,
}
