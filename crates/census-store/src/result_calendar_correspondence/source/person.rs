use super::*;
use census_domain::evidence_subject_attachment::{
    replay_reviewed_references_with_grant, AttachmentReceiptBindingV1,
};

pub(super) fn originals<'a>(
    source: &ResultCalendarSourceClaimV1,
    records: &'a [ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(&'a ReviewedReferenceRecordV1, &'a ReviewedReferenceRecordV1)> {
    let count =
        usize::try_from(source.attachment_inventory.records).map_err(|_| Error::BudgetExceeded)?;
    let prefix = records.get(..count).ok_or(Error::SourceMismatch(
        "original attachment inventory absent",
    ))?;
    let store = &prefix
        .first()
        .ok_or(Error::SourceMismatch("empty original attachment inventory"))?
        .command
        .store;
    grant
        .charge_bytes(
            count
                .checked_mul(std::mem::size_of::<AttachmentReceiptBindingV1>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    grant
        .charge_steps(count.checked_mul(2).ok_or(Error::BudgetExceeded)?)
        .map_err(grant_error)?;
    let receipts = receipt_bindings(prefix)?;
    let projection = replay_reviewed_references_with_grant(
        store,
        prefix,
        &receipts,
        &source.attachment_inventory,
        grant,
    )
    .map_err(grant_error)?;
    let result = projection
        .active_records()
        .find(|record| {
            record.command.operation == source.result_attachment.operation
                && record.record_digest == source.result_attachment.digest
                && record.command.target == source.person
        })
        .ok_or(Error::SourceMismatch(
            "result was not active in original inventory",
        ))?;
    let documentary = projection
        .active_records()
        .find(|record| {
            record.command.operation == source.documentary_attachment.operation
                && record.record_digest == source.documentary_attachment.digest
                && record.command.target == source.person
        })
        .ok_or(Error::SourceMismatch(
            "documentary source was not active in original inventory",
        ))?;
    Ok((result, documentary))
}

fn receipt_bindings(
    records: &[ReviewedReferenceRecordV1],
) -> StoreResult<Vec<AttachmentReceiptBindingV1>> {
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(records.len())
        .map_err(|_| Error::BudgetExceeded)?;
    for record in records {
        receipts.push(AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        });
    }
    Ok(receipts)
}
