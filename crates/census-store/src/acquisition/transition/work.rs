use super::current;
use crate::acquisition::capability::*;
use crate::acquisition::{codec, facts::*, read, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreResult};
use census_domain::model::*;
use fjall::Readable;

impl Store {
    pub fn register_acquisition_work<'owner>(
        &self,
        owner: impl Into<crate::CaptureOwnerRef<'owner>>,
        work: AcquisitionWorkV1,
        grant: RetryOwnerGrant,
    ) -> StoreResult<OwnedAcquisitionWork> {
        let owner = owner.into();
        let _writes = self.lock_appends();
        owner.verify_candidate(self, &work)?;
        let grant_owner = crate::CaptureOwnerRef::from(&grant.owner);
        grant_owner.verify(self)?;
        let slot_id = work.slot().id().map_err(CaptureError::Domain)?;
        let protocol = owner.protocol();
        let grant_slot = grant.slot.clone();
        if !grant_owner.same_owner(owner) || grant_slot != slot_id {
            return Err(CaptureError::SlotWorkMismatch.into());
        }
        let id = work.id().map_err(CaptureError::Domain)?;
        let slot_key = format!("slot/{slot_id}/binding");
        let work_key = format!("work/{id}/identity");
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let binding: Option<SlotBinding> =
            codec::get_fact(&snapshot, &self.journal, &slot_key, &mut budget, protocol)?;
        let identity: Option<AcquisitionWorkV1> =
            codec::get(&snapshot, &self.journal, &work_key, &mut budget, protocol)?;
        if binding
            .as_ref()
            .is_some_and(|binding| binding.version != protocol)
        {
            return Err(CaptureError::CaptureProtocolUnsupported.into());
        }
        if registration(
            self,
            &snapshot,
            (&binding, &identity),
            &work,
            &id,
            &mut budget,
            protocol,
        )? {
            if owner.public().is_some() {
                crate::acquisition::public_resources::read::for_work(
                    self,
                    &snapshot,
                    &work,
                    &mut budget,
                )?;
            }
            return Ok(OwnedAcquisitionWork {
                owner: owner.to_owned(),
                work,
                id,
            });
        }
        store_new_registration(self, owner, &work, protocol)?;
        Ok(OwnedAcquisitionWork {
            owner: owner.to_owned(),
            work,
            id,
        })
    }

    pub fn reserve_acquisition_attempt(
        &self,
        work: OwnedAcquisitionWork,
    ) -> StoreResult<ReservedAttempt> {
        if work.protocol() == CaptureProtocolVersion::V2 {
            return crate::acquisition::recovery::reserve(self, work);
        }
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let mut budget = AcquisitionReadBudget::for_work();
        let facts = current(self, &work, &snapshot, &mut budget)?;
        reserve_facts(self, work, &facts)
    }
}

fn store_new_registration(
    store: &Store,
    owner: crate::CaptureOwnerRef<'_>,
    work: &AcquisitionWorkV1,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    let mut batch = store.db.batch();
    stage_identity(store, &mut batch, work, protocol)?;
    crate::acquisition::public_resources::stage_entry(store, &mut batch, owner, work)?;
    codec::commit(batch)?;
    Ok(())
}

pub(in crate::acquisition) fn stage_identity(
    store: &Store,
    batch: &mut fjall::OwnedWriteBatch,
    work: &AcquisitionWorkV1,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    let id = work.id().map_err(CaptureError::Domain)?;
    let slot = work.slot().id().map_err(CaptureError::Domain)?;
    let binding = SlotBinding {
        version: protocol,
        slot: work.slot().clone(),
        work_id: id,
    };
    codec::stage(
        store,
        batch,
        &format!("slot/{slot}/binding"),
        &binding,
        protocol,
    )?;
    codec::stage(
        store,
        batch,
        &format!("work/{}/identity", binding.work_id),
        work,
        protocol,
    )?;
    Ok(())
}

fn registration(
    store: &Store,
    snapshot: &fjall::Snapshot,
    held: (&Option<SlotBinding>, &Option<AcquisitionWorkV1>),
    work: &AcquisitionWorkV1,
    id: &AcquisitionWorkId,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
) -> StoreResult<bool> {
    match held {
        (Some(binding), Some(held))
            if &binding.work_id == id && &binding.slot == work.slot() && held == work =>
        {
            read::read_work(
                snapshot,
                &store.journal,
                &store.receipts,
                id,
                budget,
                protocol,
            )?;
            codec::repeat(store)?;
            return Ok(true);
        }
        (Some(binding), _) if &binding.work_id != id => {
            return Err(CaptureError::ObligationConflict.into())
        }
        (None, None) => {
            if snapshot
                .prefix(
                    &store.journal,
                    Store::journal_key(protocol.phase(), &format!("work/{id}/")),
                )
                .next()
                .is_some()
            {
                return Err(CaptureError::SlotWorkMismatch.into());
            }
        }
        _ => return Err(CaptureError::SlotWorkMismatch.into()),
    }
    Ok(false)
}

pub(in crate::acquisition) fn reserve_facts(
    store: &Store,
    work: OwnedAcquisitionWork,
    facts: &WorkFacts,
) -> StoreResult<ReservedAttempt> {
    let ordinal = match facts.decision_view() {
        DecisionView::Ready => AttemptOrdinal::new(1).map_err(CaptureError::Domain)?,
        DecisionView::ResumeReserved(attempt) => {
            codec::repeat(store)?;
            let attempt = copy_attempt(attempt, facts.protocol)?;
            return Ok(ReservedAttempt { work, attempt });
        }
        DecisionView::RetryOwed(terminal) => terminal
            .attempt()
            .ordinal()
            .next()
            .map_err(|_| CaptureError::AcquisitionBudgetExhausted)?,
        DecisionView::Interrupted => return Err(CaptureError::RetainedOutcomeUnknown.into()),
        DecisionView::Exhausted => return Err(CaptureError::AcquisitionBudgetExhausted.into()),
        DecisionView::FinishPrepared(_) => {
            return Err(CaptureError::PreparedRecoveryRequired.into())
        }
        _ => return Err(CaptureError::RecoveryRequired.into()),
    };
    let attempt = new_attempt(&work, ordinal, facts.protocol)?;
    let reservation = Reservation {
        version: facts.protocol,
        attempt: copy_attempt(&attempt, facts.protocol)?,
    };
    let mut batch = store.db.batch();
    codec::stage(
        store,
        &mut batch,
        &format!("{}/reserved", read::attempt_prefix(&attempt)),
        &reservation,
        facts.protocol,
    )?;
    codec::commit(batch)?;
    Ok(ReservedAttempt { work, attempt })
}
fn new_attempt(
    work: &OwnedAcquisitionWork,
    ordinal: AttemptOrdinal,
    protocol: CaptureProtocolVersion,
) -> StoreResult<AcquisitionAttemptId> {
    let identity = match protocol {
        CaptureProtocolVersion::V1 => work.work.slot().store_identity().clone(),
        CaptureProtocolVersion::V2 => work
            .work
            .slot()
            .store_identity()
            .try_clone()
            .map_err(allocation)?,
    };
    let id = match protocol {
        CaptureProtocolVersion::V1 => work.id.clone(),
        CaptureProtocolVersion::V2 => work.id.try_clone().map_err(allocation)?,
    };
    Ok(AcquisitionAttemptId::new(identity, id, ordinal))
}
fn copy_attempt(
    attempt: &AcquisitionAttemptId,
    protocol: CaptureProtocolVersion,
) -> StoreResult<AcquisitionAttemptId> {
    match protocol {
        CaptureProtocolVersion::V1 => Ok(attempt.clone()),
        CaptureProtocolVersion::V2 => attempt.try_clone().map_err(allocation),
    }
}
fn allocation(source: std::collections::TryReserveError) -> crate::StoreError {
    CaptureError::allocation_reservation_failed("acquisition attempt identity", None, source).into()
}
