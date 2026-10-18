use super::ExportDataset;
use crate::export::qualification::current_affiliation::{
    self, FrozenAffiliationDescriptor, PinnedCurrentAffiliations,
};
use crate::export::qualification::participation::PinnedParticipation;
use crate::report::{ReportError, ReportResult};
use census_domain::model::QueryProfileV1;
use census_store::{StoreError, StoreSnapshot, Table};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod capture;
mod identity;
mod objects;
mod version;
pub(in crate::export::dataset) use version::Version;
#[cfg(test)]
mod tests;

const TABLES: [Table; 5] = [
    Table::Athletes,
    Table::Performances,
    Table::Teams,
    Table::Meets,
    Table::Events,
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectedProofs {
    version: Version,
    affiliation: FrozenAffiliationDescriptor,
    affiliation_digest: String,
    participation_digest: String,
    public_identity: identity::Bundle,
    #[serde(deserialize_with = "objects::deserialize_inventory")]
    objects: Vec<objects::Reference>,
    #[serde(skip)]
    objects_directory: Option<PathBuf>,
}

pub(crate) struct HydratedProofs {
    pub(crate) participation: PinnedParticipation,
    pub(crate) affiliations: PinnedCurrentAffiliations,
}

impl SelectedProofs {
    pub(in crate::export::dataset) fn version(&self) -> Version {
        self.version
    }

    pub(crate) fn hydrate(
        &self,
        dataset: &mut ExportDataset,
        profile: &QueryProfileV1,
        expected_fingerprints: &BTreeMap<String, String>,
        directory: &Path,
    ) -> ReportResult<HydratedProofs> {
        require_binding(dataset, profile, expected_fingerprints)?;
        self.validate()?;
        objects::verify(directory, &self.objects)?;
        match self.version {
            Version::HistoricalContextV1 => dataset.replay_historical_public_identities(
                self.public_identity.replay_historical(directory)?,
            )?,
            Version::RetainedGraphV2 | Version::ReviewedRosterGraphV3 => dataset
                .prepare_selected_public_identities(self.public_identity.replay(directory)?)?,
        }
        let affiliations = current_affiliation::replay_frozen(
            dataset,
            directory,
            &self.affiliation,
            athlete_fingerprint(expected_fingerprints)?,
            &self.affiliation_digest,
        )?;
        let participation = PinnedParticipation::replay(
            dataset,
            profile,
            directory,
            &self.participation_digest,
            expected_fingerprints,
            affiliations.roster_context(),
        )?;
        Ok(HydratedProofs {
            participation,
            affiliations,
        })
    }

    pub(crate) fn require_current(&self) -> ReportResult<()> {
        self.validate()?;
        if !matches!(
            self.version,
            Version::RetainedGraphV2 | Version::ReviewedRosterGraphV3
        ) {
            return Err(invalid(
                "historical selected identity proofs cannot authorize current publication",
            ));
        }
        Ok(())
    }

    fn validate(&self) -> ReportResult<()> {
        if self.affiliation.reviewed_roster() != (self.version == Version::ReviewedRosterGraphV3)
            || !digest(&self.affiliation_digest)
            || !digest(&self.participation_digest)
            || self.affiliation.digest()? != self.affiliation_digest
        {
            return Err(invalid(
                "selected proof envelope version or roots are invalid",
            ));
        }
        objects::validate(&self.objects)?;
        self.public_identity.validate()?;
        if !self
            .objects
            .iter()
            .any(|reference| reference.digest == self.participation_digest)
            || self
                .public_identity
                .chunks()
                .iter()
                .any(|chunk| !self.objects.contains(chunk))
        {
            return Err(invalid(
                "selected proof root or identity chunk is absent from its inventory",
            ));
        }
        Ok(())
    }
}

fn require_binding(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    fingerprints: &BTreeMap<String, String>,
) -> ReportResult<()> {
    dataset.ensure_input_binding()?;
    if dataset.selected_profile_id() != Some(&profile.id().map_err(StoreError::from)?)
        || dataset.selected_as_of() != Some(&profile.input().as_of)
        || fingerprints.len() != TABLES.len()
        || TABLES.iter().any(|table| {
            !fingerprints
                .get(table.file())
                .is_some_and(|value| digest(value))
        })
    {
        return Err(invalid(
            "selected proofs require the bound profile and five independent table roots",
        ));
    }
    Ok(())
}

fn require_physical_roots(
    snapshot: &StoreSnapshot<'_>,
    expected: &BTreeMap<String, String>,
) -> ReportResult<()> {
    for table in TABLES {
        if expected.get(table.file()) != Some(&snapshot.tables_digest(&[table])?) {
            return Err(invalid(
                "selected proof authority differs from the pinned physical table",
            ));
        }
    }
    Ok(())
}

fn athlete_fingerprint(fingerprints: &BTreeMap<String, String>) -> ReportResult<&str> {
    fingerprints
        .get(Table::Athletes.file())
        .map(String::as_str)
        .ok_or_else(|| invalid("selected proofs omit the independent athlete fingerprint"))
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
