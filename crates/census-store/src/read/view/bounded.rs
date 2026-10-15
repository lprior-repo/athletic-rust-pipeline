use crate::{Entity, StoreError, StoreResult, Table};
use fjall::Readable;

use super::{merge, StoreSnapshot};

#[derive(Debug, Clone, Copy)]
pub struct ReadBudget {
    pub rows: u64,
    pub bytes: u64,
    pub record_bytes: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadCounts {
    pub key_visits: u64,
    pub decoded_rows: u64,
    pub encoded_bytes: u64,
    pub merged_rows: u64,
}

pub fn decode_physical_observation<T: Entity>(
    table: Table,
    key: &[u8],
    raw: &[u8],
) -> StoreResult<T> {
    merge::decode(table, key, raw)
}

impl StoreSnapshot<'_> {
    pub fn for_each_merged_bounded<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        mut visit: impl FnMut(T, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        let mut current = None;
        let mut merged_rows = 0;
        let mut counts =
            self.for_each_observation_bounded(table, budget, |row, encoded_bytes| {
                let mut publish = |row| visit(row, encoded_bytes);
                merged_rows = merge::accept(&mut current, row, &mut publish, merged_rows)?;
                Ok(())
            })?;
        if let Some(row) = current {
            let mut publish = |row| visit(row, counts.encoded_bytes);
            merged_rows = merge::publish(row, &mut publish, merged_rows)?;
        }
        counts.merged_rows = merged_rows;
        Ok(counts)
    }

    pub fn for_each_observation_bounded<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        mut visit: impl FnMut(T, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        self.for_each_physical_observation_bounded(table, budget, |row, _, _, bytes| {
            visit(row, bytes)
        })
    }

    pub fn for_each_physical_observation_bounded<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        mut visit: impl FnMut(T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        let mut counts = ReadCounts::default();
        let prefix = self.layout().prefix(table);
        for guard in self.snapshot.prefix(self.entities, &prefix) {
            counts.key_visits = charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            let size = self
                .snapshot
                .size_of(self.entities, &key)
                .map_err(|source| StoreError::Read { source })?;
            let raw = read_admitted(key.len(), size, &mut counts, budget, || {
                self.snapshot
                    .get(self.entities, &key)
                    .map_err(|source| StoreError::Read { source })
            })?;
            let row = decode_physical_observation(table, &key, &raw)?;
            counts.decoded_rows = counts
                .decoded_rows
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            visit(row, &key, &raw, counts.encoded_bytes)?;
        }
        Ok(counts)
    }
}

fn read_admitted(
    key_bytes: usize,
    size: Option<u32>,
    counts: &mut ReadCounts,
    budget: ReadBudget,
    load: impl FnOnce() -> StoreResult<Option<fjall::UserValue>>,
) -> StoreResult<fjall::UserValue> {
    let size = size.ok_or_else(|| invariant("bounded observation size is missing"))?;
    let size = usize::try_from(size).map_err(|_| StoreError::CounterOverflow)?;
    if size > budget.record_bytes {
        return Err(StoreError::RowTooLarge {
            bytes: size,
            max: budget.record_bytes,
        });
    }
    let bytes = size
        .checked_add(key_bytes)
        .ok_or(StoreError::CounterOverflow)?;
    let bytes = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
    counts.encoded_bytes = charge(
        counts.encoded_bytes,
        bytes,
        budget.bytes,
        "capture encoded bytes",
    )?;
    let raw = load()?.ok_or_else(|| invariant("admitted bounded observation is missing"))?;
    if raw.len() != size {
        return Err(invariant("admitted bounded observation size changed"));
    }
    Ok(raw)
}

fn invariant(detail: &'static str) -> StoreError {
    StoreError::Invariant {
        detail: detail.into(),
    }
}

fn charge(current: u64, added: u64, available: u64, what: &'static str) -> StoreResult<u64> {
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        });
    }
    Ok(needed)
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "physical_observation_tests.rs"]
mod physical_observation_tests;
