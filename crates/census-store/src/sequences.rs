//! The sequence counters: one per table, and the only state the store's writers contend on.
//!
//! A batch of observations reserves a run of `count` consecutive sequences with a single atomic
//! reservation, so two writers appending at the same moment can never be handed the same sequence.
//! The keyspace depends on that: an observation key carries its sequence, so a reused number
//! overwrites an observation instead of adding one.
//!
//! The reservation refuses rather than wraps: a run the counter's range cannot hold leaves the counter
//! exactly where it found it, because a wrapped counter would hand out a sequence the table has
//! already spent, and the row written under it would overwrite an observation instead of adding one.
//! The row ceiling is enforced one step outward, in [`Store::reserve`] — the funnel every reservation
//! passes through — on the same next-sequence arithmetic.
//!
//! [`Store::reserve`]: super::Store::reserve
//!
//! Every counter also has a durable half: its *mark*, the `meta` row holding the sequence the table's
//! next append will use. The mark travels in the same batch as the observations that spend the
//! sequences, so an ordinary open reads one small row per table instead of walking every table to
//! find where each one resumes — [`Counters::seeded`] is that read, and the one scan a store written
//! before marks existed still owes.
//!
//! The counter reserves through [`Reserve`], which `std`'s atomic implements for production and
//! `loom`'s atomic implements for the model check in `loom_tests`, so the model exercises the
//! allocation code the store actually runs.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::MutexGuard;

use fjall::{Database, Keyspace, OwnedWriteBatch, PersistMode};

use super::{Store, StoreError, StoreResult, Table};

/// The one atomic operation sequence allocation needs.
///
/// Two implementations exist — `std::sync::atomic::AtomicU64` and `loom`'s instrumented atomic —
/// and that is what lets both the store and the model check run the same [`SequenceCounter`].
pub(super) trait Reserve: Send + Sync + 'static {
    /// A counter whose first reservation starts at `start`.
    fn starting_at(start: u64) -> Self;
    /// Atomically reserve `count` sequences, and return the first one reserved.
    ///
    /// `None` means the run does not fit in the counter's range, and the counter is then left exactly
    /// as it was. A wrapped counter would hand out a sequence the table has already spent, and an
    /// observation key carries its sequence, so the row written under one would overwrite the
    /// observation stored there rather than add to it.
    ///
    /// The reservation is relaxed on purpose: what must be unique is the *value* handed out, which
    /// the read-modify-write guarantees by itself, while the observations written under the
    /// resulting keys are published by the keyspace commit that stores them.
    fn reserve(&self, count: u64) -> Option<u64>;
    /// The value the next reservation would return.
    fn next(&self) -> u64;
}

impl Reserve for AtomicU64 {
    fn starting_at(start: u64) -> Self {
        AtomicU64::new(start)
    }

    fn reserve(&self, count: u64) -> Option<u64> {
        let mut current = self.load(Ordering::Relaxed);
        loop {
            // Load, check, exchange: a run that would leave the counter's range is refused before the
            // counter moves, and a contended exchange retries against the value the winner left.
            let next = current.checked_add(count)?;
            match self.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return Some(current),
                Err(observed) => current = observed,
            }
        }
    }

    fn next(&self) -> u64 {
        self.load(Ordering::Relaxed)
    }
}

/// One table's sequence counter.
pub(super) struct SequenceCounter<A>(A);

impl<A: Reserve> SequenceCounter<A> {
    /// A counter whose first reservation starts at `start`.
    pub(super) fn starting_at(start: u64) -> Self {
        Self(A::starting_at(start))
    }

    /// Reserve `count` consecutive sequences and return the first one, or `None` for a run the
    /// counter's range cannot hold.
    pub(super) fn reserve(&self, count: u64) -> Option<u64> {
        self.0.reserve(count)
    }

    /// The sequence a reservation made now would start at.
    pub(super) fn next(&self) -> u64 {
        self.0.next()
    }
}

/// A reservation: the sequence a batch keys its first observation under, and the mark that same
/// batch has to store.
///
/// The two travel together because the mark is the reservation's own arithmetic — the sequence past
/// the last one it hands out. A batch that stored a mark of its own instead could leave a reopen a
/// sequence the batch had already spent, and the observation written under it would overwrite the
/// one already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Reserved {
    /// The sequence the batch's first observation is keyed under.
    pub(super) base: u64,
    /// The sequence the table's next append will use: what the batch stores as the table's mark.
    pub(super) mark: u64,
}

/// The `meta` row holding one table's mark.
///
/// One row per table, beside the store's other `meta` values, so an operator reading `meta` sees
/// where every table resumes — and so no open has to read the table itself to find out.
pub(super) fn mark_key(table: Table) -> String {
    format!("sequence:{}", table.file())
}

/// Every table's counter, seeded from each table's mark.
pub(super) struct Counters(BTreeMap<&'static str, SequenceCounter<AtomicU64>>);

impl Counters {
    /// Seed one counter per table, and establish the marks a store written before marks existed has
    /// none of.
    ///
    /// Reopening must not hand out a sequence the database holds: the observation written under it
    /// would overwrite the one already there rather than add to it. A table's mark is that number. A
    /// table without one is the single case where it has to be derived, and the derivation is the
    /// scan this store has always done; its result is committed here, before any writer can append
    /// and before the legacy import runs, so the first open that misses a mark is the last open that
    /// scans that table.
    ///
    /// A mark that is present but unreadable fails the open. A number nobody can read is not a number
    /// to guess at, and a guess that came out low would overwrite stored observations — which is the
    /// one thing a reopen may never do.
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
            counters.insert(table.file(), SequenceCounter::starting_at(start));
        }
        write_marks(db, meta, &derived)?;
        Ok(Self(counters))
    }

    /// Reserve `count` consecutive sequences for one table.
    ///
    /// Both reserving callers hold the append lock, which is what makes the returned mark the table's
    /// highest: without it two writers could reserve in one order and commit in the other, and the
    /// later commit would write the lower mark.
    pub(super) fn reserve(&self, table: Table, count: u64) -> StoreResult<Reserved> {
        let counter = self
            .0
            .get(table.file())
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} has no sequence counter", table.file()),
            })?;
        // A run the counter cannot represent is refused before the counter moves: the sequences it has
        // already handed out key observations the store holds, and a second row under one of them
        // would overwrite the first rather than add to it.
        let base = counter.reserve(count).ok_or(StoreError::CounterOverflow)?;
        // `reserve` computed this sum to decide the run fits, so it cannot overflow here; the check
        // stays because the alternative is an unchecked add.
        let mark = base.checked_add(count).ok_or(StoreError::CounterOverflow)?;
        Ok(Reserved { base, mark })
    }

    /// The sequence `table`'s next append would key its first observation under.
    ///
    /// A pointer, not a count: an append reserves the sequences of a batch before the batch commits,
    /// so a commit that fails leaves this ahead of the rows the store holds. [`Store::count`] is the
    /// count, and it comes from the row ledger rather than from here.
    ///
    /// [`Store::count`]: super::Store::count
    pub(super) fn next_sequence(&self, table: Table) -> u64 {
        self.0.get(table.file()).map_or(0, SequenceCounter::next)
    }
}

/// A table's mark as the store holds it, or `None` for a table that has none yet.
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

/// The mark one scan of a table establishes: one past its highest stored sequence, or zero for a
/// table that holds nothing.
fn scanned_mark(entities: &Keyspace, table: Table) -> StoreResult<u64> {
    Ok(Store::last_sequence(entities, table)?
        .map(|sequence| sequence.saturating_add(1))
        .unwrap_or(0))
}

/// Commit the marks an open had to derive, so the scan that derived them happens once.
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
    /// Take the append lock: what orders the batches that spend a table's sequences.
    ///
    /// A reservation orders the numbers handed out, not the batches that spend them. Two writers
    /// reserve disjoint runs and commit in whichever order the filesystem gives them, so the writer
    /// that reserved first can commit last — and would then store a mark below rows already written
    /// above it, leaving the next open free to hand out sequences the database already holds. Holding
    /// this lock across the reservation, the batch and the commit is what rules that out.
    ///
    /// Nothing here crosses an `await`: the store's writers are blocking jobs. A poisoned lock is
    /// recovered, because the writer that panicked held a batch that was never committed — a batch is
    /// applied whole or not at all — so the counters and the marks still describe the keyspace.
    pub(super) fn lock_appends(&self) -> MutexGuard<'_, ()> {
        match self.appends.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Store `mark` as `table`'s mark, inside the batch that spends the sequences it accounts for.
    pub(super) fn put_mark(&self, batch: &mut OwnedWriteBatch, table: Table, mark: u64) {
        batch.insert(&self.meta, mark_key(table), mark.to_string().as_bytes());
    }
}
