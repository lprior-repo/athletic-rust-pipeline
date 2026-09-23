//! The recording sink: the rows a walk produced but did not write.
//!
//! An adapter's rows normally travel straight into the store its walk holds. A run whose acquisition
//! is routed through its deployment's `Ingest` objects writes none of them itself: it records them,
//! hands them to its caller, and the object appends them. The walk is the same walk either way —
//! same parse, same row builder, same commit boundary — because the routing lives in [`RowBatch`],
//! the writer [`AdapterContext::write_batch`](crate::AdapterContext::write_batch) hands out, and not
//! in the adapter.
//!
//! A recording belongs to one source's walk. The caller drains it when the walk returns, so what it
//! holds is exactly the rows that walk produced, and posts them to the endpoint that serves that
//! source — which is what makes an acquisition route visible as observations on a durable object
//! instead of as rows nobody counted.

use std::sync::{Mutex, PoisonError};

use census_store::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod sink;

pub use sink::{RowBatch, RowSink};

/// One table's rows, as a walk produced them: the unit a recording carries and a caller posts.
///
/// `Serialize`/`Deserialize` because the recording crosses the durable boundary a stage runs behind:
/// the rows a walk recorded are part of what that run's journal holds, so a replay posts the same
/// rows instead of walking again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedBatch {
    /// The table the rows belong to.
    pub table: Table,
    /// The rows, each carrying its canonical `id`.
    pub rows: Vec<Value>,
}

impl RecordedBatch {
    /// How many rows this batch carries.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True when this batch carries no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// One journal entry a routed walk produced: the marker that names a unit it read.
///
/// The entry is *not* written when the walk commits it. A run whose rows are posted writes the
/// marker only once the rows it covers are posted, because the invariant the store's own batch
/// keeps — no unit is journaled whose rows are missing — is the reason the marker exists at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedJournal {
    /// The phase the entry belongs to, as the walk names it (`wiaa_results`, its parser version, …).
    pub phase: String,
    /// The unit within the phase, as the walk names it (an artifact URL, a result-set key, …).
    pub key: String,
    /// The entry's payload: the evidence the walk wants to keep about the unit it read.
    pub payload: Value,
}

/// Everything one routed walk produced: the rows to post, and the journal entries to write once they
/// are posted.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Recorded {
    /// The rows, one batch per table per commit, in the order the walk produced them.
    pub rows: Vec<RecordedBatch>,
    /// The journal entries, in the order the walk produced them.
    pub journal: Vec<RecordedJournal>,
}

impl Recorded {
    /// Rows across every batch.
    pub fn rows(&self) -> usize {
        self.rows.iter().map(RecordedBatch::len).sum()
    }

    /// True when the walk neither produced rows nor read a unit worth journaling.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.journal.is_empty()
    }
}

/// What one walk recorded instead of writing.
///
/// A `Mutex` rather than a channel because the walk is synchronous: an adapter appends, commits and
/// moves on, and the caller drains once the walk has returned. Poisoning is ignored on purpose — a
/// panic elsewhere must not turn a readable recording into a second failure.
#[derive(Debug, Default)]
pub struct Recording {
    batches: Mutex<Vec<RecordedBatch>>,
    journal: Mutex<Vec<RecordedJournal>>,
}

impl Recording {
    /// An empty recording.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take everything committed so far, in the order the walk produced it.
    pub fn drain(&self) -> Recorded {
        Recorded {
            rows: std::mem::take(&mut *self.batches.lock().unwrap_or_else(PoisonError::into_inner)),
            journal: std::mem::take(
                &mut *self.journal.lock().unwrap_or_else(PoisonError::into_inner),
            ),
        }
    }

    /// Rows recorded so far, across every batch.
    pub fn rows(&self) -> usize {
        self.batches
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(RecordedBatch::len)
            .sum()
    }

    /// True when nothing at all has been committed.
    pub fn is_empty(&self) -> bool {
        self.rows() == 0
            && self
                .journal
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_empty()
    }

    fn push(&self, batch: RecordedBatch) {
        self.batches
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(batch);
    }

    fn push_journal(&self, entry: RecordedJournal) {
        self.journal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use serde_json::json;

    fn scratch() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path().join("store")).expect("store");
        (dir, store)
    }

    /// The store route writes what it commits — and only on commit.
    #[test]
    fn the_store_route_writes_on_commit() {
        let (_dir, store) = scratch();
        let mut dropped = RowSink::Store(&store).write_batch();
        dropped
            .append_many(Table::Meets, &[json!({"id": "m1"})])
            .expect("buffered");
        drop(dropped);
        assert_eq!(store.walk_table(Table::Meets).expect("walk").rows, 0);

        let mut batch = RowSink::Store(&store).write_batch();
        batch
            .append_many(Table::Meets, &[json!({"id": "m1"}), json!({"id": "m2"})])
            .expect("buffered");
        batch.commit().expect("committed");
        assert_eq!(store.walk_table(Table::Meets).expect("walk").rows, 2);
    }

    /// The recording route writes nothing and hands the rows and entries to its caller instead.
    #[test]
    fn the_recording_route_holds_rows_and_leaves_the_store_alone() {
        let (_dir, store) = scratch();
        let recording = Recording::new();
        let mut batch = RowSink::Record(&recording).write_batch();
        batch
            .append_many(Table::Meets, &[json!({"id": "m1"})])
            .expect("recorded");
        batch
            .append(Table::Performances, vec![json!({"id": "p1"})])
            .expect("recorded");
        batch
            .journal_done("milesplit_results_v1", "set-1", &json!({"url": "u"}))
            .expect("recorded");
        assert!(
            recording.is_empty(),
            "nothing is recorded before the commit"
        );
        batch.commit().expect("committed");

        assert_eq!(store.walk_table(Table::Meets).expect("walk").rows, 0);
        assert_eq!(store.walk_table(Table::Performances).expect("walk").rows, 0);
        assert_eq!(recording.rows(), 2);
        let drained = recording.drain();
        assert_eq!(drained.rows.len(), 2);
        assert_eq!(drained.rows[0].table, Table::Meets);
        assert_eq!(drained.rows[0].rows, vec![json!({"id": "m1"})]);
        assert_eq!(drained.rows[1].table, Table::Performances);
        assert_eq!(
            drained.journal,
            vec![RecordedJournal {
                phase: "milesplit_results_v1".to_string(),
                key: "set-1".to_string(),
                payload: json!({"url": "u"}),
            }],
            "the unit marker rides the recording, not the store"
        );
        assert!(recording.is_empty(), "a drain takes what it returns");
    }

    /// The recorded route journals nothing: a unit that is not written yet is not marked read.
    #[test]
    fn a_recorded_unit_is_not_journaled_until_it_is_written() {
        let (_dir, store) = scratch();
        let recording = Recording::new();
        let mut batch = RowSink::Record(&recording).write_batch();
        batch
            .journal_done(
                "wayzata_schedule_v1",
                "page-1",
                &json!({"season": "2026-27"}),
            )
            .expect("recorded");
        batch.commit().expect("committed");
        assert_eq!(
            store
                .journal_keys("wayzata_schedule_v1")
                .expect("read")
                .len(),
            0,
            "a marker the store cannot see is a marker no reader acts on"
        );
        assert_eq!(recording.drain().journal.len(), 1);
    }
}
