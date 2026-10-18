use super::*;
use crate::{Receipt, Store};

impl Store {
    pub fn prepare_result_school_enrollment_replay_v1(
        &self,
        operation: &ResultSchoolTextV1,
        approval_digest: &ResultSchoolDigestV1,
        mut grant: AttachmentReplayGrantV1,
    ) -> StoreResult<PreparedResultSchoolEnrollmentV1<'_>> {
        let _lock = self.lock_appends();
        let (command, approval, receipt) = guarded(&mut grant, |grant| {
            let proof = retained(self, grant)?;
            let record: super::super::wire::Enrollment =
                super::super::codec::decode(&proof.enrollment, grant)?;
            if &record.command.operation != operation || &record.approval_digest != approval_digest
            {
                return Err(Error::OperationConflict.into());
            }
            let approval = super::super::wire::enrollment_approval(&record.command, grant)?;
            let receipt = super::super::codec::decode(&proof.receipt, grant)?;
            Ok((record.command, approval, receipt))
        })?;
        Ok(PreparedResultSchoolEnrollmentV1 {
            revision: self.snapshot_locked().revision(),
            command,
            approval,
            approval_digest: approval_digest.clone(),
            replay: Some(receipt),
            grant,
        })
    }
}

pub(super) fn require_retained(
    store: &Store,
    command: &ResultSchoolEnrollmentCommandV1,
    digest: &ResultSchoolDigestV1,
    expected_receipt: &Receipt,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Receipt> {
    let proof = retained(store, grant)?;
    let record: super::super::wire::Enrollment =
        super::super::codec::decode(&proof.enrollment, grant)?;
    let receipt: Receipt = super::super::codec::decode(&proof.receipt, grant)?;
    if &record.command != command
        || &record.approval_digest != digest
        || super::super::codec::encode(&receipt, grant)?
            != super::super::codec::encode(expected_receipt, grant)?
    {
        return Err(Error::OperationConflict.into());
    }
    Ok(receipt)
}

fn retained(
    store: &Store,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<super::super::wire::Proof> {
    super::super::read::load(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &store.store_format()?,
        grant,
    )?
    .ok_or_else(|| Error::EnrollmentRequired.into())
}
