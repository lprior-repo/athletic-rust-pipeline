use super::{budget, RecordedJournal, RecordingState, MAX_RECORDED_WORK};
use crate::{CrawlError, CrawlResult};
use sha2::{Digest, Sha256};

pub(super) type JournalDigest = [u8; 32];

pub(super) fn digest(phase: &str, key: &str) -> JournalDigest {
    let mut digest = Sha256::new();
    digest.update(phase.len().to_le_bytes());
    digest.update(phase.as_bytes());
    digest.update(key.as_bytes());
    digest.finalize().into()
}

pub(super) fn lookup<'a>(
    state: &'a RecordingState,
    phase: &str,
    key: &str,
) -> CrawlResult<Option<&'a RecordedJournal>> {
    let Some(index) = state.journal_index.get(&digest(phase, key)) else {
        return Ok(None);
    };
    let entry = state
        .recorded
        .journal
        .get(*index)
        .ok_or_else(|| invariant("pending journal index is outside its recording"))?;
    if entry.phase != phase || entry.key != key {
        return Err(invariant("pending journal digest collision"));
    }
    Ok(Some(entry))
}

pub(super) fn reserve(state: &mut RecordingState, entries: &[RecordedJournal]) -> CrawlResult<()> {
    entries
        .iter()
        .try_for_each(|entry| lookup(state, &entry.phase, &entry.key).map(|_| ()))?;
    state.journal_index.try_reserve(entries.len()).map_err(|_| {
        budget::resource(
            "pending journal index allocation",
            entries.len(),
            MAX_RECORDED_WORK,
        )
    })
}

pub(super) fn append(state: &mut RecordingState, entries: Vec<RecordedJournal>) {
    entries.into_iter().for_each(|entry| {
        state.journal_index.insert(
            digest(&entry.phase, &entry.key),
            state.recorded.journal.len(),
        );
        state.recorded.journal.push(entry);
    });
}

fn invariant(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.to_owned(),
    }
}
