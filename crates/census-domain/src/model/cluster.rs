use super::{AthleteCandidateId, AthleteCandidateKey, AthleteId};
use serde::{Deserialize, Serialize};

/// The resolved identity membership for one raw athlete candidate.
///
/// This is a derived audit row rather than a rewrite of the raw athlete observation. Keeping the
/// candidate key and accepted case ids beside the winning cluster makes the projection reversible
/// and lets readers explain why a candidate belongs to its current survivor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AthleteClusterMember {
    pub candidate_id: AthleteCandidateId,
    pub cluster_id: AthleteId,
    pub candidate_key: AthleteCandidateKey,
    pub is_canonical: bool,
    #[serde(default)]
    pub accepted_case_ids: Vec<String>,
}

impl AthleteClusterMember {
    pub fn new(
        candidate_id: AthleteCandidateId,
        cluster_id: AthleteId,
        candidate_key: AthleteCandidateKey,
        is_canonical: bool,
        accepted_case_ids: Vec<String>,
    ) -> Self {
        Self {
            candidate_id,
            cluster_id,
            candidate_key,
            is_canonical,
            accepted_case_ids,
        }
    }
}
