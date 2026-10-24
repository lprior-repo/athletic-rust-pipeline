use super::ExportDataset;
use crate::report::ReportError;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::{CanonicalAthlete, SourceIdentity, SourceNamespace};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod capture;
mod replay;
mod root;
mod roots;
mod scope;
use root::{ReportedProfileRootSchemaV1, ReportedProfileRootV1};
pub(in crate::export::dataset) use scope::SourceScope;

#[cfg(test)]
pub(crate) mod tests;

const MAX_ROOTS: usize = 4_000_000;
const MAX_PACKET_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReportedProfilePrs {
    #[serde(deserialize_with = "roots::deserialize")]
    roots: Vec<ReportedProfileRootV1>,
    #[serde(skip)]
    tables: BTreeMap<String, Vec<ReportedProfilePrTable>>,
}

pub(crate) struct ReportedProfilePrTable {
    pub(crate) source: SourceIdentity,
    pub(crate) profile_url: String,
    pub(crate) body_sha256: String,
    pub(crate) acquired_at: DateTime<Utc>,
    pub(crate) rows: Vec<SourceProfilePrV1>,
    pub(crate) root: ReportedProfileRootV1,
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

    pub(crate) fn has_owned_profile_prs(&self) -> bool {
        self.reported_profile_prs
            .roots
            .iter()
            .any(|root| root.schema == ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1)
    }

    pub(crate) fn reported_profile_prs_for<'a>(
        &'a self,
        athlete: &'a CanonicalAthlete,
    ) -> impl Iterator<Item = &'a ReportedProfilePrTable> {
        let mut subjects = BTreeSet::new();
        athlete
            .identities()
            .filter(|source| source.namespace == SourceNamespace::MilesplitAthlete)
            .filter(move |source| subjects.insert(source.id.as_str()))
            .flat_map(|source| self.reported_profile_prs.tables.get(&source.id))
            .flatten()
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}
