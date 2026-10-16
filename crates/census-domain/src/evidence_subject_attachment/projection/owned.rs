use super::*;

#[derive(Debug)]
pub struct OwnedReviewedReferenceProjectionV1 {
    records: Vec<ReviewedReferenceRecordV1>,
    states: Vec<chains::State>,
    inventory: AttachmentInventoryV1,
}

impl OwnedReviewedReferenceProjectionV1 {
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.inventory
    }
    pub fn records(&self) -> &[ReviewedReferenceRecordV1] {
        &self.records
    }
    pub fn into_records(self) -> Vec<ReviewedReferenceRecordV1> {
        self.records
    }
    pub fn active_records(&self) -> impl Iterator<Item = &ReviewedReferenceRecordV1> {
        self.records
            .iter()
            .zip(&self.states)
            .filter_map(|(record, state)| matches!(state, chains::State::Active).then_some(record))
    }
}

pub fn decode_and_replay_reviewed_references(
    store: &StoreIdentity,
    raw_records: &[&[u8]],
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
    maximum_record_bytes: usize,
) -> Result<OwnedReviewedReferenceProjectionV1> {
    decode_and_replay_reviewed_references_with_grant(
        store,
        raw_records,
        receipts,
        expected_inventory,
        maximum_record_bytes,
        &mut AttachmentReplayGrantV1::new(),
    )
}

pub fn decode_and_replay_reviewed_references_with_grant(
    store: &StoreIdentity,
    raw_records: &[&[u8]],
    receipts: &[AttachmentReceiptBindingV1],
    expected_inventory: &AttachmentInventoryV1,
    maximum_record_bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<OwnedReviewedReferenceProjectionV1> {
    grant.admit(|grant| {
        limit(
            "record byte grant",
            maximum_record_bytes,
            ReplayLimits::SMALL.total_text_bytes,
        )?;
        limit("records", raw_records.len(), ReplayLimits::SMALL.records)?;
        if raw_records.len() != receipts.len() {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "record and receipt count",
            ));
        }
        for bytes in raw_records {
            limit("record bytes", bytes.len(), maximum_record_bytes)?;
            grant.bytes(bytes.len())?;
            grant.steps(1)?;
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(raw_records.len())
            .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
        for (bytes, receipt) in raw_records.iter().zip(receipts) {
            grant.steps(1)?;
            let record = serde_json::from_slice(bytes)?;
            validate_record(store, &record, receipt, grant)?;
            records.push(record);
        }
        let ReviewedReferenceProjectionV1 {
            states, inventory, ..
        } = project(&records, expected_inventory, grant)?;
        Ok(OwnedReviewedReferenceProjectionV1 {
            records,
            states,
            inventory,
        })
    })
}
