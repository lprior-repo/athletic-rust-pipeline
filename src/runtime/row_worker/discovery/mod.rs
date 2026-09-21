mod fetch;
mod profiles;
mod queries;

use crate::domain::identity::{AthleteId, EvidenceDigest};
use std::collections::BTreeSet;

pub(crate) use profiles::{execute_profiles, ProfileState};
pub(crate) use queries::execute_queries;

#[derive(Debug, Default)]
pub(crate) struct DiscoveryState {
    pub incomplete: bool,
    pub refs: Vec<EvidenceDigest>,
    pub issues: Vec<String>,
    pub candidate_ids: BTreeSet<AthleteId>,
    pub candidate_limit: bool,
}

impl DiscoveryState {
    pub(crate) fn complete(&self) -> bool {
        !self.incomplete && !self.candidate_limit && self.issues.is_empty()
    }
}
