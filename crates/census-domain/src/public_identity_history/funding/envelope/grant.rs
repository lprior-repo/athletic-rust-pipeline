use std::num::NonZeroU64;

use super::{BudgetState, Health, HistoryFundingError};
use crate::public_identity_history::funding::HistoryGrant;

pub(super) enum Charge {
    Held,
    Transferred,
}

pub struct HistoryMemoryGrant<'a> {
    pub(super) state: &'a std::sync::Mutex<BudgetState>,
    pub(super) bytes: NonZeroU64,
    pub(super) charge: Charge,
}

impl HistoryGrant for HistoryMemoryGrant<'_> {
    type Error = HistoryFundingError;

    fn reserved_bytes(&self) -> NonZeroU64 {
        self.bytes
    }

    fn same_owner(&self, other: &Self) -> bool {
        std::ptr::eq(self.state, other.state)
    }

    fn try_absorb(&mut self, mut other: Self) -> Result<(), Self::Error> {
        if !self.same_owner(&other) {
            return Err(HistoryFundingError::ForeignOwner);
        }
        let bytes = self
            .bytes
            .get()
            .checked_add(other.bytes.get())
            .and_then(NonZeroU64::new)
            .ok_or(HistoryFundingError::Overflow)?;
        self.bytes = bytes;
        other.charge = Charge::Transferred;
        Ok(())
    }

    fn try_split(&mut self, bytes: NonZeroU64) -> Result<Self, Self::Error> {
        let remaining = self
            .bytes
            .get()
            .checked_sub(bytes.get())
            .and_then(NonZeroU64::new)
            .ok_or(HistoryFundingError::InvalidSplit)?;
        self.bytes = remaining;
        Ok(Self {
            state: self.state,
            bytes,
            charge: Charge::Held,
        })
    }
}

impl super::HeldHistoryMemory for HistoryMemoryGrant<'_> {
    fn reserved_bytes(&self) -> NonZeroU64 {
        self.bytes
    }
}

impl Drop for HistoryMemoryGrant<'_> {
    fn drop(&mut self) {
        if matches!(self.charge, Charge::Transferred) {
            return;
        }
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                if matches!(state.health, Health::Healthy) {
                    state.health = Health::Poisoned;
                }
                state
            }
        };
        let Some(available) = state.available.checked_add(self.bytes.get()) else {
            state.health = Health::InvalidAccounting;
            return;
        };
        let Some(live) = state.live.checked_sub(self.bytes.get()) else {
            state.health = Health::InvalidAccounting;
            return;
        };
        state.available = available;
        state.live = live;
        self.charge = Charge::Transferred;
    }
}
