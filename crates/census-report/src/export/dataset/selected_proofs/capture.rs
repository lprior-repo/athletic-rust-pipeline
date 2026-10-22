use super::{
    athlete_fingerprint, identity, invalid, objects, require_binding, require_physical_roots,
    HydratedProofs, SelectedProofs, Version,
};
use crate::export::dataset::ExportDataset;
use crate::export::qualification::current_affiliation::{self, FrozenAffiliationDescriptor};
use crate::export::qualification::participation::PinnedParticipation;
use crate::report::ReportResult;
use census_domain::model::QueryProfileV1;
use census_store::StoreSnapshot;
use std::collections::BTreeMap;
use std::path::Path;

impl SelectedProofs {
    pub(crate) fn capture(
        dataset: &mut ExportDataset,
        snapshot: &StoreSnapshot<'_>,
        profile: &QueryProfileV1,
        expected_fingerprints: &BTreeMap<String, String>,
        directory: &Path,
        identity: &census_store::FrozenPublicIdentityBundleV2,
        school: Option<crate::export::qualification::participation::SchoolContext<'_, '_>>,
    ) -> ReportResult<(Self, HydratedProofs)> {
        require_binding(dataset, profile, expected_fingerprints)?;
        require_physical_roots(snapshot, expected_fingerprints)?;
        dataset
            .reported_profile_prs
            .require_initial_objects(directory)?;
        dataset.prepare_selected_public_identities(identity.replay()?)?;
        let public_identity = identity::Bundle::capture(directory, identity)?;
        let (affiliation, affiliations) =
            current_affiliation::capture_frozen(dataset, snapshot, directory)?;
        let affiliation_digest = affiliation.digest()?;
        let expected_athletes =
            Self::require_affiliation_athletes(&affiliation, expected_fingerprints)?;
        let participation = PinnedParticipation::read(
            dataset,
            snapshot,
            profile,
            directory,
            affiliation.observations(),
            expected_athletes,
            affiliations.roster_context(),
            school,
        )?;
        if participation.fingerprints() != *expected_fingerprints {
            return Err(invalid(
                "selected participation differs from the independent physical roots",
            ));
        }
        let participation_digest = participation.install_replay(directory)?;
        let proofs = Self::seal_capture(
            affiliation,
            affiliation_digest,
            participation_digest,
            public_identity,
            directory,
        )?;
        Ok((
            proofs,
            HydratedProofs {
                participation,
                affiliations,
            },
        ))
    }

    fn require_affiliation_athletes<'a>(
        affiliation: &FrozenAffiliationDescriptor,
        expected_fingerprints: &'a BTreeMap<String, String>,
    ) -> ReportResult<&'a str> {
        let expected_athletes = athlete_fingerprint(expected_fingerprints)?;
        if affiliation.athlete_table_digest() != expected_athletes {
            return Err(invalid(
                "selected affiliation differs from the independent athlete fingerprint",
            ));
        }
        Ok(expected_athletes)
    }

    fn seal_capture(
        affiliation: FrozenAffiliationDescriptor,
        affiliation_digest: String,
        participation_digest: String,
        public_identity: identity::Bundle,
        directory: &Path,
    ) -> ReportResult<Self> {
        let objects = objects::inventory(directory)?;
        let proofs = Self {
            version: if affiliation.reviewed_roster() {
                Version::ReviewedRosterGraphV3
            } else {
                Version::RetainedGraphV2
            },
            affiliation,
            affiliation_digest,
            participation_digest,
            public_identity,
            objects,
            objects_directory: Some(directory.to_path_buf()),
        };
        proofs.validate()?;
        Ok(proofs)
    }
}
