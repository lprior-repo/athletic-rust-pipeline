use super::{budget, Recorded, RecordedBatch, RecordedJournal, RecordingState};
use crate::{CrawlError, CrawlResult};
use census_store::{
    ObservationBudget, Store, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub const FRAGMENT_PHASE: &str = "meet_recorded_fragments_v1";
const MANIFEST_PHASE: &str = "meet_recorded_fragment_manifests_v1";
const MAX_FRAGMENT_RETAINED_BYTES: usize = super::MAX_RECORDED_BYTES / 4;

pub(super) struct Spool {
    pub(super) store: Store,
    pub(super) identity: String,
}

impl std::fmt::Debug for Spool {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output
            .debug_struct("Spool")
            .field("identity", &self.identity)
            .finish()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Manifest {
    digest: String,
    encoded_bytes: usize,
    rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FragmentSummary {
    pub fragments: usize,
    pub digest: String,
    pub encoded_bytes: usize,
    pub rows: usize,
}

pub(super) fn summary(state: &RecordingState) -> FragmentSummary {
    FragmentSummary {
        fragments: state.fragments,
        digest: format!("{:x}", state.fragment_digest.clone().finalize()),
        encoded_bytes: state.fragment_bytes,
        rows: state.fragment_rows,
    }
}

pub fn verify_reference(
    store: &Store,
    identity: &str,
    expected: &FragmentSummary,
) -> CrawlResult<()> {
    let mut state = RecordingState::default();
    for index in 0..expected.fragments {
        let key = fragment_key(identity, index);
        let manifest: Manifest = decode(required(store, MANIFEST_PHASE, &key)?, &key)?;
        absorb_manifest(&mut state, &manifest)?;
        state.fragments = budget::add(state.fragments, 1)?;
    }
    if summary(&state) != *expected {
        return Err(invariant(format!(
            "meet spool reference changed: {identity}"
        )));
    }
    Ok(())
}

fn absorb_manifest(state: &mut RecordingState, manifest: &Manifest) -> CrawlResult<()> {
    let mut hash = HashWriter(state.fragment_digest.clone());
    serde_json::to_writer(&mut hash, manifest).map_err(|source| CrawlError::Decode {
        url: "meet spool reference digest".into(),
        source,
    })?;
    state.fragment_digest = hash.0;
    state.fragment_bytes = budget::add(state.fragment_bytes, manifest.encoded_bytes)?;
    state.fragment_rows = budget::add(state.fragment_rows, manifest.rows)?;
    Ok(())
}

#[derive(Serialize)]
struct Combined<'a> {
    rows: &'a [RecordedBatch],
    journal: &'a [RecordedJournal],
    effects: &'a [super::RecordedEffect],
}

impl Spool {
    pub(super) fn reconstruct(&self, state: &mut RecordingState) -> CrawlResult<()> {
        let mut ordinal = 0;
        loop {
            let key = fragment_key(&self.identity, ordinal);
            let Some(manifest_payload) = self.store.journal_payload(MANIFEST_PHASE, &key)? else {
                break;
            };
            let persisted: Manifest = decode(manifest_payload, &key)?;
            let Some(fragment_payload) = self.store.journal_payload(FRAGMENT_PHASE, &key)? else {
                return Err(invariant(format!(
                    "meet fragment manifest present without payload: {key}"
                )));
            };
            let recorded: Recorded = decode(fragment_payload, &key)?;
            let computed = manifest(&recorded)?;
            if computed != persisted {
                return Err(invariant(format!(
                    "meet fragment content mismatch at frontier: {key}"
                )));
            }
            absorb_manifest(state, &persisted)?;
            state.fragments = budget::add(state.fragments, 1)?;
            ordinal = budget::add(ordinal, 1)?;
        }
        Ok(())
    }
    pub(super) fn admit(
        &self,
        state: &mut RecordingState,
        rows: &[RecordedBatch],
        journal: &[RecordedJournal],
    ) -> CrawlResult<()> {
        let incoming = row_count(rows)?;
        budget::check(
            "meet fragment atomic rows",
            incoming,
            MAX_OBSERVATION_BATCH_ROWS,
        )?;
        rows.iter()
            .flat_map(|batch| &batch.rows)
            .try_for_each(|row| ObservationBudget::record_bytes(row).map(|_| ()))?;
        let bytes = budget::add(
            ObservationBudget::request_bytes(&Combined {
                rows,
                journal,
                effects: &[],
            })?,
            budget::EFFECT_STORAGE,
        )?;
        budget::check(
            "meet fragment atomic bytes",
            bytes,
            MAX_OBSERVATION_BATCH_BYTES,
        )?;
        let extra = budget::measure(rows, journal)?;
        let retained = budget::add(extra.retained_bytes, budget::EFFECT_STORAGE)?;
        budget::check(
            "meet fragment atomic retained bytes",
            retained,
            MAX_FRAGMENT_RETAINED_BYTES,
        )?;
        if budget::add(row_count(&state.recorded.rows)?, incoming)? > MAX_OBSERVATION_BATCH_ROWS
            || budget::add(state.spool_bytes, bytes)? > MAX_OBSERVATION_BATCH_BYTES
            || budget::add(state.usage.retained_bytes, retained)? > MAX_FRAGMENT_RETAINED_BYTES
            || budget::add(state.usage.work, budget::add(extra.work, 1)?)?
                > super::MAX_RECORDED_WORK
        {
            self.flush(state)?;
        }
        state.spool_bytes = budget::add(state.spool_bytes, bytes)?;
        Ok(())
    }

    pub(super) fn flush(&self, state: &mut RecordingState) -> CrawlResult<()> {
        if state.recorded.is_empty() {
            return Ok(());
        }
        let manifest = manifest(&state.recorded)?;
        let key = fragment_key(&self.identity, state.fragments);
        if let Some(previous) = self.store.journal_payload(FRAGMENT_PHASE, &key)? {
            let previous: Recorded = decode(previous, &key)?;
            if previous != state.recorded {
                return Err(invariant(format!("meet fragment identity changed: {key}")));
            }
        } else {
            self.store
                .journal_done(FRAGMENT_PHASE, &key, &state.recorded)?;
        }
        if let Some(previous) = self.store.journal_payload(MANIFEST_PHASE, &key)? {
            let previous: Manifest = decode(previous, &key)?;
            if previous != manifest {
                return Err(invariant(format!("meet fragment manifest changed: {key}")));
            }
        } else {
            self.store.journal_done(MANIFEST_PHASE, &key, &manifest)?;
        }
        state.recorded.journal.iter().try_for_each(|entry| {
            self.store.journal_done(
                "meet_spooled_journal_v1",
                &self.journal_key(&entry.phase, &entry.key),
                &entry.payload,
            )
        })?;
        absorb_manifest(state, &manifest)?;
        state.fragments = budget::add(state.fragments, 1)?;
        state.recorded = Recorded::default();
        state.usage = super::RecordingUsage::default();
        state.spool_bytes = 0;
        state.journal_index.clear();
        Ok(())
    }

    pub(super) fn lookup(&self, phase: &str, key: &str) -> CrawlResult<Option<serde_json::Value>> {
        Ok(self
            .store
            .journal_payload("meet_spooled_journal_v1", &self.journal_key(phase, key))?)
    }

    fn journal_key(&self, phase: &str, key: &str) -> String {
        let mut hash = Sha256::new();
        hash.update(phase.as_bytes());
        hash.update([0]);
        hash.update(key.as_bytes());
        format!("{}/{:x}", self.identity, hash.finalize())
    }
}

pub fn read_fragment(store: &Store, identity: &str, index: usize) -> CrawlResult<Recorded> {
    let key = fragment_key(identity, index);
    let expected: Manifest = decode(required(store, MANIFEST_PHASE, &key)?, &key)?;
    budget::check(
        "meet fragment manifest bytes",
        expected.encoded_bytes,
        MAX_OBSERVATION_BATCH_BYTES,
    )?;
    budget::check(
        "meet fragment manifest rows",
        expected.rows,
        MAX_OBSERVATION_BATCH_ROWS,
    )?;
    let recorded: Recorded = decode(required(store, FRAGMENT_PHASE, &key)?, &key)?;
    if manifest(&recorded)? != expected {
        return Err(invariant(format!(
            "meet fragment digest or extent changed: {key}"
        )));
    }
    Ok(recorded)
}

fn required(store: &Store, phase: &str, key: &str) -> CrawlResult<serde_json::Value> {
    store
        .journal_payload(phase, key)?
        .ok_or_else(|| invariant(format!("missing durable meet fragment {phase}/{key}")))
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value, key: &str) -> CrawlResult<T> {
    serde_json::from_value(payload).map_err(|source| CrawlError::Decode {
        url: key.to_string(),
        source,
    })
}

fn manifest(recorded: &Recorded) -> CrawlResult<Manifest> {
    let encoded_bytes = ObservationBudget::request_bytes(recorded)?;
    let rows = row_count(&recorded.rows)?;
    budget::check("meet fragment rows", rows, MAX_OBSERVATION_BATCH_ROWS)?;
    let mut hash = HashWriter(Sha256::new());
    serde_json::to_writer(&mut hash, recorded).map_err(|source| CrawlError::Decode {
        url: "meet fragment digest".into(),
        source,
    })?;
    Ok(Manifest {
        digest: format!("{:x}", hash.0.finalize()),
        encoded_bytes,
        rows,
    })
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn row_count(rows: &[RecordedBatch]) -> CrawlResult<usize> {
    rows.iter()
        .try_fold(0usize, |count, batch| budget::add(count, batch.len()))
}

fn invariant(detail: String) -> CrawlError {
    CrawlError::Invariant { detail }
}

pub fn fragment_key(identity: &str, index: usize) -> String {
    format!("{identity}/{index}")
}

#[cfg(test)]
mod tests;
