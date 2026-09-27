
use census_store::{Store, StoreBatch, Table};
use serde::Serialize;
use serde_json::Value;

use super::{RecordedBatch, RecordedJournal, Recording};
use crate::{CrawlError, CrawlResult};

#[derive(Clone, Copy)]
pub enum RowSink<'a> {
    Store(&'a Store),
    Record(&'a Recording),
}

impl<'a> RowSink<'a> {
    pub fn write_batch(&self) -> RowBatch<'a> {
        RowBatch {
            sink: match self {
                Self::Store(store) => Sink::Store(store.write_batch()),
                Self::Record(recording) => Sink::Record {
                    recording,
                    rows: Vec::new(),
                    journal: Vec::new(),
                },
            },
        }
    }
}

pub struct RowBatch<'a> {
    sink: Sink<'a>,
}

enum Sink<'a> {
    Store(StoreBatch<'a>),
    Record {
        recording: &'a Recording,
        rows: Vec<RecordedBatch>,
        journal: Vec<RecordedJournal>,
    },
}

impl RowBatch<'_> {
    pub fn append_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> CrawlResult<()> {
        match &mut self.sink {
            Sink::Store(batch) => {
                batch.append_many(table, records)?;
                Ok(())
            }
            Sink::Record { rows, .. } => {
                let encoded = records
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<Result<Vec<Value>, _>>()
                    .map_err(|source| CrawlError::Encode {
                        table: table.file().to_string(),
                        source,
                    })?;
                rows.push(RecordedBatch {
                    table,
                    rows: encoded,
                });
                Ok(())
            }
        }
    }

    pub fn append(&mut self, table: Table, rows: Vec<Value>) -> CrawlResult<()> {
        match &mut self.sink {
            Sink::Store(batch) => {
                batch.append_many(table, &rows)?;
                Ok(())
            }
            Sink::Record { rows: held, .. } => {
                held.push(RecordedBatch { table, rows });
                Ok(())
            }
        }
    }

    pub fn journal_done<T: Serialize>(
        &mut self,
        phase: &str,
        key: &str,
        payload: &T,
    ) -> CrawlResult<()> {
        match &mut self.sink {
            Sink::Store(batch) => {
                batch.journal_done(phase, key, payload)?;
                Ok(())
            }
            Sink::Record { journal, .. } => {
                let payload =
                    serde_json::to_value(payload).map_err(|source| CrawlError::Encode {
                        table: format!("journal:{phase}"),
                        source,
                    })?;
                journal.push(RecordedJournal {
                    phase: phase.to_string(),
                    key: key.to_string(),
                    payload,
                });
                Ok(())
            }
        }
    }

    pub fn commit(self) -> CrawlResult<()> {
        match self.sink {
            Sink::Store(batch) => {
                batch.commit()?;
                Ok(())
            }
            Sink::Record {
                recording,
                rows,
                journal,
            } => {
                for batch in rows {
                    recording.push(batch);
                }
                for entry in journal {
                    recording.push_journal(entry);
                }
                Ok(())
            }
        }
    }
}
