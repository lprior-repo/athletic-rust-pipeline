use super::*;

mod chains;
mod owned;

pub use owned::{
    decode_and_replay_reviewed_references, decode_and_replay_reviewed_references_with_grant,
    OwnedReviewedReferenceProjectionV1,
};

#[derive(Debug)]
pub struct ReviewedReferenceProjectionV1<'a> {
    records: &'a [ReviewedReferenceRecordV1],
    states: Vec<chains::State>,
    inventory: AttachmentInventoryV1,
}

impl<'a> ReviewedReferenceProjectionV1<'a> {
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.inventory
    }
    pub fn records(&self) -> &'a [ReviewedReferenceRecordV1] {
        self.records
    }
    pub fn active_records(&self) -> impl Iterator<Item = &'a ReviewedReferenceRecordV1> + '_ {
        self.records
            .iter()
            .zip(&self.states)
            .filter_map(|(record, state)| matches!(state, chains::State::Active).then_some(record))
    }
}

pub fn replay_reviewed_references<'a>(
    store: &StoreIdentity,
    records: &'a [ReviewedReferenceRecordV1],
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
) -> Result<ReviewedReferenceProjectionV1<'a>> {
    replay_reviewed_references_with_grant(
        store,
        records,
        receipts,
        expected_inventory,
        &mut AttachmentReplayGrantV1::new(),
    )
}

pub fn replay_reviewed_references_with_grant<'a>(
    store: &StoreIdentity,
    records: &'a [ReviewedReferenceRecordV1],
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ReviewedReferenceProjectionV1<'a>> {
    grant.admit(|grant| {
        limit("records", records.len(), ReplayLimits::SMALL.records)?;
        if records.len() != receipts.len() {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "record and receipt count",
            ));
        }
        for (record, receipt) in records.iter().zip(receipts) {
            validate_record(store, record, receipt, grant)?;
        }
        project(records, expected_inventory, grant)
    })
}

pub fn replay_checked_reviewed_references_with_grant(
    store: &StoreIdentity,
    records: &super::checked::CheckedReviewedReferenceRecordsV1,
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    project_checked_reviewed_references_with_grant(
        store,
        records,
        receipts,
        expected_inventory,
        grant,
    )
    .map(|_| ())
}

pub fn project_checked_reviewed_references_with_grant<'a>(
    store: &StoreIdentity,
    records: &'a super::checked::CheckedReviewedReferenceRecordsV1,
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ReviewedReferenceProjectionV1<'a>> {
    grant.admit(|grant| {
        limit("records", records.len(), ReplayLimits::SMALL.records)?;
        if records.len() != receipts.len() {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "record and receipt count",
            ));
        }
        records.require_origin(grant)?;
        for (record, receipt) in records.iter().zip(receipts) {
            validate::record_shell(record, grant)?;
            validate_receipt(store, record, receipt, grant)?;
        }
        project(records, expected_inventory, grant)
    })
}

fn project<'a>(
    records: &'a [ReviewedReferenceRecordV1],
    expected_inventory: &AttachmentInventoryV1,
    grant: &mut validate::Grant,
) -> Result<ReviewedReferenceProjectionV1<'a>> {
    let mut states = Vec::new();
    states
        .try_reserve_exact(records.len())
        .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
    for (index, record) in records.iter().enumerate() {
        let prefix = records
            .get(..index)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        if record.command.expected_attachment_inventory != inventory::inventory(prefix, grant)? {
            return Err(EvidenceSubjectAttachmentError::Conflict(
                "attachment inventory prefix",
            ));
        }
        chains::accept(record, prefix, &mut states, grant)?;
    }
    let inventory = inventory::inventory(records, grant)?;
    if &inventory != expected_inventory {
        return Err(EvidenceSubjectAttachmentError::Conflict(
            "complete attachment inventory",
        ));
    }
    Ok(ReviewedReferenceProjectionV1 {
        records,
        states,
        inventory,
    })
}

fn validate_record(
    store: &StoreIdentity,
    record: &ReviewedReferenceRecordV1,
    receipt: &AttachmentReceiptBindingV1,
    grant: &mut validate::Grant,
) -> Result<()> {
    validate::record(record, grant)?;
    validate_receipt(store, record, receipt, grant)
}

fn validate_receipt(
    store: &StoreIdentity,
    record: &ReviewedReferenceRecordV1,
    receipt: &AttachmentReceiptBindingV1,
    grant: &mut validate::Grant,
) -> Result<()> {
    let bytes = codec::measure_with_grant(receipt, ReplayLimits::SMALL.total_text_bytes, grant)?;
    grant.bytes(bytes)?;
    grant.steps(1)?;
    if &record.command.store != store
        || receipt.operation != record.command.operation
        || receipt.record_digest != record.record_digest
    {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "record origin or receipt binding",
        ));
    }
    Ok(())
}
