use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PublicParentReadUsage {
    rows: u64,
    encoded_bytes: u64,
    receipt_probes: u64,
    present_receipts: u64,
}

impl PublicParentReadUsage {
    pub fn rows(self) -> u64 {
        self.rows
    }
    pub fn encoded_bytes(self) -> u64 {
        self.encoded_bytes
    }
    pub fn receipt_probes(self) -> u64 {
        self.receipt_probes
    }
    pub fn present_receipts(self) -> u64 {
        self.present_receipts
    }

    pub(super) fn row(&mut self, key: usize, value: usize) -> StoreResult<()> {
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        self.encoded(key.checked_add(value).ok_or(StoreError::CounterOverflow)?)
    }

    pub(super) fn encoded(&mut self, bytes: usize) -> StoreResult<()> {
        self.encoded_bytes = self
            .encoded_bytes
            .checked_add(u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }

    pub(super) fn receipt_probe(&mut self) -> StoreResult<()> {
        self.receipt_probes = self
            .receipt_probes
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }

    pub(super) fn present_receipt(&mut self) -> StoreResult<()> {
        self.present_receipts = self
            .present_receipts
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }
}
