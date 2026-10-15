use census_domain::model::{ProfileId, QueryProfileV1};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod objects;
pub(super) use objects::hash;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParticipationReplayV1 {
    pub(super) version: u32,
    pub(super) store_identity: String,
    pub(super) snapshot_sequence: u64,
    pub(super) captured_at: DateTime<Utc>,
    pub(super) input_generation: String,
    pub(super) input_digest: String,
    pub(super) source_digest: String,
    pub(super) profile_id: ProfileId,
    pub(super) profile: QueryProfileV1,
    pub(super) observations:
        Vec<crate::export::qualification::physical_observations::PhysicalObservationDescriptor>,
    pub(super) journal: census_crawl::milesplit::RetainedJournalBundle,
}

impl ParticipationReplayV1 {
    pub(crate) fn fingerprints(&self) -> std::collections::BTreeMap<String, String> {
        super::observations::fingerprints(&self.observations)
    }
}
