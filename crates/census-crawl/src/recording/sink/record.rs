use super::super::{budget, RecordedBatch, RecordedJournal, Recording, RecordingUsage};
use crate::{CrawlError, CrawlResult};
use census_store::Table;
use serde::Serialize;
use serde_json::Value;

pub(super) struct RecordSink<'a> {
    recording: &'a Recording,
    rows: Vec<RecordedBatch>,
    journal: Vec<RecordedJournal>,
    usage: RecordingUsage,
}

impl<'a> RecordSink<'a> {
    pub(super) fn new(recording: &'a Recording) -> Self {
        Self {
            recording,
            rows: Vec::new(),
            journal: Vec::new(),
            usage: RecordingUsage::default(),
        }
    }

    pub(super) fn append_many<T: Serialize>(
        &mut self,
        table: Table,
        records: &[T],
    ) -> CrawlResult<()> {
        let usage = self.prepare_rows(records)?;
        let rows = encode_rows(table, records)?;
        reserve_one(&mut self.rows)?;
        self.rows.push(RecordedBatch { table, rows });
        self.usage = usage;
        Ok(())
    }

    pub(super) fn append(&mut self, table: Table, rows: Vec<Value>) -> CrawlResult<()> {
        let usage = self.prepare_rows(&rows)?;
        reserve_one(&mut self.rows)?;
        self.rows.push(RecordedBatch { table, rows });
        self.usage = usage;
        Ok(())
    }

    pub(super) fn journal_done<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> CrawlResult<()> {
        let usage = self.prepare(&(phase, key, payload), 1)?;
        let payload = serde_json::to_value(payload).map_err(|source| CrawlError::Encode {
            table: format!("journal:{phase}"),
            source,
        })?;
        let phase = owned_text(phase)?;
        let key = owned_text(key)?;
        reserve_one(&mut self.journal)?;
        self.journal.push(RecordedJournal {
            phase,
            key,
            payload,
        });
        self.usage = usage;
        Ok(())
    }
    fn prepare<T: Serialize + ?Sized>(
        &self,
        value: &T,
        work: usize,
    ) -> CrawlResult<RecordingUsage> {
        self.prepare_usage(budget::preflight(value)?, work)
    }
    fn prepare_rows<T: Serialize>(&self, rows: &[T]) -> CrawlResult<RecordingUsage> {
        self.prepare_usage(budget::preflight_rows(rows)?, rows.len())
    }
    fn prepare_usage(&self, storage: usize, work: usize) -> CrawlResult<RecordingUsage> {
        let retained_bytes = budget::add(storage, budget::EFFECT_STORAGE)?;
        let usage = self.usage.admitted(RecordingUsage {
            retained_bytes,
            work,
        })?;
        self.recording.admit(usage.retained_bytes, usage.work)?;
        Ok(usage)
    }

    pub(super) fn commit(self) -> CrawlResult<()> {
        self.recording.append_batches(self.rows, self.journal)
    }

    pub(super) fn commit_once(
        mut self,
        effect: super::super::once::Effect<'_>,
    ) -> CrawlResult<super::RowApplication> {
        self.journal_done(
            super::super::once::PHASE,
            effect.operation(),
            &effect.witness(),
        )?;
        self.recording.append_once(self.rows, self.journal, effect)
    }
}

fn encode_rows<T: Serialize>(table: Table, records: &[T]) -> CrawlResult<Vec<Value>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(records.len()).map_err(|_| {
        budget::resource(
            "recorded rows allocation",
            records.len(),
            super::super::MAX_RECORDED_WORK,
        )
    })?;
    records.iter().try_for_each(|record| {
        rows.push(
            serde_json::to_value(record).map_err(|source| CrawlError::Encode {
                table: table.file().to_string(),
                source,
            })?,
        );
        Ok::<_, CrawlError>(())
    })?;
    Ok(rows)
}

fn reserve_one<T>(rows: &mut Vec<T>) -> CrawlResult<()> {
    let requested = budget::add(rows.len(), 1)?;
    rows.try_reserve(1).map_err(|_| {
        budget::resource(
            "recorded batch allocation",
            requested,
            super::super::MAX_RECORDED_WORK,
        )
    })
}

fn owned_text(value: &str) -> CrawlResult<String> {
    let mut text = String::new();
    text.try_reserve_exact(value.len()).map_err(|_| {
        budget::resource(
            "recorded text allocation",
            value.len(),
            super::super::MAX_RECORDED_BYTES,
        )
    })?;
    text.push_str(value);
    Ok(text)
}
