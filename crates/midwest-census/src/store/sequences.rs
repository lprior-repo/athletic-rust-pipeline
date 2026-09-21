//! The sequence counters: one per table, and the only state the store's writers contend on.
//!
//! A batch of observations reserves a run of `count` consecutive sequences with a single atomic
//! reservation, so two writers appending at the same moment can never be handed the same sequence.
//! The keyspace depends on that: an observation key carries its sequence, so a reused number
//! overwrites an observation instead of adding one.
//!
//! The counter reserves through [`Reserve`], which `std`'s atomic implements for production and
//! `loom`'s atomic implements for the model check in `loom_tests`, so the model exercises the
//! allocation code the store actually runs.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use fjall::Keyspace;

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
    /// The reservation is relaxed on purpose: what must be unique is the *value* handed out, which
    /// the read-modify-write guarantees by itself, while the observations written under the
    /// resulting keys are published by the keyspace commit that stores them.
    fn reserve(&self, count: u64) -> u64;
    /// The value the next reservation would return.
    fn next(&self) -> u64;
}

impl Reserve for AtomicU64 {
    fn starting_at(start: u64) -> Self {
        AtomicU64::new(start)
    }

    fn reserve(&self, count: u64) -> u64 {
        self.fetch_add(count, Ordering::Relaxed)
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

    /// Reserve `count` consecutive sequences and return the first one.
    pub(super) fn reserve(&self, count: u64) -> u64 {
        self.0.reserve(count)
    }

    /// The sequence a reservation made now would start at.
    pub(super) fn next(&self) -> u64 {
        self.0.next()
    }
}

/// Every table's counter, seeded from the keyspace at open.
pub(super) struct Counters(BTreeMap<&'static str, SequenceCounter<AtomicU64>>);

impl Counters {
    /// Seed one counter per table from the highest sequence already stored.
    ///
    /// Reopening must not hand out a sequence the database holds: the observation written under it
    /// would be overwritten rather than added to.
    pub(super) fn seeded(entities: &Keyspace) -> StoreResult<Self> {
        let mut counters = BTreeMap::new();
        for table in Table::ALL {
            let start = Store::last_sequence(entities, table)?
                .map(|sequence| sequence.saturating_add(1))
                .unwrap_or(0);
            counters.insert(table.file(), SequenceCounter::starting_at(start));
        }
        Ok(Self(counters))
    }

    /// Reserve `count` consecutive sequences for one table.
    pub(super) fn reserve(&self, table: Table, count: u64) -> StoreResult<u64> {
        let counter = self
            .0
            .get(table.file())
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("table {} has no sequence counter", table.file()),
            })?;
        Ok(counter.reserve(count))
    }

    /// How many observations `table` has appended: the sequence it would append next.
    pub(super) fn appended(&self, table: Table) -> u64 {
        self.0.get(table.file()).map_or(0, SequenceCounter::next)
    }
}
