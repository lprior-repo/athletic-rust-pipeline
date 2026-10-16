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

mod identity;
mod objects;
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
pub(crate) struct SelectedProofsV1 {
    version: u32,
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

impl SelectedProofsV1 {
    pub(crate) fn capture(
        dataset: &mut ExportDataset,
        snapshot: &StoreSnapshot<'_>,
        profile: &QueryProfileV1,
        expected_fingerprints: &BTreeMap<String, String>,
        directory: &Path,
    ) -> ReportResult<(Self, HydratedProofs)> {
        require_binding(dataset, profile, expected_fingerprints)?;
        for table in TABLES {
            if expected_fingerprints.get(table.file()) != Some(&snapshot.tables_digest(&[table])?) {
                return Err(invalid(
                    "selected proof authority differs from the pinned physical table",
                ));
            }
        }
        objects::require_empty(directory)?;
        let bundle = snapshot.freeze_public_identity_v1()?;
        dataset.prepare_selected_public_identities(bundle.replay()?)?;
        let public_identity = identity::Bundle::capture(directory, &bundle)?;
        let (affiliation, affiliations) =
            current_affiliation::capture_frozen(dataset, snapshot, directory)?;
        let affiliation_digest = affiliation.digest()?;
        let expected_athletes = athlete_fingerprint(expected_fingerprints)?;
        if affiliation.athlete_table_digest() != expected_athletes {
            return Err(invalid(
                "selected affiliation differs from the independent athlete fingerprint",
            ));
        }
        let participation = PinnedParticipation::read(
            dataset,
            snapshot,
            profile,
            directory,
            affiliation.observations(),
            expected_athletes,
            affiliations.roster_context(),
        )?;
        if participation.fingerprints() != *expected_fingerprints {
            return Err(invalid(
                "selected participation differs from the independent physical roots",
            ));
        }
        let participation_digest = participation.install_replay(directory)?;
        let objects = objects::inventory(directory)?;
        let proofs = Self {
            version: 1,
            affiliation,
            affiliation_digest,
            participation_digest,
            public_identity,
            objects,
            objects_directory: Some(directory.to_path_buf()),
        };
        proofs.validate()?;
        Ok((
            proofs,
            HydratedProofs {
                participation,
                affiliations,
            },
        ))
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
        dataset.prepare_selected_public_identities(self.public_identity.replay(directory)?)?;
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

    pub(crate) fn object_bytes(&self) -> ReportResult<u64> {
        self.validate()?;
        self.objects.iter().try_fold(0_u64, |total, object| {
            total
                .checked_add(object.bytes)
                .ok_or_else(|| invalid("proof inventory byte count overflow"))
        })
    }

    pub(crate) fn verify_objects(&self, directory: &Path) -> ReportResult<()> {
        self.validate()?;
        objects::verify(directory, &self.objects)
    }

    pub(crate) fn bind_directory(&mut self, directory: &Path) -> ReportResult<()> {
        self.validate()?;
        objects::verify(directory, &self.objects)?;
        self.objects_directory = Some(directory.to_path_buf());
        Ok(())
    }

    pub(crate) fn copy_objects(&self, destination: &Path) -> ReportResult<()> {
        self.validate()?;
        let source = self
            .objects_directory
            .as_deref()
            .ok_or_else(|| invalid("selected proof objects have no retained directory"))?;
        objects::copy(source, destination, &self.objects)
    }

    fn validate(&self) -> ReportResult<()> {
        if self.version != 1
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
