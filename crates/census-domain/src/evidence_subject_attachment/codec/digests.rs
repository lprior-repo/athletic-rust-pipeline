use super::*;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

impl RetainedEvidenceSubjectAttachmentCommandV1 {
    pub fn prepared_digest(&self) -> Result<AttachmentDigestV1> {
        self.prepared_digest_with_grant(&mut AttachmentReplayGrantV1::new())
    }

    pub fn prepared_digest_with_grant(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<AttachmentDigestV1> {
        grant.admit(|grant| digest(&(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1, self), grant))
    }

    pub fn approval_digest(&self) -> Result<AttachmentDigestV1> {
        self.approval_digest_with_grant(&mut AttachmentReplayGrantV1::new())
    }

    pub fn approval_digest_with_grant(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<AttachmentDigestV1> {
        grant.admit(|grant| {
            let value = Approval {
                schema: APPROVAL_SCHEMA,
                actor: self.actor,
                prepared: self,
            };
            measure_with_grant(&value, ReplayLimits::SMALL.total_text_bytes, grant)?;
            grant.steps(1)?;
            let mut writer = DigestWriter(Sha256::new());
            writer.0.update(APPROVAL_SCHEMA.as_bytes());
            writer.0.update([0]);
            crate::model::write_serialized(&mut writer, &value)?;
            AttachmentDigestV1::parse(&format!("{:x}", writer.0.finalize()))
        })
    }
}

impl ReviewedReferenceRecordV1 {
    pub fn calculated_record_digest(&self) -> Result<AttachmentDigestV1> {
        self.calculated_record_digest_with_grant(&mut AttachmentReplayGrantV1::new())
    }

    pub fn calculated_record_digest_with_grant(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<AttachmentDigestV1> {
        grant.admit(|grant| {
            digest(
                &(
                    RECORD_SCHEMA,
                    &self.command,
                    &self.prepared_digest,
                    &self.approval_digest,
                    self.committed_at,
                ),
                grant,
            )
        })
    }

    pub fn reference(&self) -> AttachmentRecordReferenceV1 {
        AttachmentRecordReferenceV1 {
            operation: self.command.operation.clone(),
            digest: self.record_digest.clone(),
        }
    }
}

pub(in super::super) fn digest(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<AttachmentDigestV1> {
    measure_with_grant(value, ReplayLimits::SMALL.total_text_bytes, grant)?;
    grant.steps(1)?;
    AttachmentDigestV1::of(value)
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
