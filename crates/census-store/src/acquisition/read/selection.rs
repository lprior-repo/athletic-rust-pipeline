use super::{read_work, AcquisitionReadBudget, CaptureError};
use crate::acquisition::{facts::digest, ResultsCaptureOwnerV1};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::*;
use fjall::Readable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExistingRecoveryRoute {
    Unregistered,
    ExistingNoPrepared,
    Prepared(PreparedRecoveryTarget),
}

impl Store {
    pub fn read_existing_recovery_target(
        &self,
        owner: &ResultsCaptureOwnerV1,
        exact_work: &AcquisitionWorkV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<ExistingRecoveryRoute> {
        if owner.protocol() != CaptureProtocolVersion::V2 {
            return Err(CaptureError::CaptureProtocolUnsupported.into());
        }
        let _writes = self.lock_appends();
        crate::acquisition::admission::owner(self, owner, budget)?;
        crate::acquisition::admission::matches_work(owner, exact_work)?;
        budget.target_examined()?;
        budget.reserve_recovery_owned(MAX_CAPTURE_RECORD_BYTES + 8192)?;
        let snapshot = self.db.snapshot();
        let id = exact_work.id().map_err(CaptureError::Domain)?;
        let slot = exact_work.slot().id().map_err(CaptureError::Domain)?;
        let work_prefix = Store::journal_key(owner.protocol().phase(), &format!("work/{id}/"));
        let row = snapshot.prefix(&self.journal, &work_prefix).next();
        let present_work = row
            .map(|guard| guard.key().map_err(|source| StoreError::Read { source }))
            .transpose()?
            .is_some();
        let binding = Store::journal_key(owner.protocol().phase(), &format!("slot/{slot}/binding"));
        let present_slot = snapshot
            .contains_key(&self.journal, &binding)
            .map_err(|source| StoreError::Read { source })?;
        match (present_work, present_slot) {
            (false, false) => {
                budget.absent_target()?;
                return Ok(ExistingRecoveryRoute::Unregistered);
            }
            (true, true) => {}
            _ => return Err(CaptureError::SlotWorkMismatch.into()),
        }
        let facts = read_work(
            &snapshot,
            &self.journal,
            &self.receipts,
            &id,
            budget,
            owner.protocol(),
        )?;
        if &facts.work != exact_work {
            return Err(CaptureError::ObligationConflict.into());
        }
        prepared_route(owner, slot, &facts)
    }
}

fn prepared_route(
    owner: &ResultsCaptureOwnerV1,
    slot: AcquisitionSlotId,
    facts: &crate::acquisition::facts::WorkFacts,
) -> StoreResult<ExistingRecoveryRoute> {
    let Some(last) = facts.attempts.last() else {
        return Ok(ExistingRecoveryRoute::ExistingNoPrepared);
    };
    if facts.result.is_some() || last.terminal.is_some() {
        return Ok(ExistingRecoveryRoute::ExistingNoPrepared);
    }
    let Some(prepared) = &last.prepared else {
        return Ok(ExistingRecoveryRoute::ExistingNoPrepared);
    };
    let target = PreparedRecoveryTarget::new(
        owner.store_identity().try_clone().map_err(|source| {
            CaptureError::allocation_reservation_failed(
                "prepared selection store identity",
                None,
                source,
            )
        })?,
        slot,
        prepared.attempt.try_clone().map_err(|source| {
            CaptureError::allocation_reservation_failed(
                "prepared selection attempt id",
                None,
                source,
            )
        })?,
        digest(prepared)?,
    )
    .map_err(CaptureError::Domain)?;
    Ok(ExistingRecoveryRoute::Prepared(target))
}
