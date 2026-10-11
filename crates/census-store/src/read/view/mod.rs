use crate::keys::Layout;
use crate::{Store, StoreError, StoreResult};
use fjall::{Readable, Snapshot};

mod merge;
mod operations;
pub struct StoreSnapshot<'s> {
    snapshot: Snapshot,
    entities: &'s fjall::Keyspace,
    journal: &'s fjall::Keyspace,
    root: &'s std::path::Path,
    derived_generation: u64,
    evidence_generation: u64,
}

impl<'s> StoreSnapshot<'s> {
    pub(crate) fn new(
        snapshot: Snapshot,
        entities: &'s fjall::Keyspace,
        journal: &'s fjall::Keyspace,
        root: &'s std::path::Path,
        derived_generation: u64,
        evidence_generation: u64,
    ) -> Self {
        Self {
            snapshot,
            entities,
            journal,
            root,
            derived_generation,
            evidence_generation,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.snapshot.seqno()
    }

    pub fn derived_generation(&self) -> u64 {
        self.derived_generation
    }

    pub fn evidence_generation(&self) -> u64 {
        self.evidence_generation
    }

    fn layout(&self) -> Layout {
        Layout {
            derived_generation: self.derived_generation,
        }
    }

    pub fn journal_payload(
        &self,
        phase: &str,
        key: &str,
    ) -> StoreResult<Option<serde_json::Value>> {
        let raw = self
            .snapshot
            .get(self.journal, Store::journal_key(phase, key))
            .map_err(|source| StoreError::Read { source })?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let row_key = Store::journal_key(phase, key);
        super::decode_journal_entry(phase, key, raw.as_ref(), |index| {
            let chunk_key = crate::journal::chunk_key(&row_key, index)?;
            let chunk = self
                .snapshot
                .get(self.journal, chunk_key)
                .map_err(|source| StoreError::Read { source })?;
            chunk
                .map(|chunk| super::decode_journal_chunk(phase, key, index, chunk.as_ref()))
                .transpose()
        })
        .map(Some)
    }
}

#[cfg(test)]
mod tests;
