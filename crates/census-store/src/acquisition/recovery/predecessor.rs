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
    let (http, _fence) = begin::acquire_archive_fence(store, &mut account)?;
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
    facts
        .attempts
        .iter()
        .take(2)
        .try_for_each(|attempt| -> StoreResult<()> {
            verify_failed_retention(attempt, &http, &mut account, &mut scratch)
        })?;
    account.reserve_owned(3 * MAX_CAPTURE_RECORD_BYTES)?;
    crate::acquisition::transition::reserve_facts(store, work, &facts)
}

fn verify_failed_retention(
    attempt: &AttemptFacts,
    http: &Path,
    account: &mut budget::TargetAccount<'_>,
    scratch: &mut [u8; 32768],
) -> StoreResult<()> {
    if !attempt.terminal.as_ref().is_some_and(|terminal| {
        matches!(
            terminal.terminal(),
            AcquisitionTerminal::PreparedRetentionFailed { .. }
        )
    }) {
        return Ok(());
    }
    let prepared = attempt
        .prepared
        .as_ref()
        .ok_or(CaptureError::CorruptClosure)?;
    let paths = backing::Paths::new(http, &prepared.capture, account)?;
    if paths.body_missing(account)?.is_none() {
        backing::verify(
            &paths.body,
            prepared.capture.content_bytes(),
            prepared.capture.content_digest(),
            RecoveryPayloadRole::Body,
            account,
            scratch,
        )?;
    }
    if paths.metadata_missing(account)?.is_none() {
        backing::verify(
            &paths.metadata,
            u64::try_from(prepared.capture.metadata_bytes().len())
                .map_err(|_| crate::StoreError::CounterOverflow)?,
            prepared.capture.metadata_digest(),
            RecoveryPayloadRole::Metadata,
            account,
            scratch,
        )?;
    }
    Ok(())
}
