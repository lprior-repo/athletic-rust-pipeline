use super::ExportDataset;
use crate::report::ReportError;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::{CanonicalAthlete, SourceIdentity, SourceNamespace};
use census_store::RetainedCaptureRootV1;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod capture;
mod replay;
mod roots;

#[cfg(test)]
pub(crate) mod tests;

const MAX_ROOTS: usize = 16_384;
const MAX_PACKET_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReportedProfilePrs {
    #[serde(deserialize_with = "roots::deserialize")]
    roots: Vec<RetainedCaptureRootV1>,
    #[serde(skip)]
    tables: BTreeMap<String, Vec<ReportedProfilePrTable>>,
}

pub(crate) struct ReportedProfilePrTable {
    pub(crate) source: SourceIdentity,
    pub(crate) profile_url: String,
    pub(crate) body_sha256: String,
    pub(crate) acquired_at: DateTime<Utc>,
    pub(crate) rows: Vec<SourceProfilePrV1>,
    pub(crate) root: RetainedCaptureRootV1,
    pub(crate) interpretation: String,
}

impl ReportedProfilePrs {
    pub(super) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
}

impl ExportDataset {
    pub(crate) fn has_reported_profile_prs(&self) -> bool {
        !self.reported_profile_prs.is_empty()
    }

    pub(crate) fn reported_profile_prs_for<'a>(
        &'a self,
        athlete: &'a CanonicalAthlete,
    ) -> impl Iterator<Item = &'a ReportedProfilePrTable> {
        athlete
            .identities()
            .filter(|source| source.namespace == SourceNamespace::MilesplitAthlete)
            .flat_map(|source| self.reported_profile_prs.tables.get(&source.id))
            .flatten()
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}
