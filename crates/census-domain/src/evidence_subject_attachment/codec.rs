use super::*;
use serde::Serialize;

mod digests;
mod measure;

pub(super) use digests::digest;

#[cfg(test)]
pub(super) use measure::measure;

pub fn decode_reviewed_reference(
    bytes: &[u8],
    maximum_bytes: usize,
) -> Result<ReviewedReferenceRecordV1> {
    decode_reviewed_reference_with_grant(bytes, maximum_bytes, &mut AttachmentReplayGrantV1::new())
}

pub fn decode_reviewed_reference_with_grant(
    bytes: &[u8],
    maximum_bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ReviewedReferenceRecordV1> {
    grant.admit(|grant| {
        limit(
            "record byte grant",
            maximum_bytes,
            ReplayLimits::SMALL.total_text_bytes,
        )?;
        limit("record bytes", bytes.len(), maximum_bytes)?;
        grant.steps(1)?;
        let record = serde_json::from_slice(bytes)?;
        validate::record(&record, grant)?;
        Ok(record)
    })
}

pub fn encode_reviewed_reference(
    record: &ReviewedReferenceRecordV1,
    maximum_bytes: usize,
) -> Result<Vec<u8>> {
    encode_reviewed_reference_with_grant(record, maximum_bytes, &mut AttachmentReplayGrantV1::new())
}

pub fn encode_reviewed_reference_with_grant(
    record: &ReviewedReferenceRecordV1,
    maximum_bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    grant.admit(|grant| {
        limit(
            "record byte grant",
            maximum_bytes,
            ReplayLimits::SMALL.total_text_bytes,
        )?;
        validate::record(record, grant)?;
        let needed = measure_with_grant(record, maximum_bytes, grant)?;
        encode_measured(record, needed, grant)
    })
}

pub(super) fn measure_with_grant(
    value: &impl Serialize,
    maximum: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<usize> {
    grant.steps(1)?;
    measure::measure(value, maximum)
}

fn encode_measured(
    value: &impl Serialize,
    needed: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    grant.steps(1)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(needed)
        .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
    crate::model::write_serialized(&mut bytes, value)?;
    if bytes.len() != needed {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "encoding size changed",
        ));
    }
    Ok(bytes)
}

impl RetainedEvidenceSubjectAttachmentCommandV1 {
    pub fn approval_bytes(&self, maximum_bytes: usize) -> Result<Vec<u8>> {
        self.approval_bytes_with_grant(maximum_bytes, &mut AttachmentReplayGrantV1::new())
    }

    pub fn approval_bytes_with_grant(
        &self,
        maximum_bytes: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<Vec<u8>> {
        grant.admit(|grant| {
            limit(
                "approval byte grant",
                maximum_bytes,
                ReplayLimits::SMALL.total_text_bytes,
            )?;
            validate::command(self, grant)?;
            let value = Approval {
                schema: APPROVAL_SCHEMA,
                actor: self.actor,
                prepared: self,
            };
            let needed = measure_with_grant(&value, maximum_bytes, grant)?;
            grant.bytes(needed)?;
            encode_measured(&value, needed, grant)
        })
    }
}

#[derive(Serialize)]
struct Approval<'a> {
    schema: &'static str,
    actor: AttachmentReviewActorV1,
    prepared: &'a RetainedEvidenceSubjectAttachmentCommandV1,
}

#[cfg(test)]
mod tests;
