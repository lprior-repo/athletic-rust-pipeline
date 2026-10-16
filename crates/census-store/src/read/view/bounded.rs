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
        Ok(self
            .scan_physical_observations(table, budget, PhysicalScan::Whole, &mut visit)?
            .counts)
    }

    pub(super) fn scan_physical_observations<T: Entity>(
        &self,
        table: Table,
        budget: ReadBudget,
        mode: PhysicalScan<'_>,
        mut visit: impl FnMut(T, &[u8], &[u8], u64) -> StoreResult<()>,
    ) -> StoreResult<PhysicalScanResult> {
        let mut counts = ReadCounts::default();
        let prefix = self.layout().prefix(table);
        let rows = mode.rows(self, &prefix);
        for guard in rows {
            if matches!(mode, PhysicalScan::Whole) {
                charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            }
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if !key.starts_with(&prefix) {
                break;
            }
            if matches!(mode, PhysicalScan::Page(_))
                && counts.key_visits == budget.rows
                && counts.decoded_rows > 0
            {
                return Ok(PhysicalScanResult {
                    counts,
                    next_key: Some(key),
                });
            }
            let mut admitted = counts;
            admitted.key_visits =
                charge(counts.key_visits, 1, budget.rows, "capture physical rows")?;
            let raw = self.load_physical_observation(&key, &mut admitted, budget);
            let raw = match raw {
                Err(error) if mode.can_continue(&error, counts, budget) => {
                    return Ok(PhysicalScanResult {
                        counts,
                        next_key: Some(key),
                    });
                }
                result => result?,
            };
            let row = decode_physical_observation(table, &key, &raw)?;
            admitted.decoded_rows = counts
                .decoded_rows
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            visit(row, &key, &raw, admitted.encoded_bytes)?;
            counts = admitted;
        }
        Ok(PhysicalScanResult {
            counts,
            next_key: None,
        })
    }

    fn load_physical_observation(
        &self,
        key: &[u8],
        counts: &mut ReadCounts,
        budget: ReadBudget,
    ) -> StoreResult<fjall::UserValue> {
        let size = self
            .snapshot
            .size_of(self.entities, key)
            .map_err(|source| StoreError::Read { source })?;
        read_admitted(key.len(), size, counts, budget, || {
            self.snapshot
                .get(self.entities, key)
                .map_err(|source| StoreError::Read { source })
        })
    }
}

pub(super) enum PhysicalScan<'a> {
    Whole,
    Page(Option<&'a [u8]>),
}

pub(super) struct PhysicalScanResult {
    pub counts: ReadCounts,
    pub next_key: Option<fjall::UserKey>,
}

impl PhysicalScan<'_> {
    fn rows(&self, snapshot: &StoreSnapshot<'_>, prefix: &[u8]) -> fjall::Iter {
        match self {
            Self::Page(Some(start)) => snapshot.snapshot.range::<&[u8], _>(
                snapshot.entities,
                (
                    std::ops::Bound::Included(*start),
                    std::ops::Bound::Unbounded,
                ),
            ),
            Self::Whole | Self::Page(None) => snapshot.snapshot.prefix(snapshot.entities, prefix),
        }
    }

    fn can_continue(&self, error: &StoreError, counts: ReadCounts, budget: ReadBudget) -> bool {
        matches!(self, Self::Page(_))
            && counts.decoded_rows > 0
            && matches!(error, StoreError::BudgetExceeded {
                what: "capture encoded bytes", needed, ..
            } if needed.checked_sub(counts.encoded_bytes).is_some_and(|bytes| bytes <= budget.bytes))
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
