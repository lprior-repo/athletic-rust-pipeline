use super::{counter::amount, counters::ReadDebit, RecoveryReadLedger};
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::{StoreError, StoreResult};

impl AcquisitionReadBudget {
    pub(in crate::acquisition) fn for_recovery(
        roots: usize,
        encoded: usize,
        count: u64,
    ) -> StoreResult<Self> {
        Ok(Self {
            usage: super::super::AcquisitionSelectionUsage::empty(),
            roots,
            encoded,
            backing: 2_147_483_648,
            retained: 67_108_864,
            verified: std::collections::BTreeMap::new(),
            recovery: RecoveryReadLedger::planned(count)?,
        })
    }
    pub(in crate::acquisition) fn recovery_row(
        &mut self,
        key: usize,
        value: usize,
    ) -> StoreResult<()> {
        let bytes = key.checked_add(value).ok_or(StoreError::CounterOverflow)?;
        let Some(roots) = self.roots.checked_sub(1) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "roots",
                required: 1,
                limit: u64::try_from(self.roots).map_err(|_| StoreError::CounterOverflow)?,
            }
            .into());
        };
        let required = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        let available = u64::try_from(self.encoded).map_err(|_| StoreError::CounterOverflow)?;
        let Some(encoded) = self.encoded.checked_sub(bytes) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required,
                limit: available,
            }
            .into());
        };
        let recovery = self.recovery.charged(ReadDebit::Row(amount(bytes)?))?;
        let mut usage = self.usage;
        usage.row(bytes)?;
        self.roots = roots;
        self.encoded = encoded;
        self.recovery = recovery;
        self.usage = usage;
        Ok(())
    }
    pub(in crate::acquisition) fn recovery_encoded(&mut self, bytes: usize) -> StoreResult<()> {
        let required = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
        let available = u64::try_from(self.encoded).map_err(|_| StoreError::CounterOverflow)?;
        let Some(encoded) = self.encoded.checked_sub(bytes) else {
            return Err(CaptureError::ReadBudgetLimitExceeded {
                resource: "encoded bytes",
                required,
                limit: available,
            }
            .into());
        };
        let recovery = self.recovery.charged(ReadDebit::Encoded(amount(bytes)?))?;
        let mut usage = self.usage;
        usage.encoded(bytes)?;
        self.encoded = encoded;
        self.recovery = recovery;
        self.usage = usage;
        Ok(())
    }
    pub(in crate::acquisition) fn recovery_receipt_probe(&mut self) -> StoreResult<()> {
        let recovery = self.recovery.charged(ReadDebit::Receipt)?;
        let mut usage = self.usage;
        usage.receipt_probe()?;
        self.recovery = recovery;
        self.usage = usage;
        Ok(())
    }
}

impl AcquisitionReadBudget {
    #[cfg(test)]
    pub(in crate::acquisition) fn recovery_totals(&self) -> super::counters::ReadCounters {
        self.recovery.total
    }
}

#[cfg(test)]
impl AcquisitionReadBudget {
    pub(in crate::acquisition) fn recovery_narrow_physical_reads(
        &mut self,
        roots: usize,
        encoded: usize,
        receipts: u64,
    ) {
        self.roots = self.roots.min(roots);
        self.encoded = self.encoded.min(encoded);
        self.recovery.total.receipts.limit = self.recovery.total.receipts.limit.min(receipts);
    }

    pub(in crate::acquisition) fn recovery_limit_owned(&mut self, limit: u64) {
        self.recovery.total.allocations.owned.limit = limit;
    }
    pub(in crate::acquisition) fn recovery_limit_calls(&mut self, limit: u64) {
        self.recovery.total.allocations.calls.limit = limit;
    }
}
