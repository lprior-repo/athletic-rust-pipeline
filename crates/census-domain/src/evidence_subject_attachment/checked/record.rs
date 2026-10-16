use super::*;
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct CheckedReviewedReferenceRecordV1 {
    record: ReviewedReferenceRecordV1,
    origin: Arc<()>,
}

pub fn decode_checked_reviewed_reference_with_grant(
    bytes: &[u8],
    maximum_bytes: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<CheckedReviewedReferenceRecordV1> {
    grant.admit(|grant| {
        let record = codec::decode_reviewed_reference_with_grant(bytes, maximum_bytes, grant)?;
        Ok(CheckedReviewedReferenceRecordV1 {
            record,
            origin: grant.issue_origin()?,
        })
    })
}

impl CheckedReviewedReferenceRecordV1 {
    pub fn record(&self) -> &ReviewedReferenceRecordV1 {
        &self.record
    }

    pub fn into_record(self) -> ReviewedReferenceRecordV1 {
        self.record
    }

    pub fn set_committed_at_with_grant(
        &mut self,
        committed_at: DateTime<Utc>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<()> {
        grant.admit(|grant| {
            grant.require_origin(&self.origin)?;
            grant.steps(1)?;
            if committed_at < self.record.command.reviewed_at {
                return Err(EvidenceSubjectAttachmentError::Invalid(
                    "commit precedes explicit review",
                ));
            }
            self.record.committed_at = committed_at;
            self.record.record_digest = self.record.calculated_record_digest_with_grant(grant)?;
            Ok(())
        })
    }

    pub fn encode_with_grant(
        &self,
        maximum_bytes: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<Vec<u8>> {
        grant.admit(|grant| {
            grant.require_origin(&self.origin)?;
            limit(
                "record byte grant",
                maximum_bytes,
                ReplayLimits::SMALL.total_text_bytes,
            )?;
            validate::record_shell(&self.record, grant)?;
            let needed = codec::measure_with_grant(&self.record, maximum_bytes, grant)?;
            codec::encode_measured(&self.record, needed, grant)
        })
    }

    pub fn decode_roundtrip_with_grant(
        &self,
        bytes: &[u8],
        maximum_bytes: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<Self> {
        grant.admit(|grant| {
            grant.require_origin(&self.origin)?;
            limit(
                "record byte grant",
                maximum_bytes,
                ReplayLimits::SMALL.total_text_bytes,
            )?;
            limit("record bytes", bytes.len(), maximum_bytes)?;
            grant.steps(1)?;
            let record: ReviewedReferenceRecordV1 = serde_json::from_slice(bytes)?;
            grant.steps(1)?;
            if record != self.record {
                return Err(EvidenceSubjectAttachmentError::Invalid(
                    "checked record roundtrip differs",
                ));
            }
            validate::record_shell(&record, grant)?;
            Ok(Self {
                record,
                origin: grant.issue_origin()?,
            })
        })
    }

    pub(super) fn from_parts(record: ReviewedReferenceRecordV1, origin: Arc<()>) -> Self {
        Self { record, origin }
    }

    pub(super) fn into_parts(self) -> (ReviewedReferenceRecordV1, Arc<()>) {
        (self.record, self.origin)
    }

    pub(super) fn require_origin(&self, grant: &mut AttachmentReplayGrantV1) -> Result<()> {
        grant.require_origin(&self.origin)
    }
}

impl Deref for CheckedReviewedReferenceRecordV1 {
    type Target = ReviewedReferenceRecordV1;

    fn deref(&self) -> &Self::Target {
        &self.record
    }
}
