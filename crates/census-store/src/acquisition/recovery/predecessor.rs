use super::*;
use crate::acquisition::{admission, read, OwnedAcquisitionWork, ReservedAttempt};
use crate::StoreResult;

pub(in crate::acquisition) fn reserve(
    store: &Store,
    work: OwnedAcquisitionWork,
) -> StoreResult<ReservedAttempt> {
    let mut budget = PreparedRecoveryBudget::predecessor()?;
    let mut account = budget.begin_target()?;
    admission::owner(store, &work.owner, account.reads())?;
    account.path_bytes(
        store
            .root()
            .as_os_str()
            .len()
            .checked_add(64)
            .ok_or(crate::StoreError::CounterOverflow)?,
    )?;
    account.reserve_owned(
        store
            .root()
            .as_os_str()
            .len()
            .checked_mul(3)
            .and_then(|bytes| bytes.checked_add(256))
            .ok_or(crate::StoreError::CounterOverflow)?,
    )?;
    let http = store.http_cache_dir();
    backing::safe_root(&http, &mut account)?;
    fence::preflight(&http, &mut account)?;
    account.diagnostic_bytes(
        http.as_os_str()
            .len()
            .checked_add(32)
            .ok_or(crate::StoreError::CounterOverflow)?,
    )?;
    account.direct_operation(RecoveryFsOperation::Open)?;
    let fence = ArchiveWriteFence::acquire(&http)?;
    fence::verify(&fence, &mut account)?;
    let _writes = store.lock_appends();
    let facts = read::read_work(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        &work.id,
        account.reads(),
        CaptureProtocolVersion::V2,
    )?;
    if facts.work != work.work {
        return Err(CaptureError::ObligationConflict.into());
    }
    account.reserve_scratch()?;
    let mut scratch = [0; 32768];
    for attempt in facts.attempts.iter().take(2) {
        if !attempt.terminal.as_ref().is_some_and(|terminal| {
            matches!(
                terminal.terminal(),
                AcquisitionTerminal::PreparedRetentionFailed { .. }
            )
        }) {
            continue;
        }
        let prepared = attempt
            .prepared
            .as_ref()
            .ok_or(CaptureError::CorruptClosure)?;
        let paths = backing::Paths::new(&http, &prepared.capture, &mut account)?;
        if paths.body_missing(&mut account)?.is_none() {
            backing::verify(
                &paths.body,
                prepared.capture.content_bytes(),
                prepared.capture.content_digest(),
                RecoveryPayloadRole::Body,
                &mut account,
                &mut scratch,
            )?;
        }
        if paths.metadata_missing(&mut account)?.is_none() {
            backing::verify(
                &paths.metadata,
                u64::try_from(prepared.capture.metadata_bytes().len())
                    .map_err(|_| crate::StoreError::CounterOverflow)?,
                prepared.capture.metadata_digest(),
                RecoveryPayloadRole::Metadata,
                &mut account,
                &mut scratch,
            )?;
        }
    }
    account.reserve_owned(3 * MAX_CAPTURE_RECORD_BYTES)?;
    crate::acquisition::transition::reserve_facts(store, work, &facts)
}
