use super::super::legacy_wire::{self, LegacyBounds, LegacyEntry, LegacyRoot};
use super::super::object_wire::JournalPath;
use super::{RetainedJournalBytes, RetainedJournalReadSource};
use crate::journal::chunk_key;
use crate::{JournalReadBudget, Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

pub(in crate::read::view) fn payload_bounded<T>(
    source: &impl RetainedJournalReadSource,
    phase: &str,
    key: &str,
    budget: JournalReadBudget,
    decode: impl Fn(JournalPath<'_>, &str, &[u8]) -> StoreResult<LegacyEntry<T>>,
) -> StoreResult<Option<T>> {
    let row_key = Store::journal_key(phase, key);
    let path = JournalPath { phase, key };
    let Some(raw) = legacy_get(source, path, "physical root", &row_key)? else {
        return Ok(None);
    };
    match legacy_wire::root_with_decoder(path, raw.as_ref(), budget, &decode)? {
        LegacyRoot::Inline(payload) => Ok(Some(payload)),
        LegacyRoot::Chunked(bounds) => {
            let encoded = legacy_chunks(source, path, &row_key, &bounds, &decode)?;
            let entry = decode(path, "reassembled entry", &encoded)?;
            legacy_wire::payload(path, entry).map(Some)
        }
    }
}

fn legacy_get<'a>(
    source: &'a impl RetainedJournalReadSource,
    path: JournalPath<'_>,
    what: &'static str,
    key: &[u8],
) -> StoreResult<Option<RetainedJournalBytes<'a>>> {
    if let Some(length) = source.journal_size(key)? {
        let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        legacy_wire::limit(path, what, bytes, MAX_JOURNAL_VALUE_BYTES)?;
    }
    source.journal_get(key)
}

pub(in crate::read::view) fn with_payload_bounded<R>(
    source: &impl RetainedJournalReadSource,
    phase: &str,
    key: &str,
    budget: JournalReadBudget,
    visitor: impl Fn(&serde_json::value::RawValue) -> StoreResult<R>,
) -> StoreResult<Option<R>> {
    payload_bounded(source, phase, key, budget, |path, what, raw| {
        super::super::legacy_raw::envelope::projected_entry(path, what, raw, budget, &visitor)
    })
}
fn legacy_chunks<T>(
    source: &impl RetainedJournalReadSource,
    path: JournalPath<'_>,
    row_key: &[u8],
    bounds: &LegacyBounds,
    decode: &impl Fn(JournalPath<'_>, &str, &[u8]) -> StoreResult<LegacyEntry<T>>,
) -> StoreResult<Vec<u8>> {
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(bounds.total_bytes)
        .map_err(|source| StoreError::Refused {
            detail: format!(
                "legacy journal {}/{} cannot reserve {} encoded bytes: {source}",
                path.phase, path.key, bounds.total_bytes
            ),
        })?;
    encoded.resize(bounds.total_bytes, 0);
    for index in 0..bounds.chunks {
        let physical = chunk_key(row_key, index)?;
        let raw = legacy_get(source, path, "physical chunk", &physical)?
            .ok_or_else(|| path.refusal(&format!("chunk {index} is missing")))?;
        let range = bounds
            .range(index)
            .ok_or_else(|| path.refusal(&format!("chunk {index} does not cover its manifest")))?;
        let output = encoded
            .get_mut(range)
            .ok_or_else(|| path.refusal(&format!("chunk {index} exceeds its encoded envelope")))?;
        let value = legacy_wire::chunk_record(path, index, raw.as_ref(), decode)?;
        legacy_wire::chunk_into(path, bounds, index, &value, output)?;
    }
    Ok(encoded)
}
