use super::{invalid, SelectedAuthority};
use crate::export::dataset::{frozen, read, ExportDataset, Loaded};
use crate::export::qualification::participation::{archive, school_projection};
use crate::export::qualification::SelectionBinding;
use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileSelectionV1, QueryProfileV1};
use census_store::{
    FrozenPublicIdentityBundleV2, ReplayedResultSchoolCorrespondencesV1, Store, StoreSnapshot,
};
use std::path::Path;

#[path = "preflight.rs"]
mod preflight;
#[path = "school.rs"]
mod school;

impl ExportDataset {
    pub fn from_selected_snapshot(
        store: &Store,
        snapshot: &StoreSnapshot<'_>,
        selection: &ProfileSelectionV1,
        objects_directory: &Path,
    ) -> ReportResult<Self> {
        let (profile, authority) = selected_authority(store, snapshot, selection)?;
        let mut loaded = read::all(snapshot)?;
        let lineage = frozen::lineage(store, snapshot)?;
        let identity = snapshot.freeze_public_identity_v2()?;
        let mut grant = AttachmentReplayGrantV1::new();
        let school_closure = capture_school(snapshot, &identity, &mut grant)?;
        let replayed_school = school_closure
            .as_ref()
            .map(|closure| {
                closure.replay(
                    &lineage.store_identity,
                    lineage.snapshot_sequence,
                    &mut grant,
                )
            })
            .transpose()?;
        extend_school(&mut loaded, replayed_school.as_ref(), &mut grant)?;
        loaded.selection = Some(SelectionBinding {
            profile_id: profile.id().map_err(census_store::StoreError::from)?,
            as_of: profile.input().as_of.clone(),
            participation_rule: Some(profile.input().policies.participation.clone()),
            authority: Some(authority),
        });
        let mut dataset = Self::from_loaded(loaded, lineage)?;
        dataset.selected_input_origin = super::super::SelectedInputOrigin::Current;
        let fingerprints = dataset.selected_authority()?.fingerprints().clone();
        let school_context =
            school_closure
                .as_ref()
                .zip(replayed_school.as_ref())
                .map(|(closure, source)| {
                    crate::export::qualification::participation::SchoolContext {
                        source,
                        closure,
                        grant: &mut grant,
                    }
                });
        let (proofs, hydrated) = super::super::selected_proofs::SelectedProofs::capture(
            &mut dataset,
            snapshot,
            &profile,
            &fingerprints,
            objects_directory,
            &identity,
            school_context,
        )?;
        dataset.attach_selected_proofs(proofs, hydrated)?;
        Ok(dataset)
    }
}

fn selected_authority(
    store: &Store,
    snapshot: &StoreSnapshot<'_>,
    selection: &ProfileSelectionV1,
) -> ReportResult<(QueryProfileV1, SelectedAuthority)> {
    snapshot.ensure_origin(store)?;
    let profile = snapshot.selected_profile(selection)?;
    census_domain::profile_execution::validate_current_selected_qualification_profile(&profile)
        .map_err(|error| invalid(&error.to_string()))?;
    preflight::admit(snapshot)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or_else(|| invalid("selected publication has no retained public identity authority"))?;
    let authority = SelectedAuthority::capture(profile.clone(), snapshot, &accepted)?;
    authority.validate()?;
    Ok((profile, authority))
}

fn capture_school(
    snapshot: &StoreSnapshot<'_>,
    identity: &FrozenPublicIdentityBundleV2,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<Option<archive::SchoolClosure>> {
    if snapshot.result_school_enrollment_proof_v1(grant)?.is_some() {
        Ok(Some(archive::capture_school(snapshot, identity, grant)?))
    } else {
        Ok(None)
    }
}

fn extend_school(
    loaded: &mut Loaded,
    source: Option<&ReplayedResultSchoolCorrespondencesV1<'_>>,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<()> {
    if let Some(source) = source {
        school::extend(loaded, school_projection::collect(source, grant)?)?;
    }
    Ok(())
}
