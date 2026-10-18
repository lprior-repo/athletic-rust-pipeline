use super::*;
use census_domain::evidence_subject_attachment::{
    replay_reviewed_references_with_grant, AttachmentReceiptBindingV1,
};

pub(super) fn original<'a>(
    source: &ResultSchoolSourceClaimV1,
    records: &'a [ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a ReviewedReferenceRecordV1> {
    let count =
        usize::try_from(source.person_inventory.records).map_err(|_| Error::BudgetExceeded)?;
    let prefix = records
        .get(..count)
        .ok_or(Error::SourceMismatch("original person inventory absent"))?;
    let store = &prefix
        .first()
        .ok_or(Error::SourceMismatch("empty original person inventory"))?
        .command
        .store;
    grant
        .charge_bytes(
            count
                .checked_mul(std::mem::size_of::<AttachmentReceiptBindingV1>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(count)
        .map_err(|_| Error::BudgetExceeded)?;
    prefix.iter().try_for_each(|record| {
        grant.charge_steps(1).map_err(grant_error)?;
        receipts.push(AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        });
        Ok::<_, StoreError>(())
    })?;
    let projection = replay_reviewed_references_with_grant(
        store,
        prefix,
        &receipts,
        &source.person_inventory,
        grant,
    )
    .map_err(grant_error)?;
    let record = projection
        .active_records()
        .find(|record| {
            record.command.operation == source.person_attachment.operation
                && record.record_digest == source.person_attachment.digest
                && record.command.target == source.person
        })
        .ok_or(Error::SourceMismatch(
            "person was not active in original inventory",
        ))?;
    Ok(record)
}
