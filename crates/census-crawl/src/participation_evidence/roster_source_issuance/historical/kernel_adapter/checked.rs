use crate::participation_evidence::invalid;
use census_store::StoreResult;

#[must_use]
pub(in super::super::super) struct CheckedKernelReplayV1 {
    #[cfg(test)]
    pub(super) digest: &'static str,
    #[cfg(test)]
    pub(super) bytes: u64,
    pub(super) semantic_contract: &'static str,
}

impl CheckedKernelReplayV1 {
    #[cfg(test)]
    pub(in super::super::super) fn require_reference(
        &self,
        digest: &str,
        bytes: u64,
    ) -> StoreResult<()> {
        if self.digest != digest || self.bytes != bytes {
            return Err(invalid("historical semantic kernel reference mismatch"));
        }
        Ok(())
    }

    pub(in super::super::super) fn semantic_contract(&self) -> &'static str {
        self.semantic_contract
    }

    pub(in super::super::super) fn require_semantic_contract(
        &self,
        contract: &str,
    ) -> StoreResult<()> {
        if contract != self.semantic_contract {
            return Err(invalid("historical semantic kernel owner policy mismatch"));
        }
        Ok(())
    }
}
