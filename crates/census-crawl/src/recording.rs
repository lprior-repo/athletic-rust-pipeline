use std::sync::Mutex;

use census_store::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod apply;
#[cfg(test)]
mod apply_tests;
mod budget;
mod effect;
mod journal_index;
mod mutation;
mod once;
pub(crate) mod projection;
pub(crate) mod row;
mod sink;
pub use apply::PreparedRecorded;
pub use budget::{RecordingUsage, MAX_RECORDED_BYTES, MAX_RECORDED_WORK};
pub use effect::RecordedEffect;

pub use sink::{RowApplication, RowBatch, RowSink};

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
    #[serde(default)]
    pub effects: Vec<RecordedEffect>,
}

impl Recorded {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.journal.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct Recording {
    state: Mutex<RecordingState>,
}

#[derive(Debug, Default)]
struct RecordingState {
    recorded: Recorded,
    usage: RecordingUsage,
    journal_index: std::collections::HashMap<journal_index::JournalDigest, usize>,
}

impl Recording {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain(&self) -> Recorded {
        let mut state = self.state();
        state.usage = RecordingUsage::default();
        state.journal_index = std::collections::HashMap::new();
        std::mem::take(&mut state.recorded)
    }

    pub fn rows(&self) -> usize {
        self.state()
            .recorded
            .rows
            .iter()
            .map(RecordedBatch::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        let state = self.state();
        state.recorded.rows.is_empty() && state.recorded.journal.is_empty()
    }

    pub fn usage(&self) -> RecordingUsage {
        self.state().usage
    }

    pub fn admit(&self, retained_bytes: usize, work: usize) -> crate::CrawlResult<()> {
        self.state().usage.admitted(RecordingUsage {
            retained_bytes,
            work,
        })?;
        Ok(())
    }

    pub fn inspect_journal<T>(
        &self,
        phase: &str,
        key: &str,
        inspect: impl FnOnce(Option<&Value>) -> crate::CrawlResult<T>,
    ) -> crate::CrawlResult<T> {
        let state = self.state();
        inspect(journal_index::lookup(&state, phase, key)?.map(|entry| &entry.payload))
    }

    pub fn append_batches(
        &self,
        rows: Vec<RecordedBatch>,
        journal: Vec<RecordedJournal>,
    ) -> crate::CrawlResult<()> {
        mutation::append_state(&mut self.state(), rows, journal, None)
    }

    fn append_once(
        &self,
        rows: Vec<RecordedBatch>,
        journal: Vec<RecordedJournal>,
        effect: once::Effect<'_>,
    ) -> crate::CrawlResult<RowApplication> {
        let mut state = self.state();
        let previous = journal_index::lookup(&state, once::PHASE, effect.operation())?;
        if effect.seen(previous.map(|entry| &entry.payload))? {
            return Ok(RowApplication::Repeated);
        }
        let extent = RecordedEffect::pending(&state.recorded, &rows, &journal)?;
        mutation::append_state(&mut state, rows, journal, Some(extent))?;
        Ok(RowApplication::Admitted)
    }

    fn state(&self) -> std::sync::MutexGuard<'_, RecordingState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poison) => poison.into_inner(),
        }
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
        let mut batch = RowSink::Record {
            store: &store,
            recording: &recording,
        }
        .write_batch();
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
        let mut batch = RowSink::Record {
            store: &store,
            recording: &recording,
        }
        .write_batch();
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

    #[test]
    fn bounded_rows_keep_individual_node_limits_and_exact_batch_conservation() -> TestResult {
        #[derive(Serialize)]
        struct Row {
            id: u16,
            values: Vec<u8>,
        }
        let (_dir, store) = scratch()?;
        let recording = Recording::new();
        let rows: Vec<_> = (0..1000)
            .map(|id| Row {
                id,
                values: vec![7; 100],
            })
            .collect();
        let sink = RowSink::Record {
            store: &store,
            recording: &recording,
        };
        let mut batch = sink.write_batch();
        batch.append_many(Table::Meets, &rows)?;
        batch.commit()?;
        check!(eq; recording.rows(), rows.len());
        let oversized = Row {
            id: 1000,
            values: vec![7; MAX_RECORDED_WORK],
        };
        let mut refused = sink.write_batch();
        check!(matches!(
            refused.append_many(Table::Meets, &[oversized]),
            Err(crate::CrawlError::Resource {
                resource: "recording serialization work",
                ..
            })
        ));
        drop(refused);
        check!(eq; recording.rows(), rows.len());
        check!(eq; store.walk_table(Table::Meets)?.rows, 0);
        let drained = recording.drain();
        check!(eq; drained.rows.len(), 1);
        let retained = drained.rows.first().ok_or("recorded batch")?;
        check!(eq; retained.table, Table::Meets);
        check!(eq; retained.rows.len(), rows.len());
        for (retained, original) in retained.rows.iter().zip(rows) {
            check!(eq; retained, &serde_json::to_value(original)?);
        }
        Ok(())
    }
}
