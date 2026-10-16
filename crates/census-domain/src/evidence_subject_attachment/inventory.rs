use super::*;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

pub(super) fn inventory(
    records: &[ReviewedReferenceRecordV1],
    grant: &mut validate::Grant,
) -> Result<AttachmentInventoryV1> {
    limit("records", records.len(), ReplayLimits::SMALL.records)?;
    grant.steps(records.len())?;
    Ok(AttachmentInventoryV1 {
        version: 1,
        records: u64::try_from(records.len())
            .map_err(|_| EvidenceSubjectAttachmentError::Overflow)?,
        digest: codec::digest(&(INVENTORY_SCHEMA, References(records)), grant)?,
    })
}

pub fn calculate_reviewed_reference_inventory_with_grant(
    records: &[ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<AttachmentInventoryV1> {
    grant.admit(|grant| inventory(records, grant))
}

struct References<'a>(&'a [ReviewedReferenceRecordV1]);

impl Serialize for References<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for record in self.0 {
            sequence.serialize_element(&Reference {
                operation: &record.command.operation,
                digest: &record.record_digest,
            })?;
        }
        sequence.end()
    }
}

#[derive(Serialize)]
struct Reference<'a> {
    operation: &'a AttachmentTextV1,
    digest: &'a AttachmentDigestV1,
}

impl AttachmentInventoryV1 {
    pub fn empty() -> Result<Self> {
        inventory(&[], &mut validate::Grant::new())
    }
}
