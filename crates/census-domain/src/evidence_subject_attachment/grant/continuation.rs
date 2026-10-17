use super::{AttachmentReplayGrantV1, AttachmentReplayUsageV1};
use crate::evidence_subject_attachment::{EvidenceSubjectAttachmentError, Result};
use std::sync::Arc;

#[derive(Debug)]
pub struct AttachmentReplayContinuationV1 {
    origin: Arc<()>,
    floor: AttachmentReplayUsageV1,
}

impl AttachmentReplayGrantV1 {
    pub fn checkpoint(&mut self) -> Result<AttachmentReplayContinuationV1> {
        let origin = self.issue_origin()?;
        Ok(AttachmentReplayContinuationV1 { origin, floor: self.usage() })
    }
}

impl AttachmentReplayContinuationV1 {
    pub fn require(&self, grant: &mut AttachmentReplayGrantV1) -> Result<()> {
        grant.admit(|grant| {
            let usage = grant.usage();
            if usage.encoded_bytes() < self.floor.encoded_bytes()
                || usage.traversal_steps() < self.floor.traversal_steps()
            {
                return Err(EvidenceSubjectAttachmentError::Invalid("grant continuation usage regressed"));
            }
            grant.require_origin(&self.origin)
        })
    }
}

#[cfg(test)]
mod tests;
