use super::StoreResult;

pub(super) fn pause_before_commit(
    operation: &str,
    receipt: &crate::Receipt,
    sequence: u64,
) -> StoreResult<()> {
    crate::native_worker_boundary::pause(
        crate::native_worker_boundary::Position::PublicIdentityStagedBeforeCommit { sequence },
        operation,
        &receipt.digest,
    )
}

pub(super) fn pause_after_commit(
    operation: &str,
    receipt: &crate::Receipt,
    sequence: u64,
) -> StoreResult<()> {
    crate::native_worker_boundary::pause(
        crate::native_worker_boundary::Position::PublicIdentityCommittedBeforeReturn { sequence },
        operation,
        &receipt.digest,
    )
}
