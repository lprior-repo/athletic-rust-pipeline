use serde::Deserialize;
use serde_json::Value;

use super::object_wire::{JournalPath, ObjectBounds, RecordRange};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JournalDigest(String);

impl JournalDigest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    pub key: String,
    pub at: String,
    pub payload: Value,
}

#[derive(Debug)]
pub struct JournalRecord {
    sequence: usize,
    range: RecordRange,
    entry: JournalEntry,
}

impl JournalRecord {
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    pub fn offset(&self) -> usize {
        self.range.offset
    }

    pub fn length(&self) -> usize {
        self.range.length
    }

    pub fn entry(&self) -> &JournalEntry {
        &self.entry
    }

    pub fn payload(&self) -> &Value {
        &self.entry.payload
    }
}

pub struct JournalObject<'s> {
    pub(super) phase: String,
    pub(super) key: String,
    pub(super) object: JournalDigest,
    pub(super) bounds: ObjectBounds,
    pub(super) snapshot: fjall::Snapshot,
    pub(super) journal: &'s fjall::Keyspace,
}

impl std::fmt::Debug for JournalObject<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JournalObject")
            .field("phase", &self.phase)
            .field("key", &self.key)
            .field("object", &self.object)
            .field("sequence", &self.snapshot.seqno())
            .finish_non_exhaustive()
    }
}

impl<'s> JournalObject<'s> {
    pub(super) fn new(
        phase: String,
        key: String,
        object: String,
        bounds: ObjectBounds,
        snapshot: fjall::Snapshot,
        journal: &'s fjall::Keyspace,
    ) -> Self {
        Self {
            phase,
            key,
            object: JournalDigest(object),
            bounds,
            snapshot,
            journal,
        }
    }

    pub fn object_id(&self) -> &str {
        self.object.as_str()
    }
    pub fn total_bytes(&self) -> usize {
        self.bounds.total_bytes
    }
    pub fn chunk_count(&self) -> usize {
        self.bounds.chunks
    }
    pub fn record_count(&self) -> usize {
        self.bounds.records
    }

    pub fn records(&self) -> JournalRecords<'_, 's> {
        JournalRecords {
            object: self,
            index: 0,
            ranges: Vec::new(),
            chunk: Vec::new(),
            chunk_index: None,
            failed: false,
        }
    }

    pub(super) fn path(&self) -> JournalPath<'_> {
        JournalPath {
            phase: &self.phase,
            key: &self.key,
        }
    }
}

#[derive(Debug)]
pub struct JournalRecords<'o, 's> {
    object: &'o JournalObject<'s>,
    index: usize,
    ranges: Vec<RecordRange>,
    chunk: Vec<u8>,
    chunk_index: Option<usize>,
    failed: bool,
}

impl JournalRecords<'_, '_> {
    #[cfg(test)]
    pub(crate) fn retained_capacity(&self) -> (usize, usize) {
        (self.ranges.capacity(), self.chunk.capacity())
    }

    fn record_range(&mut self) -> StoreResult<RecordRange> {
        let page = self.index / crate::journal::OBJECT_INDEX_PAGE_ENTRIES;
        let position = self.index % crate::journal::OBJECT_INDEX_PAGE_ENTRIES;
        if position == 0 {
            self.ranges = Vec::new();
            self.ranges = self.object.read_page(page)?;
        }
        self.ranges.get(position).copied().ok_or_else(|| {
            self.object
                .path()
                .refusal("record is absent from its index page")
        })
    }

    fn record_bytes(&mut self, range: RecordRange) -> StoreResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(range.length)
            .map_err(|source| StoreError::Refused {
                detail: format!(
                    "journal record of {} bytes cannot be reserved: {source}",
                    range.length
                ),
            })?;
        let mut offset = range.offset;
        while bytes.len() < range.length {
            let index = offset / crate::journal::OBJECT_CHUNK_BYTES;
            if self.chunk_index != Some(index) {
                self.chunk = Vec::new();
                self.chunk = self.object.read_chunk(index)?;
                self.chunk_index = Some(index);
            }
            let start = offset % crate::journal::OBJECT_CHUNK_BYTES;
            let available = self.chunk.get(start..).ok_or_else(|| {
                self.object
                    .path()
                    .refusal("record range leaves the pinned chunks")
            })?;
            let take = available.len().min(range.length - bytes.len());
            if take == 0 {
                return Err(self
                    .object
                    .path()
                    .refusal("record range leaves the pinned chunks"));
            }
            bytes.extend_from_slice(&available[..take]);
            offset = offset
                .checked_add(take)
                .ok_or(StoreError::CounterOverflow)?;
        }
        Ok(bytes)
    }

    fn read_record(&mut self) -> StoreResult<JournalRecord> {
        let range = self.record_range()?;
        let bytes = self.record_bytes(range)?;
        let entry = serde_json::from_slice(&bytes).map_err(|source| StoreError::Decode {
            key: format!(
                "journal object record {} for {}/{}",
                self.index, self.object.phase, self.object.key
            ),
            source,
        })?;
        Ok(JournalRecord {
            sequence: self.index,
            range,
            entry,
        })
    }
}

impl Iterator for JournalRecords<'_, '_> {
    type Item = StoreResult<JournalRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.index >= self.object.record_count() {
            return None;
        }
        let result = self.read_record();
        match &result {
            Ok(_) => self.index += 1,
            Err(_) => self.failed = true,
        }
        Some(result)
    }
}
