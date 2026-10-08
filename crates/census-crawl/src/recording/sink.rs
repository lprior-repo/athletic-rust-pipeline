use super::{once, Recording};
use crate::CrawlResult;
use census_store::{Store, StoreBatch, Table};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

mod record;
use record::RecordSink;

#[derive(Clone, Copy)]
pub enum RowSink<'a> {
    Store(&'a Store),
    Record {
        store: &'a Store,
        recording: &'a Recording,
    },
}

impl<'a> RowSink<'a> {
    pub fn write_batch(&self) -> RowBatch<'a> {
        let (store, sink) = match *self {
            Self::Store(store) => (store, Sink::Store(store.write_batch())),
            Self::Record { store, recording } => (store, Sink::Record(RecordSink::new(recording))),
        };
        RowBatch {
            store,
            sink,
            pending_journal: HashSet::new(),
        }
    }

    pub fn effect_is_committed(&self, operation: &str, digest: &str) -> CrawlResult<bool> {
        let effect = once::Effect::new(operation, digest)?;
        let store = match *self {
            Self::Store(store) => store,
            Self::Record { store, recording } => {
                let pending = recording
                    .inspect_journal(once::PHASE, operation, |payload| effect.seen(payload))?;
                if pending {
                    return Ok(true);
                }
                store
            }
        };
        effect.committed(store)
    }
}

pub struct RowBatch<'a> {
    store: &'a Store,
    sink: Sink<'a>,
    pending_journal: HashSet<super::journal_index::JournalDigest>,
}

enum Sink<'a> {
    Store(StoreBatch<'a>),
    Record(RecordSink<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowApplication {
    Admitted,
    Repeated,
}

impl RowApplication {
    pub fn written(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

impl RowBatch<'_> {
    pub fn append_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> CrawlResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        match &mut self.sink {
            Sink::Store(batch) => {
                batch.record_many(table, records)?;
                Ok(())
            }
            Sink::Record(batch) => batch.append_many(table, records),
        }
    }

    pub fn append(&mut self, table: Table, rows: Vec<Value>) -> CrawlResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        match &mut self.sink {
            Sink::Store(batch) => {
                batch.record_many(table, &rows)?;
                Ok(())
            }
            Sink::Record(batch) => batch.append(table, rows),
        }
    }

    pub fn journal_done<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> CrawlResult<()> {
        let digest = super::journal_index::digest(phase, key);
        self.reserve_journal_key(&digest)?;
        match &mut self.sink {
            Sink::Store(batch) => batch.journal_done(phase, key, payload)?,
            Sink::Record(batch) => batch.journal_done(phase, key, payload)?,
        }
        self.pending_journal.insert(digest);
        Ok(())
    }

    pub(crate) fn journal_is_staged(&self, phase: &str, key: &str) -> bool {
        self.pending_journal
            .contains(&super::journal_index::digest(phase, key))
    }

    fn reserve_journal_key(
        &mut self,
        digest: &super::journal_index::JournalDigest,
    ) -> CrawlResult<()> {
        if self.pending_journal.contains(digest) {
            return Ok(());
        }
        let requested = super::budget::add(self.pending_journal.len(), 1)?;
        super::budget::check(
            "batch journal index entries",
            requested,
            super::MAX_RECORDED_WORK,
        )?;
        self.pending_journal.try_reserve(1).map_err(|_| {
            super::budget::resource(
                "batch journal index allocation",
                requested,
                super::MAX_RECORDED_WORK,
            )
        })
    }

    pub fn commit(self) -> CrawlResult<()> {
        match self.sink {
            Sink::Store(batch) => {
                batch.commit()?;
                Ok(())
            }
            Sink::Record(batch) => batch.commit(),
        }
    }

    pub fn commit_once(self, operation: &str, digest: &str) -> CrawlResult<RowApplication> {
        let effect = once::Effect::new(operation, digest)?;
        if effect.committed(self.store)? {
            return Ok(RowApplication::Repeated);
        }
        match self.sink {
            Sink::Store(mut batch) => {
                batch.journal_done(once::PHASE, operation, &effect.witness())?;
                let applied = batch.commit_once(operation, digest)?;
                Ok(if applied.written() {
                    RowApplication::Admitted
                } else {
                    RowApplication::Repeated
                })
            }
            Sink::Record(batch) => batch.commit_once(effect),
        }
    }
}
