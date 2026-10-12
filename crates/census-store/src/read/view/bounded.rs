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

impl StoreSnapshot<'_> {
    pub fn for_each_merged_bounded<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        mut visit: impl FnMut(T, u64) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        let mut counts = ReadCounts::default();
        let mut current = None;
        let prefix = self.layout().prefix(table);
        for guard in self.snapshot.prefix(self.entities, &prefix) {
            counts.key_visits = charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            if raw.len() > budget.record_bytes {
                return Err(StoreError::RowTooLarge {
                    bytes: raw.len(),
                    max: budget.record_bytes,
                });
            }
            let bytes = raw
                .len()
                .checked_add(key.len())
                .ok_or(StoreError::CounterOverflow)?;
            let bytes = u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?;
            counts.encoded_bytes = charge(
                counts.encoded_bytes,
                bytes,
                budget.bytes,
                "capture encoded bytes",
            )?;
            let row = merge::decode(table, &key, &raw)?;
            counts.decoded_rows = counts
                .decoded_rows
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            let mut publish = |row| visit(row, counts.encoded_bytes);
            counts.merged_rows =
                merge::accept(&mut current, row, &mut publish, counts.merged_rows)?;
        }
        if let Some(row) = current {
            let mut publish = |row| visit(row, counts.encoded_bytes);
            counts.merged_rows = merge::publish(row, &mut publish, counts.merged_rows)?;
        }
        Ok(counts)
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
