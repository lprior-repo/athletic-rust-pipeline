use std::num::NonZeroU64;
use std::sync::Mutex;

use super::{ConstructionStorage, HistoryFunding};

mod grant;
pub use grant::HistoryMemoryGrant;

pub trait HeldHistoryMemory {
    fn reserved_bytes(&self) -> NonZeroU64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HistoryFundingError {
    #[error("history host envelope has insufficient memory")]
    NoBytes,
    #[error("history host envelope ownership mismatch")]
    ForeignOwner,
    #[error("history host envelope arithmetic overflow")]
    Overflow,
    #[error("history host envelope is poisoned")]
    Poisoned,
    #[error("history host envelope accounting is invalid")]
    InvalidAccounting,
    #[error("history grant split must leave a nonzero remainder")]
    InvalidSplit,
}

enum Health {
    Healthy,
    Poisoned,
    InvalidAccounting,
}

struct BudgetState {
    available: u64,
    live: u64,
    health: Health,
}

pub struct HistoryMemoryBudget<P> {
    state: Mutex<BudgetState>,
    _parent: P,
}

impl<P: HeldHistoryMemory> HistoryMemoryBudget<P> {
    pub fn new(parent: P) -> Self {
        let total = parent.reserved_bytes().get();
        Self {
            state: Mutex::new(BudgetState {
                available: total,
                live: 0,
                health: Health::Healthy,
            }),
            _parent: parent,
        }
    }

    pub fn funding(&self) -> HistoryBudgetFunding<'_> {
        HistoryBudgetFunding { state: &self.state }
    }
}

pub struct HistoryBudgetFunding<'a> {
    state: &'a Mutex<BudgetState>,
}

impl<'a> HistoryFunding for HistoryBudgetFunding<'a> {
    type Error = HistoryFundingError;
    type Grant = HistoryMemoryGrant<'a>;

    fn reserve(&mut self, request: ConstructionStorage) -> Result<Self::Grant, Self::Error> {
        let bytes = request.reserved_bytes();
        let mut state = self
            .state
            .lock()
            .map_err(|_| HistoryFundingError::Poisoned)?;
        match state.health {
            Health::Healthy => {}
            Health::Poisoned => return Err(HistoryFundingError::Poisoned),
            Health::InvalidAccounting => return Err(HistoryFundingError::InvalidAccounting),
        }
        let available = state
            .available
            .checked_sub(bytes.get())
            .ok_or(HistoryFundingError::NoBytes)?;
        let live = state
            .live
            .checked_add(bytes.get())
            .ok_or(HistoryFundingError::Overflow)?;
        state.available = available;
        state.live = live;
        drop(state);
        Ok(HistoryMemoryGrant {
            state: self.state,
            bytes,
            charge: grant::Charge::Held,
        })
    }

    fn ensure_owner(&self, grant: &Self::Grant) -> Result<(), Self::Error> {
        if std::ptr::eq(grant.state, self.state) {
            Ok(())
        } else {
            Err(HistoryFundingError::ForeignOwner)
        }
    }
}

#[cfg(test)]
mod tests;
