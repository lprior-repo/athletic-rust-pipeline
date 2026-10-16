use super::{limit, EvidenceSubjectAttachmentError, ReplayLimits, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentReplayUsageV1 {
    encoded_bytes: usize,
    traversal_steps: usize,
}

impl AttachmentReplayUsageV1 {
    pub fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
    pub fn traversal_steps(&self) -> usize {
        self.traversal_steps
    }
}

#[derive(Debug)]
enum Admission {
    Open,
    Poisoned,
}

#[derive(Debug)]
pub struct AttachmentReplayGrantV1 {
    usage: AttachmentReplayUsageV1,
    admission: Admission,
}

impl Default for AttachmentReplayGrantV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl AttachmentReplayGrantV1 {
    pub fn new() -> Self {
        Self {
            usage: AttachmentReplayUsageV1 {
                encoded_bytes: 0,
                traversal_steps: 0,
            },
            admission: Admission::Open,
        }
    }

    pub fn usage(&self) -> AttachmentReplayUsageV1 {
        self.usage
    }

    pub fn poison(&mut self) {
        self.admission = Admission::Poisoned;
    }

    pub fn ensure_available(&self) -> Result<()> {
        match self.admission {
            Admission::Open => Ok(()),
            Admission::Poisoned => Err(EvidenceSubjectAttachmentError::Poisoned),
        }
    }

    pub fn charge_bytes(&mut self, bytes: usize) -> Result<()> {
        self.admit(|grant| {
            let needed = grant
                .usage
                .encoded_bytes
                .checked_add(bytes)
                .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
            limit(
                "aggregate encoded bytes",
                needed,
                ReplayLimits::SMALL.total_text_bytes,
            )?;
            grant.usage.encoded_bytes = needed;
            Ok(())
        })
    }

    pub fn charge_steps(&mut self, steps: usize) -> Result<()> {
        self.admit(|grant| {
            let needed = grant
                .usage
                .traversal_steps
                .checked_add(steps)
                .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
            limit(
                "aggregate traversal steps",
                needed,
                ReplayLimits::SMALL.traversal_steps,
            )?;
            grant.usage.traversal_steps = needed;
            Ok(())
        })
    }

    pub(super) fn bytes(&mut self, bytes: usize) -> Result<()> {
        self.charge_bytes(bytes)
    }
    pub(super) fn steps(&mut self, steps: usize) -> Result<()> {
        self.charge_steps(steps)
    }

    pub(super) fn admit<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.ensure_available()?;
        let result = operation(self);
        if result.is_err() {
            self.poison();
        }
        result
    }
}
