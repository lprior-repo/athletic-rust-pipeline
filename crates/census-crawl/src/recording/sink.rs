//! The writer a walk holds, and the batch it commits through.
//!
//! The routing lives here: an adapter never chooses between the store and a recording, it writes
//! through [`RowBatch`], and this module decides where the rows end up. Both routes keep the store's
//! own rule — nothing is written or recorded until [`RowBatch::commit`] — so a batch dropped without
//! one leaves either writer exactly as it found it.

use census_store::{Store, StoreBatch, Table};
use serde::Serialize;
use serde_json::Value;

use super::{RecordedBatch, RecordedJournal, Recording};
use crate::{CrawlError, CrawlResult};

/// Where a walk's rows go: the store it holds, or a recording its caller drains.
#[derive(Clone, Copy)]
pub enum RowSink<'a> {
    /// Write the rows: the route a run that owns its store takes.
    Store(&'a Store),
    /// Hold the rows: the route a run that posts them to an `Ingest` object takes.
    Record(&'a Recording),
}

impl<'a> RowSink<'a> {
    /// Start a batch on this sink. Nothing is written or recorded until [`RowBatch::commit`], which
    /// is the store's own rule: a batch dropped without one leaves the sink exactly as it found it.
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

/// A batch of rows that is written or recorded when it commits.
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
    /// Buffer one table's rows.
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

    /// Buffer rows that are already in the wire form a recording carries.
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

    /// Buffer the marker that names one unit this walk read.
    ///
    /// The store route writes it with the rows it covers, which is what makes a unit journaled only
    /// once its rows are durable. The recorded route holds it for the caller to write *after* it has
    /// posted those rows, which keeps the same order across the two writers.
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

    /// Commit: the store writes the batch, a recording files it under its sink.
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
