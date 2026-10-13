use fjall::Readable;
use serde_json::Value;

use super::legacy_wire::{self, LegacyBounds, LegacyRoot};
use super::object_wire::JournalPath;
use super::StoreSnapshot;
use crate::journal::{chunk_key, OBJECT_RECORD_BYTES_LIMIT};
use crate::{JournalReadBudget, Store, StoreError, StoreResult};

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
        let row_key = Store::journal_key(phase, key);
        let Some(raw) = self.legacy_get(&row_key)? else {
            return Ok(None);
        };
        let path = JournalPath { phase, key };
        match legacy_wire::root(path, raw.as_ref(), budget)? {
            LegacyRoot::Inline(payload) => Ok(Some(payload)),
            LegacyRoot::Chunked(bounds) => self.legacy_chunks(path, &row_key, &bounds).map(Some),
        }
    }

    fn legacy_get(&self, key: &[u8]) -> StoreResult<Option<fjall::Slice>> {
        self.snapshot
            .get(self.journal, key)
            .map_err(|source| StoreError::Read { source })
    }

    fn legacy_chunks(
        &self,
        path: JournalPath<'_>,
        row_key: &[u8],
        bounds: &LegacyBounds,
    ) -> StoreResult<Value> {
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
                .legacy_get(&physical)?
                .ok_or_else(|| path.refusal(&format!("chunk {index} is missing")))?;
            let range = bounds.range(index).ok_or_else(|| {
                path.refusal(&format!("chunk {index} does not cover its manifest"))
            })?;
            let output = encoded.get_mut(range).ok_or_else(|| {
                path.refusal(&format!("chunk {index} exceeds its encoded envelope"))
            })?;
            legacy_wire::chunk_into(path, bounds, index, raw.as_ref(), output)?;
        }
        let entry = legacy_wire::record(path, "reassembled entry", &encoded)?;
        legacy_wire::payload(path, entry)
    }
}
