use serde::Deserialize;
use serde_json::Value;

use super::object_wire::{JournalPath, RecordRange};
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

#[derive(Debug)]
pub struct JournalObject {
    phase: String,
    key: String,
    object: JournalDigest,
    total_bytes: usize,
    ranges: Vec<RecordRange>,
    chunks: Vec<Vec<u8>>,
}

impl JournalObject {
    pub(super) fn new(
        phase: String,
        key: String,
        object: String,
        total_bytes: usize,
        ranges: Vec<RecordRange>,
        chunks: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            phase,
            key,
            object: JournalDigest(object),
            total_bytes,
            ranges,
            chunks,
        }
    }

    pub fn object_id(&self) -> &str {
        self.object.as_str()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn record_count(&self) -> usize {
        self.ranges.len()
    }

    pub fn records(&self) -> JournalRecords<'_> {
        JournalRecords {
            object: self,
            index: 0,
            failed: false,
        }
    }

    fn path(&self) -> JournalPath<'_> {
        JournalPath {
            phase: &self.phase,
            key: &self.key,
        }
    }

    fn record_bytes(&self, range: RecordRange) -> StoreResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(range.length)
            .map_err(|source| StoreError::Refused {
                detail: format!(
                    "journal record of {} bytes cannot be reserved: {source}",
                    range.length
                ),
            })?;
        let mut skip = range.offset;
        let mut take = range.length;
        for chunk in &self.chunks {
            if take == 0 {
                break;
            }
            let length = chunk.len();
            if skip >= length {
                skip = skip
                    .checked_sub(length)
                    .ok_or(StoreError::CounterOverflow)?;
                continue;
            }
            let available = length
                .checked_sub(skip)
                .ok_or(StoreError::CounterOverflow)?;
            let span = std::cmp::min(available, take);
            let stop = skip.checked_add(span).ok_or(StoreError::CounterOverflow)?;
            let piece = chunk
                .get(skip..stop)
                .ok_or_else(|| self.path().refusal("record range leaves the pinned chunks"))?;
            bytes.extend_from_slice(piece);
            skip = 0;
            take = take.checked_sub(span).ok_or(StoreError::CounterOverflow)?;
        }
        if take != 0 {
            return Err(self.path().refusal("record range leaves the pinned chunks"));
        }
        Ok(bytes)
    }
}

#[derive(Debug)]
pub struct JournalRecords<'o> {
    object: &'o JournalObject,
    index: usize,
    failed: bool,
}

impl Iterator for JournalRecords<'_> {
    type Item = StoreResult<JournalRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let range = *self.object.ranges.get(self.index)?;
        let sequence = self.index;
        self.index = self.index.saturating_add(1);
        let decoded = self
            .object
            .record_bytes(range)
            .and_then(|bytes| decode_entry(self.object, sequence, &bytes));
        match decoded {
            Ok(entry) => Some(Ok(JournalRecord {
                sequence,
                range,
                entry,
            })),
            Err(error) => {
                self.failed = true;
                Some(Err(error))
            }
        }
    }
}

fn decode_entry(
    object: &JournalObject,
    sequence: usize,
    bytes: &[u8],
) -> StoreResult<JournalEntry> {
    serde_json::from_slice(bytes).map_err(|source| StoreError::Decode {
        key: format!(
            "journal object record {sequence} for {}/{}",
            object.phase, object.key
        ),
        source,
    })
}
