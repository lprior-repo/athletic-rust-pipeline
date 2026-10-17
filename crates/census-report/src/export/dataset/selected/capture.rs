use super::{invalid, SelectedAuthority};
use crate::export::dataset::{frozen, read, ExportDataset};
use crate::export::qualification::SelectionBinding;
use crate::report::ReportResult;
use census_domain::model::ProfileSelectionV1;
use census_store::{Store, StoreSnapshot};
use std::path::Path;

#[path = "preflight.rs"]
mod preflight;

impl ExportDataset {
    pub fn from_selected_snapshot(
        store: &Store,
        snapshot: &StoreSnapshot<'_>,
        selection: &ProfileSelectionV1,
        objects_directory: &Path,
    ) -> ReportResult<Self> {
        snapshot.ensure_origin(store)?;
        let profile = snapshot.selected_profile(selection)?;
        census_domain::profile_execution::validate_current_selected_qualification_profile(&profile)
            .map_err(|error| invalid(&error.to_string()))?;
        preflight::admit(snapshot)?;
        let accepted = snapshot
            .accepted_public_identity_snapshot()?
            .ok_or_else(|| {
                invalid("selected publication has no retained public identity authority")
            })?;
        let authority = SelectedAuthority::capture(profile.clone(), snapshot, &accepted)?;
        authority.validate()?;
        drop(accepted);
        let mut loaded = read::all(snapshot)?;
        loaded.selection = Some(SelectionBinding {
            profile_id: profile.id().map_err(census_store::StoreError::from)?,
            as_of: profile.input().as_of.clone(),
            participation_rule: Some(profile.input().policies.participation.clone()),
            authority: Some(authority),
        });
        let mut dataset = Self::from_loaded(loaded, frozen::lineage(store, snapshot)?)?;
        dataset.selected_input_origin = super::super::SelectedInputOrigin::Current;
        let fingerprints = dataset.selected_authority()?.fingerprints().clone();
        let (proofs, hydrated) = super::super::selected_proofs::SelectedProofs::capture(
            &mut dataset,
            snapshot,
            &profile,
            &fingerprints,
            objects_directory,
        )?;
        dataset.attach_selected_proofs(proofs, hydrated)?;
        Ok(dataset)
    }
}
