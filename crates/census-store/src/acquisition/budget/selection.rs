use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcquisitionSelectionUsage {
    roots: u64,
    encoded_bytes: u64,
    receipt_probes: u64,
    present_receipts: u64,
    targets_examined: u64,
    absent_target_probes: u64,
}
impl AcquisitionSelectionUsage {
    pub fn new(
        roots: u64,
        encoded_bytes: u64,
        receipt_probes: u64,
        present_receipts: u64,
        targets_examined: u64,
        absent_target_probes: u64,
    ) -> StoreResult<Self> {
        let existing = targets_examined.checked_sub(absent_target_probes).ok_or(
            CaptureError::ReadBudgetExceeded {
                resource: "selection usage",
            },
        )?;
        let minimum_roots = existing.checked_mul(2).ok_or(StoreError::CounterOverflow)?;
        let maximum_roots = existing
            .checked_mul(42)
            .ok_or(StoreError::CounterOverflow)?;
        let maximum_probes = existing.checked_mul(3).ok_or(StoreError::CounterOverflow)?;
        let maximum_encoded = roots
            .checked_mul(69_632)
            .and_then(|bytes| {
                present_receipts
                    .checked_mul(66_048)
                    .and_then(|receipts| bytes.checked_add(receipts))
            })
            .ok_or(StoreError::CounterOverflow)?;
        if roots > 1_048_576 {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "roots",
                required: roots,
                limit: 1_048_576,
            }
            .into());
        }
        if encoded_bytes > 536_870_912 {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required: encoded_bytes,
                limit: 536_870_912,
            }
            .into());
        }
        if targets_examined > 65_536 {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "selection targets",
                required: targets_examined,
                limit: 65_536,
            }
            .into());
        }
        if roots < minimum_roots
            || roots > maximum_roots
            || receipt_probes > maximum_probes
            || present_receipts > receipt_probes
            || encoded_bytes > maximum_encoded
            || (roots != 0 && encoded_bytes == 0)
        {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "selection usage",
            }
            .into());
        }
        Ok(Self {
            roots,
            encoded_bytes,
            receipt_probes,
            present_receipts,
            targets_examined,
            absent_target_probes,
        })
    }
    pub fn roots(self) -> u64 {
        self.roots
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
    pub fn targets_examined(self) -> u64 {
        self.targets_examined
    }
    pub fn absent_target_probes(self) -> u64 {
        self.absent_target_probes
    }
    pub(super) fn empty() -> Self {
        Self {
            roots: 0,
            encoded_bytes: 0,
            receipt_probes: 0,
            present_receipts: 0,
            targets_examined: 0,
            absent_target_probes: 0,
        }
    }
    pub(super) fn row(&mut self, bytes: usize) -> StoreResult<()> {
        self.roots = self
            .roots
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        self.encoded(bytes)
    }
    pub(super) fn encoded(&mut self, bytes: usize) -> StoreResult<()> {
        self.encoded_bytes = self
            .encoded_bytes
            .checked_add(u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }
    pub(super) fn target_examined(&mut self) -> StoreResult<()> {
        if self.targets_examined == 65_536 {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "selection targets",
                required: 65_537,
                limit: 65_536,
            }
            .into());
        }
        self.targets_examined = self
            .targets_examined
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }
    pub(super) fn absent_target(&mut self) -> StoreResult<()> {
        self.absent_target_probes = self
            .absent_target_probes
            .checked_add(1)
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
