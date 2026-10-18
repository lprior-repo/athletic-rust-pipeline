use super::{AffiliationCandidateError, LocallyCheckedEvidence};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OwnerSourceSpan {
    from: usize,
    through_exclusive: usize,
}

impl OwnerSourceSpan {
    pub const MAX_BYTES: usize = 2 * 1024 * 1024;

    pub fn new(from: usize, through_exclusive: usize) -> Result<Self, AffiliationCandidateError> {
        let length = through_exclusive
            .checked_sub(from)
            .ok_or(AffiliationCandidateError::InvalidLocator)?;
        if length == 0 || length > Self::MAX_BYTES {
            return Err(AffiliationCandidateError::InvalidLocator);
        }
        Ok(Self {
            from,
            through_exclusive,
        })
    }

    pub fn range(self) -> std::ops::Range<usize> {
        self.from..self.through_exclusive
    }

    pub fn read<'a>(
        self,
        evidence: &LocallyCheckedEvidence<'a>,
    ) -> Result<&'a [u8], AffiliationCandidateError> {
        evidence
            .raw()
            .body
            .get(self.from..self.through_exclusive)
            .ok_or(AffiliationCandidateError::InvalidLocator)
    }
}

impl std::fmt::Debug for OwnerSourceSpan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourceSpan")
            .field("from", &self.from)
            .field("through_exclusive", &self.through_exclusive)
            .finish()
    }
}

#[cfg(test)]
mod tests;
