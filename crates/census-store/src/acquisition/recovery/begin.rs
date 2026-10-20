use super::*;
use crate::acquisition::{admission, read};
use crate::{StoreError, StoreResult};
use std::sync::Arc;

impl Store {
    pub fn begin_prepared_recovery<'store, 'budget>(
        &'store self,
        owner: &crate::ResultsCaptureOwnerV1,
        target: &PreparedRecoveryTarget,
        budget: &'budget mut PreparedRecoveryBudget,
    ) -> StoreResult<PreparedRecoverySession<'store, 'budget>> {
        let mut account = budget.begin_target()?;
        account.diagnostic_bytes(512)?;
        account.reserve_owned(512)?;
        admit_recovery_owner(self, owner, target, &mut account)?;
        let (http, fence) = acquire_archive_fence(self, &mut account)?;
        let writes = self.lock_appends();
        let mut facts = read::read_work(
            &self.db.snapshot(),
            &self.journal,
            &self.receipts,
            target.attempt().work_id(),
            account.reads(),
            CaptureProtocolVersion::V2,
        )?;
        admission::matches_work(owner, &facts.work)?;
        let index = exact_target(&facts, target)?;
        account.reserve_owned(320)?;
        let target = target.try_clone().map_err(|source| {
            CaptureError::allocation_reservation_failed("prepared recovery target", None, source)
        })?;
        let prepared = facts
            .attempts
            .get_mut(index)
            .and_then(|attempt| attempt.prepared.take())
            .ok_or(CaptureError::PreparedTargetMismatch)?;
        let paths = backing::Paths::new(&http, &prepared.capture, &mut account)?;
        account.reserve_scratch()?;
        Ok(PreparedRecoverySession {
            store: self,
            writes,
            fence,
            facts,
            prepared,
            target,
            index,
            account,
            scratch: [0; 32768],
            paths,
            state: State::Uninspected,
        })
    }
}

fn admit_recovery_owner(
    store: &Store,
    owner: &crate::ResultsCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    account: &mut budget::TargetAccount<'_>,
) -> StoreResult<()> {
    if owner.protocol() != CaptureProtocolVersion::V2 {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if !Arc::ptr_eq(&owner.writer.origin, &store.appends)
        || owner.store_identity() != target.store_identity()
    {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    match account.owner_admission(store, owner)? {
        budget::OwnerAdmission::Validated => {}
        budget::OwnerAdmission::Refresh => admission::owner(store, owner, account.reads())?,
        budget::OwnerAdmission::Bind => {
            let proof = admission::validated(store, owner, account.reads())?;
            account.bind_owner(proof)?;
        }
    }
    Ok(())
}

pub(super) fn acquire_archive_fence(
    store: &Store,
    account: &mut budget::TargetAccount<'_>,
) -> StoreResult<(std::path::PathBuf, ArchiveWriteFence)> {
    account.path_bytes(
        store
            .root()
            .as_os_str()
            .len()
            .checked_add(64)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    account.reserve_owned(
        store
            .root()
            .as_os_str()
            .len()
            .checked_mul(3)
            .and_then(|bytes| bytes.checked_add(256))
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    let http = store.http_cache_dir();
    backing::safe_root(&http, account)?;
    fence::preflight(&http, account)?;
    account.diagnostic_bytes(
        http.as_os_str()
            .len()
            .checked_add(32)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    account.direct_operation(RecoveryFsOperation::Open)?;
    let fence = ArchiveWriteFence::acquire(&http)?;
    fence::verify(&fence, account)?;
    Ok((http, fence))
}

fn exact_target(facts: &WorkFacts, target: &PreparedRecoveryTarget) -> StoreResult<usize> {
    if facts.protocol != CaptureProtocolVersion::V2
        || facts.work.slot().store_identity() != target.store_identity()
        || facts.work.slot().id().map_err(CaptureError::Domain)? != *target.slot_id()
    {
        return Err(CaptureError::PreparedTargetMismatch.into());
    }
    let index = usize::from(target.attempt().ordinal().get())
        .checked_sub(1)
        .ok_or(CaptureError::PreparedTargetMismatch)?;
    let attempt = facts
        .attempts
        .get(index)
        .ok_or(CaptureError::PreparedTargetMismatch)?;
    let prepared = attempt
        .prepared
        .as_ref()
        .ok_or(CaptureError::PreparedTargetMismatch)?;
    if &attempt.reservation.attempt != target.attempt()
        || digest(prepared)? != *target.prepared_digest()
    {
        return Err(CaptureError::PreparedTargetMismatch.into());
    }
    if attempt.terminal.is_none()
        && (index.checked_add(1) != Some(facts.attempts.len()) || facts.result.is_some())
    {
        return Err(CaptureError::PreparedTargetMismatch.into());
    }
    Ok(index)
}
