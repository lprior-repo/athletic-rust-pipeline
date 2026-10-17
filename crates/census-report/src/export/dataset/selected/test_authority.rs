use super::*;
use census_domain::model::QueryProfileV1;
use census_store::StoreSnapshot;

impl ExportDataset {
    pub(crate) fn bind_selected_authority_fixture_from_snapshot(
        &mut self,
        profile: &QueryProfileV1,
        snapshot: &StoreSnapshot<'_>,
    ) -> ReportResult<()> {
        let accepted = snapshot
            .accepted_public_identity_snapshot()?
            .ok_or_else(|| ReportError::Invariant {
                detail: "selected fixture requires actual accepted identity".into(),
            })?;
        let authority = SelectedAuthority::capture(profile.clone(), snapshot, &accepted)?;
        authority.validate()?;
        if snapshot.sequence() != self.lineage.snapshot_sequence
            || snapshot.store_identity()?.as_str() != self.lineage.store_identity
        {
            return Err(ReportError::Invariant {
                detail: "selected fixture snapshot differs from loaded input".into(),
            });
        }
        self.selection = Some(crate::export::qualification::SelectionBinding {
            profile_id: profile.id().map_err(census_store::StoreError::from)?,
            as_of: profile.input().as_of.clone(),
            participation_rule: Some(profile.input().policies.participation.clone()),
            authority: Some(authority),
        });
        frozen::bind(self)
    }
}
