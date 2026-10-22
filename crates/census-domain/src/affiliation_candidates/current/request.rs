use super::{CurrentAffiliationError, CurrentAffiliationInventory, CurrentAffiliationRequest};

pub(super) fn check(
    request: &CurrentAffiliationRequest<'_>,
    inventory: &CurrentAffiliationInventory<'_>,
) -> Result<(), CurrentAffiliationError> {
    if request.known != inventory.declared_view {
        return Err(CurrentAffiliationError::InventoryViewMismatch);
    }
    let profile = request.profile.input();
    if !profile.sports.contains(&request.sport) {
        return Err(CurrentAffiliationError::SportNotSelected);
    }
    if request.acquisition_cutoff > request.known.captured_at {
        return Err(CurrentAffiliationError::AcquisitionCutoffAfterFrozenInstant);
    }
    Ok(())
}
