use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy)]
pub(in crate::acquisition) struct Counter {
    pub(in crate::acquisition) limit: u64,
    pub(in crate::acquisition) used: u64,
}
impl Counter {
    pub(in crate::acquisition) const fn new(limit: u64) -> Self {
        Self { limit, used: 0 }
    }
    pub(in crate::acquisition) fn charged(
        self,
        amount: u64,
        resource: &'static str,
    ) -> StoreResult<Self> {
        let used = self
            .used
            .checked_add(amount)
            .ok_or(StoreError::CounterOverflow)?;
        if used > self.limit {
            return Err(CaptureError::ReadBudgetExceeded { resource }.into());
        }
        Ok(Self {
            limit: self.limit,
            used,
        })
    }
    pub(in crate::acquisition) fn require(
        self,
        amount: u64,
        resource: &'static str,
    ) -> StoreResult<()> {
        self.charged(amount, resource).map(|_| ())
    }
}

pub(in crate::acquisition) fn product(count: u64, unit: u64) -> StoreResult<u64> {
    count.checked_mul(unit).ok_or(StoreError::CounterOverflow)
}

pub(in crate::acquisition) fn amount(bytes: usize) -> StoreResult<u64> {
    u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)
}
