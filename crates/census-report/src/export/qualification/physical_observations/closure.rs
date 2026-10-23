use super::*;
use sha2::{Digest, Sha256};

pub(super) struct Closure {
    hasher: Sha256,
    previous: Vec<u8>,
    rows: u64,
    bytes: u64,
    budget: ReadBudget,
}

impl Closure {
    pub(super) fn new(table: Table, budget: ReadBudget) -> StoreResult<Self> {
        admission(table, budget)?;
        let mut hasher = Sha256::new();
        hasher.update(table.file().as_bytes());
        hasher.update([0]);
        Ok(Self {
            hasher,
            previous: Vec::new(),
            rows: 0,
            bytes: 0,
            budget,
        })
    }

    pub(super) fn push(&mut self, key: &[u8], raw: &[u8]) -> StoreResult<()> {
        if key.is_empty() || key.len() > census_store::MAX_ID_BYTES + 64 || raw.is_empty() {
            return Err(invalid(
                "physical observation exceeds its key or record bound",
            ));
        }
        if raw.len() > self.budget.record_bytes {
            return Err(StoreError::BudgetExceeded {
                what: "physical proof record bytes",
                needed: u64::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)?,
                available: u64::try_from(self.budget.record_bytes)
                    .map_err(|_| StoreError::CounterOverflow)?,
            });
        }
        if !self.previous.is_empty() && key <= self.previous.as_slice() {
            return Err(invalid(
                "physical observation keys are duplicated or out of order",
            ));
        }
        self.rows = charge(self.rows, 1, self.budget.rows, "physical proof rows")?;
        let bytes = key
            .len()
            .checked_add(raw.len())
            .ok_or(StoreError::CounterOverflow)?;
        self.bytes = charge(
            self.bytes,
            u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?,
            self.budget.bytes,
            "physical proof bytes",
        )?;
        self.hasher.update(key);
        self.hasher.update([0x1f]);
        self.hasher.update(raw);
        self.hasher.update([0x1e]);
        self.previous.clear();
        self.previous.extend_from_slice(key);
        Ok(())
    }

    pub(super) fn finish(self) -> (ReadCounts, String) {
        (
            ReadCounts {
                key_visits: self.rows,
                decoded_rows: self.rows,
                encoded_bytes: self.bytes,
                merged_rows: 0,
            },
            format!("{:x}", self.hasher.finalize()),
        )
    }
}

fn charge(current: u64, added: u64, maximum: u64, what: &'static str) -> StoreResult<u64> {
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > maximum {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available: maximum,
        });
    }
    Ok(needed)
}

#[cfg(test)]
mod tests;
