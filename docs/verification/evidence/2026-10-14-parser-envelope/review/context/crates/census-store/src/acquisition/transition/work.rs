use super::current;
use crate::acquisition::capability::*;
use crate::acquisition::{codec, facts::*, read, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreResult};
use census_domain::model::*;
use fjall::Readable;

impl Store {
    pub fn register_acquisition_work(
        &self,
        owner: &ResultsCaptureOwnerV1,
        work: AcquisitionWorkV1,
        grant: RetryOwnerGrant,
    ) -> StoreResult<OwnedAcquisitionWork> {
        let _writes = self.lock_appends();
        owner.verify_work(self, &work)?;
        grant.owner.verify(self)?;
        let slot_id = work.slot().id().map_err(CaptureError::Domain)?;
        if grant.owner.scope != owner.scope || grant.slot != slot_id {
            return Err(CaptureError::SlotWorkMismatch.into());
        }
        let id = work.id().map_err(CaptureError::Domain)?;
        let slot_key = format!("slot/{slot_id}/binding");
        let work_key = format!("work/{id}/identity");
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let binding: Option<SlotBinding> =
            codec::get(&snapshot, &self.journal, &slot_key, &mut budget)?;
        let identity: Option<AcquisitionWorkV1> =
            codec::get(&snapshot, &self.journal, &work_key, &mut budget)?;
        match (&binding, &identity) {
            (Some(binding), Some(held))
                if binding.work_id == id && &binding.slot == work.slot() && held == &work =>
            {
                read::read_work(&snapshot, &self.journal, &self.receipts, &id, &mut budget)?;
                codec::repeat(self)?;
                return Ok(OwnedAcquisitionWork {
                    owner: owner.clone(),
                    work,
                    id,
                });
            }
            (Some(binding), _) if binding.work_id != id => {
                return Err(CaptureError::ObligationConflict.into())
            }
            (None, None) => {
                if snapshot
                    .prefix(
                        &self.journal,
                        Store::journal_key(SOURCE_ACQUISITION_PHASE, &format!("work/{id}/")),
                    )
                    .next()
                    .is_some()
                {
                    return Err(CaptureError::SlotWorkMismatch.into());
                }
            }
            _ => return Err(CaptureError::SlotWorkMismatch.into()),
        }
        let binding = SlotBinding {
            version: VersionOne,
            slot: work.slot().clone(),
            work_id: id.clone(),
        };
        let mut batch = self.db.batch();
        codec::stage(self, &mut batch, &slot_key, &binding)?;
        codec::stage(self, &mut batch, &work_key, &work)?;
        codec::commit(batch)?;
        Ok(OwnedAcquisitionWork {
            owner: owner.clone(),
            work,
            id,
        })
    }

    pub fn reserve_acquisition_attempt(
        &self,
        work: OwnedAcquisitionWork,
    ) -> StoreResult<ReservedAttempt> {
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &work, &snapshot, &mut budget)?;
        let ordinal = match facts.decision() {
            AcquireDecision::Ready => AttemptOrdinal::new(1).map_err(CaptureError::Domain)?,
            AcquireDecision::ResumeReserved(attempt) => {
                codec::repeat(self)?;
                return Ok(ReservedAttempt { work, attempt });
            }
            AcquireDecision::RetryOwed(terminal) => terminal
                .attempt()
                .ordinal()
                .next()
                .map_err(|_| CaptureError::AcquisitionBudgetExhausted)?,
            AcquireDecision::Interrupted => return Err(CaptureError::RetainedOutcomeUnknown.into()),
            AcquireDecision::Exhausted => {
                return Err(CaptureError::AcquisitionBudgetExhausted.into())
            }
            AcquireDecision::FinishPrepared(_) => {
                return Err(CaptureError::PreparedRecoveryRequired.into())
            }
            _ => return Err(CaptureError::RecoveryRequired.into()),
        };
        let attempt = AcquisitionAttemptId::new(
            work.work.slot().store_identity().clone(),
            work.id.clone(),
            ordinal,
        );
        let reservation = Reservation {
            version: VersionOne,
            attempt: attempt.clone(),
        };
        let mut batch = self.db.batch();
        codec::stage(
            self,
            &mut batch,
            &format!("{}/reserved", read::attempt_prefix(&attempt)),
            &reservation,
        )?;
        codec::commit(batch)?;
        Ok(ReservedAttempt { work, attempt })
    }
}
