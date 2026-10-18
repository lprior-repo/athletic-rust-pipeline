use super::{invalid, ExportDataset, ReportResult, StoreSnapshot};
use census_store::AcceptedPublicIdentitySnapshot;

impl ExportDataset {
    pub(crate) fn ensure_current_documentary_authority(
        &self,
        snapshot: &StoreSnapshot<'_>,
    ) -> ReportResult<()> {
        if let Some(accepted) = checked_identity(self, snapshot)? {
            self.verified_participation()?
                .require_current_documentary_authority(snapshot, &accepted)?;
        }
        Ok(())
    }
}

pub(super) fn checked_identity(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
) -> ReportResult<Option<AcceptedPublicIdentitySnapshot>> {
    if !dataset
        .selection
        .as_ref()
        .is_some_and(|binding| binding.authority.is_some())
    {
        return Ok(None);
    }
    let authority = dataset.selected_authority()?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or_else(|| invalid("selected authority disappeared from the current snapshot"))?;
    authority.ensure_identity(&accepted)?;
    Ok(Some(accepted))
}
