use super::{invalid, SelectedAuthority};
use crate::export::dataset::{frozen, read, ExportDataset};
use crate::export::qualification::SelectionBinding;
use crate::report::ReportResult;
use census_domain::model::ProfileSelectionV1;
use census_store::{Store, StoreSnapshot};
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
        Self::selected_snapshot_for_job(store, snapshot, selection, objects_directory, None)
    }

    pub(in crate::export::dataset) fn selected_snapshot_for_job(
        store: &Store,
        snapshot: &StoreSnapshot<'_>,
        selection: &ProfileSelectionV1,
        objects_directory: &Path,
        job: Option<&str>,
    ) -> ReportResult<Self> {
        snapshot.ensure_origin(store)?;
        let (profile, authority) = capture_authority(snapshot, selection)?;
        let mut loaded = read::all(snapshot)?;
        let mut lineage = frozen::lineage(store, snapshot)?;
        lineage.export_job = job.map(str::to_owned);
        let identity = snapshot.freeze_public_identity_v2()?;
        let mut grant = census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new();
        let school_closure = if snapshot
            .result_school_enrollment_proof_v1(&mut grant)?
            .is_some()
        {
            Some(
                crate::export::qualification::participation::archive::capture_school(
                    snapshot, &identity, &mut grant,
                )?,
            )
        } else {
            None
        };
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
        if let Some(source) = &replayed_school {
            extend_school_projection(source, &mut grant, &mut loaded)?;
        }
        let mut dataset = selected_dataset(loaded, lineage, &profile, authority)?;
        attach_selected_proofs(
            &mut dataset,
            SelectedProofInputs {
                snapshot,
                profile: &profile,
                objects_directory,
                identity: &identity,
                grant: &mut grant,
                closure: school_closure.as_ref(),
                source: replayed_school.as_ref(),
            },
        )?;
        Ok(dataset)
    }
}

struct SelectedProofInputs<'a, 'store, 'source> {
    snapshot: &'a StoreSnapshot<'store>,
    profile: &'a census_domain::model::QueryProfileV1,
    objects_directory: &'a Path,
    identity: &'a census_store::FrozenPublicIdentityBundleV2,
    grant: &'a mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
    closure: Option<&'a crate::export::qualification::participation::archive::SchoolClosure>,
    source: Option<&'a census_store::ReplayedResultSchoolCorrespondencesV1<'source>>,
}

fn attach_selected_proofs(
    dataset: &mut ExportDataset,
    inputs: SelectedProofInputs<'_, '_, '_>,
) -> ReportResult<()> {
    let fingerprints = dataset.selected_authority()?.fingerprints().clone();
    let school_context = inputs.closure.zip(inputs.source).map(|(closure, source)| {
        crate::export::qualification::participation::SchoolContext {
            source,
            closure,
            grant: inputs.grant,
        }
    });
    let (proofs, hydrated) = super::super::selected_proofs::SelectedProofs::capture(
        dataset,
        inputs.snapshot,
        inputs.profile,
        &fingerprints,
        inputs.objects_directory,
        inputs.identity,
        school_context,
    )?;
    dataset.attach_selected_proofs(proofs, hydrated)
}

fn selected_dataset(
    mut loaded: super::super::Loaded,
    lineage: super::super::DatasetLineage,
    profile: &census_domain::model::QueryProfileV1,
    authority: SelectedAuthority,
) -> ReportResult<ExportDataset> {
    loaded.selection = Some(SelectionBinding {
        profile_id: profile.id().map_err(census_store::StoreError::from)?,
        as_of: profile.input().as_of.clone(),
        participation_rule: Some(profile.input().policies.participation.clone()),
        authority: Some(authority),
    });
    let mut dataset = ExportDataset::from_loaded(loaded, lineage)?;
    dataset.selected_input_origin = super::super::SelectedInputOrigin::Current;
    Ok(dataset)
}

fn capture_authority(
    snapshot: &StoreSnapshot<'_>,
    selection: &ProfileSelectionV1,
) -> ReportResult<(census_domain::model::QueryProfileV1, SelectedAuthority)> {
    let profile = snapshot.selected_profile(selection)?;
    census_domain::profile_execution::validate_current_selected_qualification_profile(&profile)
        .map_err(|error| invalid(&error.to_string()))?;
    preflight::admit(snapshot)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or_else(|| invalid("selected publication has no retained public identity authority"))?;
    let authority = SelectedAuthority::capture(profile.clone(), snapshot, &accepted)?;
    authority.validate()?;
    drop(accepted);
    Ok((profile, authority))
}

fn extend_school_projection(
    source: &census_store::ReplayedResultSchoolCorrespondencesV1<'_>,
    grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
    loaded: &mut super::super::Loaded,
) -> ReportResult<()> {
    let projection =
        crate::export::qualification::participation::school_projection::collect(source, grant)?;
    school::extend(loaded, projection)
}
