use census_domain::model::{ProfileId, QueryProfileV1};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod canonical;
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParticipationReplayV2 {
    pub(super) version: u32,
    pub(super) retained: ParticipationReplayV1,
    pub(super) independent: census_crawl::participation_evidence::IndependentParticipationBundleV1,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ParticipationReplay {
    V1(ParticipationReplayV1),
    V2(ParticipationReplayV2),
}

impl ParticipationReplay {
    pub(crate) fn retained(&self) -> &ParticipationReplayV1 {
        match self {
            Self::V1(value) => value,
            Self::V2(value) => &value.retained,
        }
    }
    pub(crate) fn independent(
        &self,
    ) -> Option<&census_crawl::participation_evidence::IndependentParticipationBundleV1> {
        match self {
            Self::V1(_) => None,
            Self::V2(value) => Some(&value.independent),
        }
    }
    pub(crate) fn fingerprints(&self) -> std::collections::BTreeMap<String, String> {
        self.retained().fingerprints()
    }
    pub(crate) fn install(
        &self,
        directory: &std::path::Path,
    ) -> crate::report::ReportResult<String> {
        match self {
            Self::V1(value) => value.install(directory),
            Self::V2(value) => objects::install(value, directory, 2),
        }
    }
    pub(crate) fn load(
        directory: &std::path::Path,
        descriptor: &str,
    ) -> crate::report::ReportResult<Self> {
        match objects::version(directory, descriptor)? {
            1 => ParticipationReplayV1::load(directory, descriptor).map(Self::V1),
            2 => objects::load(directory, descriptor, 2).map(Self::V2),
            _ => Err(super::invalid("unsupported participation replay descriptor version").into()),
        }
    }
}
