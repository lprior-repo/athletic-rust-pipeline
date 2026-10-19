use super::*;
use crate::{Receipt, Store};

impl Store {
    pub fn prepare_result_calendar_enrollment_replay_v1(
        &self,
        operation: &ResultCalendarTextV1,
        approval_digest: &ResultCalendarDigestV1,
        mut grant: AttachmentReplayGrantV1,
    ) -> StoreResult<PreparedResultCalendarEnrollmentV1<'_>> {
        let _lock = self.lock_appends();
        let (command, prior_school, approval, receipt) = guarded(&mut grant, |grant| {
            let proof = retained(self, grant)?;
            let enrollment = proof.enrollment.get().as_bytes();
            let record: super::super::wire::Enrollment =
                super::super::codec::decode(enrollment, grant)?;
            if &record.command.operation != operation || &record.approval_digest != approval_digest
            {
                return Err(Error::OperationConflict.into());
            }
            let approval = super::super::wire::enrollment_approval(&record.command, grant)?;
            let receipt: Receipt =
                super::super::codec::decode(proof.receipt.get().as_bytes(), grant)?;
            Ok((record.command, record.prior_school, approval, receipt))
        })?;
        Ok(PreparedResultCalendarEnrollmentV1 {
            revision: self.snapshot_locked().revision(),
            command,
            prior_school,
            approval,
            approval_digest: approval_digest.clone(),
            replay: Some(receipt),
            grant,
        })
    }
}

pub(super) fn require_retained(
    store: &Store,
    command: &ResultCalendarEnrollmentCommandV1,
    prior_school: &serde_json::value::RawValue,
    digest: &ResultCalendarDigestV1,
    expected_receipt: &Receipt,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Receipt> {
    let snapshot = store.db.snapshot();
    let current_school = super::super::read::school_proof(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        grant,
    )?;
    if current_school != prior_school.get().as_bytes() {
        return Err(Error::OperationConflict.into());
    }
    let proof = retained(store, grant)?;
    let record: super::super::wire::Enrollment =
        super::super::codec::decode(proof.enrollment.get().as_bytes(), grant)?;
    let receipt: Receipt = super::super::codec::decode(proof.receipt.get().as_bytes(), grant)?;
    if &record.command != command
        || record.prior_school.get() != prior_school.get()
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
    let snapshot = store.db.snapshot();
    let format = super::super::read::snapshot_format(&snapshot, &store.meta, grant)?;
    if format.lifecycle_revision != Some(LIFECYCLE) {
        return Err(Error::EnrollmentRequired.into());
    }
    super::super::read::enrollment(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        grant,
    )
}
