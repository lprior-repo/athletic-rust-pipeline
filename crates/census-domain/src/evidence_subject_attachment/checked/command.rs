use super::*;

#[derive(Debug)]
pub struct CheckedAttachmentCommandV1 {
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    origin: Arc<()>,
}

pub fn check_attachment_command_with_grant(
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<CheckedAttachmentCommandV1> {
    grant.admit(|grant| {
        validate::command(&command, grant)?;
        Ok(CheckedAttachmentCommandV1 {
            command,
            origin: grant.issue_origin()?,
        })
    })
}

impl CheckedAttachmentCommandV1 {
    pub fn command(&self) -> &RetainedEvidenceSubjectAttachmentCommandV1 {
        &self.command
    }

    pub fn into_command(self) -> RetainedEvidenceSubjectAttachmentCommandV1 {
        self.command
    }

    pub fn approval_bytes_with_grant(
        &self,
        maximum_bytes: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<Vec<u8>> {
        grant.admit(|grant| {
            grant.require_origin(&self.origin)?;
            limit(
                "approval byte grant",
                maximum_bytes,
                ReplayLimits::SMALL.total_text_bytes,
            )?;
            codec::approval_bytes(&self.command, maximum_bytes, grant)
        })
    }

    pub fn into_draft_with_grant(
        self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<CheckedReviewedReferenceRecordV1> {
        grant.admit(|grant| {
            grant.require_origin(&self.origin)?;
            let prepared_digest = self.command.prepared_digest_with_grant(grant)?;
            let approval_digest = self.command.approval_digest_with_grant(grant)?;
            let committed_at = self.command.reviewed_at;
            let mut record = ReviewedReferenceRecordV1 {
                command: self.command,
                prepared_digest,
                approval_digest,
                committed_at,
                record_digest: AttachmentDigestV1::parse(&"0".repeat(64))?,
            };
            record.record_digest = record.calculated_record_digest_with_grant(grant)?;
            Ok(CheckedReviewedReferenceRecordV1::from_parts(
                record,
                self.origin,
            ))
        })
    }
}

impl Deref for CheckedAttachmentCommandV1 {
    type Target = RetainedEvidenceSubjectAttachmentCommandV1;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}
