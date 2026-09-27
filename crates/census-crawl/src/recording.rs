
use std::sync::{Mutex, PoisonError};

use census_store::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod sink;

pub use sink::{RowBatch, RowSink};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedBatch {
    pub table: Table,
    pub rows: Vec<Value>,
}

impl RecordedBatch {
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedJournal {
    pub phase: String,
    pub key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Recorded {
    pub rows: Vec<RecordedBatch>,
    pub journal: Vec<RecordedJournal>,
}

impl Recorded {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.journal.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct Recording {
    batches: Mutex<Vec<RecordedBatch>>,
    journal: Mutex<Vec<RecordedJournal>>,
}

impl Recording {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain(&self) -> Recorded {
        Recorded {
            rows: std::mem::take(&mut *self.batches.lock().unwrap_or_else(PoisonError::into_inner)),
            journal: std::mem::take(
                &mut *self.journal.lock().unwrap_or_else(PoisonError::into_inner),
            ),
        }
    }

    pub fn rows(&self) -> usize {
        self.batches
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(RecordedBatch::len)
            .sum()
    }

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
    use census_store::Store;
    use serde_json::json;

    fn scratch() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path().join("store")).expect("store");
        (dir, store)
    }

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
