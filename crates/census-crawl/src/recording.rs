use std::sync::Mutex;

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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
            rows: std::mem::take(&mut *match self.batches.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }),
            journal: std::mem::take(&mut *match self.journal.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }),
        }
    }

    pub fn rows(&self) -> usize {
        match self.batches.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
        .iter()
        .map(RecordedBatch::len)
        .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.rows() == 0
            && match self.journal.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }
            .is_empty()
    }

    fn push(&self, batch: RecordedBatch) {
        match self.batches.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
        .push(batch);
    }

    fn push_journal(&self, entry: RecordedJournal) {
        match self.journal.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
        .push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_store::Store;
    use serde_json::json;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn scratch() -> TestResult<(tempfile::TempDir, Store)> {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path().join("store"))?;
        Ok((dir, store))
    }

    #[test]
    fn the_store_route_writes_on_commit() -> TestResult {
        let (_dir, store) = scratch()?;
        let mut dropped = RowSink::Store(&store).write_batch();
        dropped.append_many(Table::Meets, &[json!({"id": "m1"})])?;
        drop(dropped);
        check!(eq; store.walk_table(Table::Meets)?.rows, 0);

        let mut batch = RowSink::Store(&store).write_batch();
        batch.append_many(Table::Meets, &[json!({"id": "m1"}), json!({"id": "m2"})])?;
        batch.commit()?;
        check!(eq; store.walk_table(Table::Meets)?.rows, 2);
        Ok(())
    }

    #[test]
    fn the_recording_route_holds_rows_and_leaves_the_store_alone() -> TestResult {
        let (_dir, store) = scratch()?;
        let recording = Recording::new();
        let mut batch = RowSink::Record(&recording).write_batch();
        batch.append_many(Table::Meets, &[json!({"id": "m1"})])?;
        batch.append(Table::Performances, vec![json!({"id": "p1"})])?;
        batch.journal_done("milesplit_results_v1", "set-1", &json!({"url": "u"}))?;
        check!(
            recording.is_empty(),
            "nothing is recorded before the commit"
        );
        batch.commit()?;

        check!(eq; store.walk_table(Table::Meets)?.rows, 0);
        check!(eq; store.walk_table(Table::Performances)?.rows, 0);
        check!(eq; recording.rows(), 2);
        let drained = recording.drain();
        check!(eq; drained.rows.len(), 2);
        check!(eq; drained.rows[0].table, Table::Meets);
        check!(eq; drained.rows[0].rows, vec![json!({"id": "m1"})]);
        check!(eq; drained.rows[1].table, Table::Performances);
        check!(eq;
            drained.journal,
            vec![RecordedJournal {
                phase: "milesplit_results_v1".to_string(),
                key: "set-1".to_string(),
                payload: json!({"url": "u"}),
            }],
            "the unit marker rides the recording, not the store"
        );
        check!(recording.is_empty(), "a drain takes what it returns");
        Ok(())
    }

    #[test]
    fn a_recorded_unit_is_not_journaled_until_it_is_written() -> TestResult {
        let (_dir, store) = scratch()?;
        let recording = Recording::new();
        let mut batch = RowSink::Record(&recording).write_batch();
        batch.journal_done(
            "wayzata_schedule_v1",
            "page-1",
            &json!({"season": "2026-27"}),
        )?;
        batch.commit()?;
        check!(eq;
            store
                .journal_keys("wayzata_schedule_v1")
                ?
                .len(),
            0,
            "a marker the store cannot see is a marker no reader acts on"
        );
        check!(eq; recording.drain().journal.len(), 1);
        Ok(())
    }
}
