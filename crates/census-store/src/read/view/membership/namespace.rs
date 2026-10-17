use crate::{StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES};

#[derive(Debug, Clone, Copy)]
pub struct JournalNamespace<'a> {
    phase: &'a str,
    logical_prefix: &'a str,
}

enum PrefixRequirement {
    Nonempty,
    WholePhase,
}

impl<'a> JournalNamespace<'a> {
    pub fn new(phase: &'a str, logical_prefix: &'a str) -> StoreResult<Self> {
        Self::checked(phase, logical_prefix, PrefixRequirement::Nonempty)
    }

    pub(crate) fn whole_phase(phase: &'a str) -> StoreResult<Self> {
        Self::checked(phase, "", PrefixRequirement::WholePhase)
    }

    fn checked(
        phase: &'a str,
        logical_prefix: &'a str,
        requirement: PrefixRequirement,
    ) -> StoreResult<Self> {
        let length = phase
            .len()
            .checked_add(logical_prefix.len())
            .and_then(|bytes| bytes.checked_add(1))
            .ok_or(StoreError::CounterOverflow)?;
        if length > MAX_JOURNAL_KEY_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: length,
                max: MAX_JOURNAL_KEY_BYTES,
            });
        }
        validate_text(phase)?;
        match requirement {
            PrefixRequirement::Nonempty => validate_text(logical_prefix)?,
            PrefixRequirement::WholePhase => {}
        }
        if phase.as_bytes() == crate::journal::OBJECT_NAMESPACE {
            return Err(StoreError::Refused {
                detail: "shared object storage is not a logical journal namespace".to_string(),
            });
        }
        Ok(Self {
            phase,
            logical_prefix,
        })
    }

    pub fn phase(self) -> &'a str {
        self.phase
    }

    pub fn logical_prefix(self) -> &'a str {
        self.logical_prefix
    }

    pub(crate) fn physical_prefix(self) -> StoreResult<Vec<u8>> {
        let mut prefix = Vec::new();
        prefix
            .try_reserve_exact(MAX_JOURNAL_KEY_BYTES)
            .map_err(|_| allocation_refused())?;
        prefix.extend_from_slice(self.phase.as_bytes());
        prefix.push(0);
        prefix.extend_from_slice(self.logical_prefix.as_bytes());
        Ok(prefix)
    }

    pub(super) fn logical_suffix(self, physical: &[u8]) -> StoreResult<&[u8]> {
        let suffix = physical
            .strip_prefix(self.phase.as_bytes())
            .and_then(|suffix| suffix.strip_prefix(&[0]))
            .filter(|suffix| suffix.starts_with(self.logical_prefix.as_bytes()));
        suffix.ok_or_else(|| self.refusal("physical key is outside its namespace"))
    }

    pub(super) fn refusal(self, detail: &'static str) -> StoreError {
        StoreError::JournalDescriptor {
            phase: self.phase.to_string(),
            key: self.logical_prefix.to_string(),
            detail: detail.to_string(),
        }
    }
}

pub(super) fn validate_text(text: &str) -> StoreResult<()> {
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(StoreError::Refused {
            detail: "journal namespace and logical keys must be nonempty control-free text"
                .to_string(),
        });
    }
    Ok(())
}

pub(super) fn allocation_refused() -> StoreError {
    StoreError::Refused {
        detail: "journal namespace scratch allocation refused".to_string(),
    }
}

#[cfg(test)]
mod tests;
