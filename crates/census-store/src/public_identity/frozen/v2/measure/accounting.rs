use crate::{StoreError, StoreResult};

#[derive(Default)]
pub(super) struct JsonArrayBytes {
    items: usize,
    content: usize,
}

impl JsonArrayBytes {
    pub(super) fn push(&mut self, bytes: usize) -> StoreResult<()> {
        let items = self
            .items
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let content = self
            .content
            .checked_add(usize::from(self.items != 0))
            .and_then(|value| value.checked_add(bytes))
            .ok_or(StoreError::CounterOverflow)?;
        self.items = items;
        self.content = content;
        Ok(())
    }
    pub(super) fn content(&self) -> usize {
        self.content
    }
    pub(super) fn items(&self) -> usize {
        self.items
    }
}

pub(super) struct ByteBudget {
    maximum: usize,
    used: usize,
}

impl ByteBudget {
    pub(super) fn new(maximum: usize) -> Self {
        Self { maximum, used: 0 }
    }
    pub(super) fn add(&mut self, bytes: usize) -> StoreResult<()> {
        let needed = self
            .used
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        if needed > self.maximum {
            return Err(StoreError::BudgetExceeded {
                what: "frozen identity diagnostic canonical JSON bytes",
                needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
                available: u64::try_from(self.maximum).map_err(|_| StoreError::CounterOverflow)?,
            });
        }
        self.used = needed;
        Ok(())
    }
    pub(super) fn used(&self) -> usize {
        self.used
    }
    pub(super) fn remaining(&self) -> StoreResult<usize> {
        self.maximum
            .checked_sub(self.used)
            .ok_or(StoreError::CounterOverflow)
    }
}

pub(super) fn checked_sum(left: usize, right: usize) -> StoreResult<usize> {
    left.checked_add(right).ok_or(StoreError::CounterOverflow)
}

#[cfg(test)]
mod tests;
