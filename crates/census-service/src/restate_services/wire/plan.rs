
use serde::{Deserialize, Serialize};

use crate::restate_services::plan::{owed, sweepable, UnitDisposition};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePlan {
    #[serde(default)]
    pub sweepable: Vec<String>,
    #[serde(default)]
    pub refused: Vec<RefusedSource>,
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
                })
                .collect(),
            fingerprint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusedSource {
    pub slug: String,
    pub reason: String,
}
