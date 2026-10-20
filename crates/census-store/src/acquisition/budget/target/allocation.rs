use super::{
    counter::{product, Counter},
    limits,
};
use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy)]
pub(in crate::acquisition) enum AllocationKind {
    Owned,
    Body,
    Scratch,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::acquisition) struct Allocations {
    pub(in crate::acquisition) owned: Counter,
    pub(in crate::acquisition) body: Counter,
    pub(in crate::acquisition) scratch: Counter,
    pub(in crate::acquisition) calls: Counter,
    pub(in crate::acquisition) live_owned: u64,
    pub(in crate::acquisition) live_body: u64,
    pub(in crate::acquisition) live_scratch: u64,
    pub(in crate::acquisition) peak: u64,
}
impl Allocations {
    pub(in crate::acquisition) fn new(count: u64) -> StoreResult<Self> {
        Ok(Self {
            owned: Counter::new(product(count, limits::NONBODY)?),
            body: Counter::new(product(count, limits::BODY)?),
            scratch: Counter::new(product(count, limits::SCRATCH)?),
            calls: Counter::new(u64::MAX),
            live_owned: 0,
            live_body: 0,
            live_scratch: 0,
            peak: 0,
        })
    }
    pub(in crate::acquisition) const fn selection() -> Self {
        Self {
            owned: Counter::new(limits::NONBODY),
            body: Counter::new(0),
            scratch: Counter::new(0),
            calls: Counter::new(u64::MAX),
            live_owned: 0,
            live_body: 0,
            live_scratch: 0,
            peak: 0,
        }
    }
    pub(in crate::acquisition) const fn empty() -> Self {
        Self {
            owned: Counter::new(0),
            body: Counter::new(0),
            scratch: Counter::new(0),
            calls: Counter::new(u64::MAX),
            live_owned: 0,
            live_body: 0,
            live_scratch: 0,
            peak: 0,
        }
    }
    pub(in crate::acquisition) fn reserved(
        mut self,
        kind: AllocationKind,
        bytes: u64,
    ) -> StoreResult<Self> {
        match kind {
            AllocationKind::Owned => {
                self.owned = self.owned.charged(bytes, "allocation")?;
                self.live_owned = self
                    .live_owned
                    .checked_add(bytes)
                    .ok_or(StoreError::CounterOverflow)?;
            }
            AllocationKind::Body => {
                self.body = self.body.charged(bytes, "body allocation")?;
                self.live_body = self
                    .live_body
                    .checked_add(bytes)
                    .ok_or(StoreError::CounterOverflow)?;
            }
            AllocationKind::Scratch => {
                self.scratch = self.scratch.charged(bytes, "scratch allocation")?;
                self.live_scratch = self
                    .live_scratch
                    .checked_add(bytes)
                    .ok_or(StoreError::CounterOverflow)?;
            }
        }
        self.calls = self.calls.charged(1, "allocation calls")?;
        self.peak = self.peak.max(self.live()?);
        Ok(self)
    }
    pub(in crate::acquisition) fn released(mut self, bytes: u64) -> StoreResult<Self> {
        self.live_owned = self.live_owned.checked_sub(bytes).ok_or(
            CaptureError::AllocationAccountingInvariant {
                resource: "live allocation",
            },
        )?;
        Ok(self)
    }
    pub(in crate::acquisition) fn live(self) -> StoreResult<u64> {
        self.live_owned
            .checked_add(self.live_body)
            .and_then(|bytes| bytes.checked_add(self.live_scratch))
            .ok_or(StoreError::CounterOverflow)
    }
    pub(in crate::acquisition) fn remove_live(&mut self, target: Self) -> StoreResult<()> {
        let owned = self.live_owned.checked_sub(target.live_owned);
        let body = self.live_body.checked_sub(target.live_body);
        let scratch = self.live_scratch.checked_sub(target.live_scratch);
        match (owned, body, scratch) {
            (Some(owned), Some(body), Some(scratch)) => {
                self.live_owned = owned;
                self.live_body = body;
                self.live_scratch = scratch;
                Ok(())
            }
            _ => Err(CaptureError::AllocationAccountingInvariant {
                resource: "live allocation",
            }
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AllocationKind, Allocations};
    use crate::{acquisition::CaptureError, StoreError};

    #[test]
    fn reservation_limits_and_real_reservation_failures_are_distinct() {
        let over_limit = Allocations::selection()
            .reserved(AllocationKind::Owned, u64::MAX)
            .expect_err("logical cap must refuse before an allocation attempt");
        assert!(matches!(
            over_limit,
            StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: "allocation",
                required: u64::MAX,
                limit,
            }) if limit < u64::MAX
        ));

        let mut requested = Vec::<u8>::new();
        let source = requested
            .try_reserve_exact(usize::MAX)
            .expect_err("capacity overflow provides a deterministic reservation failure");
        let error =
            CaptureError::allocation_reservation_failed("test buffer", Some(usize::MAX), source);
        assert!(matches!(
            error,
            CaptureError::AllocationReservationFailed {
                resource: "test buffer",
                requested_bytes: Some(usize::MAX),
                ..
            }
        ));
    }

    #[test]
    fn accounting_underflow_is_not_a_budget_refusal() {
        let error = Allocations::empty()
            .released(1)
            .expect_err("releasing unowned bytes is an invariant violation");
        assert!(matches!(
            error,
            StoreError::Capture(CaptureError::AllocationAccountingInvariant {
                resource: "live allocation",
            })
        ));
    }
}
