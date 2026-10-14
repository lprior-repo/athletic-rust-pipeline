use super::{diagnostic, replace, shell};
use crate::net::{FetchError, RecoveryCleanupLeaf};
use census_store::{RecoveryArchiveIo, RecoveryPayloadRole};
use std::path::Path;

#[derive(Clone, Copy)]
pub(in crate::net) enum Primary {
    Create,
    StoreBudget,
    WouldBlock,
    InvalidData,
}

pub(in crate::net) fn actual_original_and_cleanup(
    path: &Path,
    anchor: &Path,
    kind: Primary,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    diagnostic::admit_error_boxes(io)?;
    let original = match kind {
        Primary::Create => shell::create(path, io).map(|_| ()),
        Primary::StoreBudget => io
            .payload(RecoveryPayloadRole::Body, 2_147_483_649)
            .map_err(Into::into),
        Primary::WouldBlock => replace::lock(anchor, io).map(|_| ()),
        Primary::InvalidData => shell::regular(anchor, io).map(|_| ()),
    };
    let cleanup = shell::sync_directory(path, io);
    diagnostic::with_cleanup(anchor, original, cleanup, io)
}

pub(in crate::net) fn retains_both_typed_causes(error: &FetchError) -> bool {
    match error {
        FetchError::Cache { source, .. } => {
            let Some(causes) = source
                .get_ref()
                .and_then(|inner| inner.downcast_ref::<diagnostic::CacheCleanupFailure>())
            else {
                return false;
            };
            matches!(
                causes.cleanup.first(),
                Some(Some(RecoveryCleanupLeaf::Cache { .. }))
            )
        }
        FetchError::RecoveryCleanup(error) => matches!(
            error.cleanup().first(),
            Some(Some(RecoveryCleanupLeaf::Cache { .. }))
        ),
        _ => false,
    }
}

pub(in crate::net) fn retains_primary_and_diagnostic_refusal(error: &FetchError) -> bool {
    let FetchError::Cache { source, .. } = error else {
        return false;
    };
    let Some(causes) = source
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<diagnostic::CacheCleanupFailure>())
    else {
        return false;
    };
    matches!(causes.cleanup.first(), Some(Some(RecoveryCleanupLeaf::Store(error))) if matches!(**error, census_store::StoreError::Capture(census_store::CaptureError::ReadBudgetExceeded { resource: "diagnostic bytes" })))
}

pub(in crate::net) fn third_cleanup(
    path: &Path,
    anchor: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    diagnostic::admit_error_boxes(io)?;
    let mut original = shell::regular(anchor, io).map(|_| ());
    for _ in 0..3 {
        let cleanup = shell::sync_directory(path, io);
        original = diagnostic::with_cleanup(anchor, original, cleanup, io);
    }
    original
}

pub(in crate::net) fn fallback_store_boxes(
    path: &Path,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    diagnostic::admit_error_boxes(io)?;
    for shift in (0..=13).rev() {
        let bytes = 1_usize
            .checked_shl(shift)
            .ok_or(census_store::StoreError::CounterOverflow)?;
        match io.diagnostic_bytes(bytes) {
            Ok(()) => {}
            Err(census_store::StoreError::Capture(
                census_store::CaptureError::ReadBudgetExceeded {
                    resource: "diagnostic bytes",
                },
            )) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let original = io
        .payload(RecoveryPayloadRole::Body, 2_147_483_649)
        .map_err(Into::into);
    let cleanup = io
        .payload(RecoveryPayloadRole::Body, 2_147_483_649)
        .map_err(Into::into);
    let result = diagnostic::with_cleanup(path, original, cleanup, io);
    match result {
        Err(error) => super::stage::fail(path, error, io),
        Ok(()) => Err(census_store::CaptureError::TransitionRefused.into()),
    }
}

pub(in crate::net) fn deplete_requested_owned(
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    for shift in (0..=26).rev() {
        let bytes = 1_usize
            .checked_shl(shift)
            .ok_or(census_store::StoreError::CounterOverflow)?;
        match io.reserve_owned(bytes) {
            Ok(()) => io.release_owned(bytes)?,
            Err(census_store::StoreError::Capture(
                census_store::CaptureError::ReadBudgetExceeded {
                    resource: "allocation",
                },
            )) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(in crate::net) fn entry_admission_after_depletion(
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    deplete_requested_owned(io)?;
    diagnostic::admit_error_boxes(io)
}
