use std::sync::Mutex;

use census_store::recorded::{
    FragmentSummary, Recorded, RecordedBatch, RecordedEffect, RecordedJournal,
};
use serde_json::Value;

use crate::CrawlResult;

mod apply;
#[cfg(test)]
mod apply_tests;
mod budget;
mod effect;
mod journal_index;
mod mutation;
mod once;
pub(crate) mod plan;
pub(crate) mod projection;
pub(crate) mod row;
mod sink;
pub mod spool;
pub use apply::{apply_recorded, prepare_recorded, PreparedRecorded};
pub use budget::{RecordingUsage, MAX_RECORDED_BYTES, MAX_RECORDED_WORK};

pub use sink::{RowApplication, RowBatch, RowSink};

#[derive(Debug, Default)]
pub struct Recording {
    state: Mutex<RecordingState>,
    spool: Option<spool::Spool>,
}

#[derive(Debug, Default)]
struct RecordingState {
    recorded: Recorded,
    usage: RecordingUsage,
    journal_index: std::collections::HashMap<journal_index::JournalDigest, usize>,
    fragments: usize,
    fragment_digest: sha2::Sha256,
    fragment_bytes: usize,
    fragment_rows: usize,
    spool_bytes: usize,
}

impl Recording {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spooled(store: census_store::Store, identity: String) -> CrawlResult<Self> {
        let spool = spool::Spool { store, identity };
        let recording = Self {
            state: Mutex::new(RecordingState::default()),
            spool: Some(spool),
        };
        {
            let mut state = recording.state();
            if let Some(spool) = &recording.spool {
                spool.reconstruct(&mut state)?;
            }
        }
        Ok(recording)
    }
    pub fn fragments(&self) -> usize {
        self.state().fragments
    }

    pub fn fragment_bytes(&self) -> usize {
        self.state().fragment_bytes
    }

    pub fn fragment_rows(&self) -> usize {
        self.state().fragment_rows
    }

    pub(crate) fn fragment_identity(&self) -> Option<&str> {
        self.spool.as_ref().map(|spool| spool.identity.as_str())
    }

    pub fn finish_fragments(&self) -> crate::CrawlResult<FragmentSummary> {
        let mut state = self.state();
        let spool = self
            .spool
            .as_ref()
            .ok_or_else(|| crate::CrawlError::Invariant {
                detail: "recording has no durable fragment spool".into(),
            })?;
        spool.flush(&mut state)?;
        Ok(spool::summary(&state))
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
        let held = if self.spool.is_some() {
            RecordingUsage::default()
        } else {
            self.state().usage
        };
        held.admitted(RecordingUsage {
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
        if let Some(entry) = journal_index::lookup(&state, phase, key)? {
            return inspect(Some(&entry.payload));
        }
        let pending = match &self.spool {
            Some(spool) => spool.lookup(phase, key)?,
            None => None,
        };
        inspect(pending.as_ref())
    }

    pub fn append_batches(
        &self,
        rows: Vec<RecordedBatch>,
        journal: Vec<RecordedJournal>,
    ) -> crate::CrawlResult<()> {
        let mut state = self.state();
        if let Some(spool) = &self.spool {
            spool.admit(&mut state, &rows, &journal)?;
        }
        mutation::append_state(&mut state, rows, journal, None)
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
        if let Some(spool) = &self.spool {
            spool.admit(&mut state, &rows, &journal)?;
        }
        let extent = effect::pending(&state.recorded, &rows, &journal)?;
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
mod tests;
