use super::{witness, StoreResult};

pub(super) fn pause_before_commit(
    witness: &witness::Witness,
    receipt: &crate::Receipt,
    sequence: u64,
) -> StoreResult<()> {
    crate::native_worker_boundary::pause(
        crate::native_worker_boundary::Position::PublicIdentityStagedBeforeCommit { sequence },
        &witness.operation,
        &receipt.digest,
    )
}

pub(super) fn pause_after_commit(
    witness: &witness::Witness,
    receipt: &crate::Receipt,
    sequence: u64,
) -> StoreResult<()> {
    crate::native_worker_boundary::pause(
        crate::native_worker_boundary::Position::PublicIdentityCommittedBeforeReturn { sequence },
        &witness.operation,
        &receipt.digest,
    )
}
