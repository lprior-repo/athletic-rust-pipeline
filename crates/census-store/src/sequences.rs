use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::MutexGuard;

use fjall::{Database, Keyspace, OwnedWriteBatch, PersistMode};

use super::batch::refuse_over_bound;
use super::{Store, StoreError, StoreResult, Table};

pub(super) trait SequenceValue: Send + Sync + 'static {
    fn starting_at(start: u64) -> Self;
    fn publish(&self, mark: u64);
    fn next(&self) -> u64;
}
impl SequenceValue for AtomicU64 {
    fn starting_at(start: u64) -> Self {
        AtomicU64::new(start)
    }

    fn publish(&self, mark: u64) {
        self.store(mark, Ordering::Relaxed);
    }

    fn next(&self) -> u64 {
        self.load(Ordering::Relaxed)
    }
}

pub(super) struct SequenceCounter<A>(A);

impl<A: SequenceValue> SequenceCounter<A> {
    pub(super) fn starting_at(start: u64) -> Self {
        Self(A::starting_at(start))
    }

    pub(super) fn plan(&self, table: Table, count: u64) -> StoreResult<Reserved> {
        let base = self.next();
        let mark = base.checked_add(count).ok_or(StoreError::CounterOverflow)?;
        refuse_over_bound(table, mark)?;
        Ok(Reserved { base, mark })
    }

    pub(super) fn publish(&self, mark: u64) {
        self.0.publish(mark);
    }

    pub(super) fn next(&self) -> u64 {
        self.0.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Reserved {
    pub(super) base: u64,
    pub(super) mark: u64,
}

pub(super) fn mark_key(table: Table) -> String {
    format!("sequence:{}", table.file())
}

pub(super) struct Counters(BTreeMap<&'static str, SequenceCounter<AtomicU64>>);

impl Counters {
    pub(super) fn seeded(db: &Database, entities: &Keyspace, meta: &Keyspace) -> StoreResult<Self> {
        let mut counters = BTreeMap::new();
        let mut derived: Vec<(Table, u64)> = Vec::new();
        for table in Table::ALL {
            let start = match stored_mark(meta, table)? {
                Some(mark) => mark,
                None => {
                    let mark = scanned_mark(entities, table)?;
                    derived.push((table, mark));
                    mark
                }
            };
            refuse_over_bound(table, start)?;
            counters.insert(table.file(), SequenceCounter::starting_at(start));
        }
        write_marks(db, meta, &derived)?;
        Ok(Self(counters))
    }

    pub(super) fn plan(&self, table: Table, count: u64) -> StoreResult<Reserved> {
        self.counter(table)?.plan(table, count)
    }

    pub(super) fn publish(&self, table: Table, mark: u64) -> StoreResult<()> {
        self.counter(table)?.publish(mark);
        Ok(())
    }

    pub(super) fn next_sequence(&self, table: Table) -> StoreResult<u64> {
        Ok(self.counter(table)?.next())
    }

    fn counter(&self, table: Table) -> StoreResult<&SequenceCounter<AtomicU64>> {
        self.0
            .get(table.file())
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} has no sequence counter", table.file()),
            })
    }
}

fn stored_mark(meta: &Keyspace, table: Table) -> StoreResult<Option<u64>> {
    let key = mark_key(table);
    let Some(value) = meta
        .get(&key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    match std::str::from_utf8(&value)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
    {
        Some(mark) => Ok(Some(mark)),
        None => Err(StoreError::Invariant {
            detail: format!("{key} is not a sequence mark"),
        }),
    }
}

fn scanned_mark(entities: &Keyspace, table: Table) -> StoreResult<u64> {
    match Store::last_sequence(entities, table)? {
        Some(sequence) => sequence.checked_add(1).ok_or(StoreError::CounterOverflow),
        None => Ok(0),
    }
}

fn write_marks(db: &Database, meta: &Keyspace, derived: &[(Table, u64)]) -> StoreResult<()> {
    if derived.is_empty() {
        return Ok(());
    }
    let mut batch = db.batch();
    for (table, mark) in derived {
        batch.insert(meta, mark_key(*table), mark.to_string().as_bytes());
    }
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

impl Store {
    pub(super) fn lock_appends(&self) -> MutexGuard<'_, ()> {
        match self.appends.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(super) fn put_mark(&self, batch: &mut OwnedWriteBatch, table: Table, mark: u64) {
        batch.insert(&self.meta, mark_key(table), mark.to_string().as_bytes());
    }
}
