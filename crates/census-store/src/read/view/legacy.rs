use fjall::Readable;
use serde_json::{value::RawValue, Value};

use super::legacy_wire::{self, LegacyBounds, LegacyEntry, LegacyRoot};
use super::object_wire::JournalPath;
use super::StoreSnapshot;
use crate::journal::{chunk_key, OBJECT_RECORD_BYTES_LIMIT};
use crate::{JournalReadBudget, Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

impl StoreSnapshot<'_> {
    pub fn journal_payload(&self, phase: &str, key: &str) -> StoreResult<Option<Value>> {
        let budget = JournalReadBudget::new(OBJECT_RECORD_BYTES_LIMIT)?;
        self.journal_payload_bounded(phase, key, budget)
    }

    pub fn journal_payload_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        self.legacy_payload_bounded(phase, key, budget, legacy_wire::value_entry)
    }

    pub fn journal_payload_raw_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Box<RawValue>>> {
        self.legacy_payload_bounded(phase, key, budget, |path, what, raw| {
            super::legacy_raw::entry(path, what, raw, budget)
        })
    }

    fn legacy_payload_bounded<T>(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
        decode: impl Fn(JournalPath<'_>, &str, &[u8]) -> StoreResult<LegacyEntry<T>>,
    ) -> StoreResult<Option<T>> {
        let row_key = Store::journal_key(phase, key);
        let path = JournalPath { phase, key };
        let Some(raw) = self.legacy_get(path, "physical root", &row_key)? else {
            return Ok(None);
        };
        match legacy_wire::root_with_decoder(path, raw.as_ref(), budget, &decode)? {
            LegacyRoot::Inline(payload) => Ok(Some(payload)),
            LegacyRoot::Chunked(bounds) => {
                let encoded = self.legacy_chunks(path, &row_key, &bounds, &decode)?;
                let entry = decode(path, "reassembled entry", &encoded)?;
                legacy_wire::payload(path, entry).map(Some)
            }
        }
    }

    fn legacy_get(
        &self,
        path: JournalPath<'_>,
        what: &'static str,
        key: &[u8],
    ) -> StoreResult<Option<fjall::Slice>> {
        if let Some(length) = self
            .snapshot
            .size_of(self.journal, key)
            .map_err(|source| StoreError::Read { source })?
        {
            let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
            legacy_wire::limit(path, what, bytes, MAX_JOURNAL_VALUE_BYTES)?;
        }
        self.snapshot
            .get(self.journal, key)
            .map_err(|source| StoreError::Read { source })
    }

    fn legacy_chunks<T>(
        &self,
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
            let raw = self
                .legacy_get(path, "physical chunk", &physical)?
                .ok_or_else(|| path.refusal(&format!("chunk {index} is missing")))?;
            let range = bounds.range(index).ok_or_else(|| {
                path.refusal(&format!("chunk {index} does not cover its manifest"))
            })?;
            let output = encoded.get_mut(range).ok_or_else(|| {
                path.refusal(&format!("chunk {index} exceeds its encoded envelope"))
            })?;
            let value = legacy_wire::chunk_record(path, index, raw.as_ref(), decode)?;
            legacy_wire::chunk_into(path, bounds, index, &value, output)?;
        }
        Ok(encoded)
    }
}
