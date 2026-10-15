use serde::Serialize;
use sha2::{Digest, Sha256};

use super::encode::RecordBuffer;
use super::{
    bounded, index_digest, refused, reserved, IndexEntry, ObjectPlan, Pass, StagedContextObject,
    CONTEXT_CHUNKS, CONTEXT_PAGES, CONTEXT_RECORDS, CONTEXT_TOTAL_BYTES,
};
use crate::journal::{object_digest, OBJECT_CHUNK_BYTES, OBJECT_INDEX_PAGE_ENTRIES};
use crate::{StoreError, StoreResult};

enum StreamState {
    Accepting,
    Refused,
}

pub(crate) struct ContextRecordSink<'a> {
    pass: Pass<'a>,
    state: StreamState,
    record: RecordBuffer,
    chunk: Vec<u8>,
    page: Vec<IndexEntry>,
    chunk_digests: Vec<String>,
    page_digests: Vec<String>,
    hash: Sha256,
    total_bytes: usize,
    records: usize,
}

impl<'a> ContextRecordSink<'a> {
    pub(crate) fn written_bytes(&self) -> usize {
        self.total_bytes
    }

    #[cfg(test)]
    pub(super) fn retained_capacities(&self) -> [usize; 5] {
        [
            self.record.capacity(),
            self.chunk.capacity(),
            self.page.capacity(),
            self.chunk_digests.capacity(),
            self.page_digests.capacity(),
        ]
    }

    pub(super) fn new(pass: Pass<'a>) -> StoreResult<Self> {
        Ok(Self {
            pass,
            state: StreamState::Accepting,
            record: RecordBuffer::new()?,
            chunk: reserved(OBJECT_CHUNK_BYTES)?,
            page: reserved(OBJECT_INDEX_PAGE_ENTRIES)?,
            chunk_digests: reserved(CONTEXT_CHUNKS)?,
            page_digests: reserved(CONTEXT_PAGES)?,
            hash: Sha256::new(),
            total_bytes: 0,
            records: 0,
        })
    }

    pub(crate) fn record<T: Serialize>(&mut self, record: &T) -> StoreResult<()> {
        self.require_accepting()?;
        let mut buffer = std::mem::replace(&mut self.record, RecordBuffer::empty());
        let result = buffer.encode(record).and_then(|bytes| self.accept(bytes));
        self.record = buffer;
        if result.is_err() {
            self.state = StreamState::Refused;
        }
        result
    }

    fn require_accepting(&self) -> StoreResult<()> {
        match self.state {
            StreamState::Accepting => Ok(()),
            StreamState::Refused => Err(refused("record callback ignored a prior sink failure")),
        }
    }

    fn accept(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let records = self
            .records
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let total = self
            .total_bytes
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        bounded("context records", records, CONTEXT_RECORDS)?;
        bounded("context total bytes", total, CONTEXT_TOTAL_BYTES)?;
        self.hash.update(bytes);
        bytes
            .chunks(OBJECT_CHUNK_BYTES)
            .try_for_each(|piece| self.append(piece))?;
        self.page.push(IndexEntry {
            sequence: self.records,
            offset: self.total_bytes,
            length: bytes.len(),
        });
        self.records = records;
        self.total_bytes = total;
        if self.page.len() == OBJECT_INDEX_PAGE_ENTRIES {
            self.flush_page()?;
        }
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let available = OBJECT_CHUNK_BYTES
            .checked_sub(self.chunk.len())
            .ok_or(StoreError::CounterOverflow)?;
        let (head, tail) = bytes.split_at(available.min(bytes.len()));
        self.chunk.extend_from_slice(head);
        self.flush_full_chunk()?;
        self.chunk.extend_from_slice(tail);
        self.flush_full_chunk()
    }

    fn flush_full_chunk(&mut self) -> StoreResult<()> {
        if self.chunk.len() == OBJECT_CHUNK_BYTES {
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> StoreResult<()> {
        let index = self.chunk_digests.len();
        let count = index.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        bounded("context chunks", count, CONTEXT_CHUNKS)?;
        let digest = object_digest(&self.chunk);
        if let Pass::Stage(target) = &mut self.pass {
            target.stage_chunk(index, &self.chunk, &digest)?;
        }
        self.chunk_digests.push(digest);
        self.chunk.clear();
        Ok(())
    }

    fn flush_page(&mut self) -> StoreResult<()> {
        let page = self.page_digests.len();
        let count = page.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        bounded("context index pages", count, CONTEXT_PAGES)?;
        let digest = index_digest(&self.page)?;
        if let Pass::Stage(target) = &mut self.pass {
            target.stage_page(page, &self.page, &digest)?;
        }
        self.page_digests.push(digest);
        self.page.clear();
        Ok(())
    }

    pub(super) fn finish(mut self) -> StoreResult<ObjectPlan> {
        self.require_accepting()?;
        if !self.chunk.is_empty() || self.total_bytes == 0 {
            self.flush_chunk()?;
        }
        if !self.page.is_empty() {
            self.flush_page()?;
        }
        Ok(ObjectPlan {
            staged: StagedContextObject {
                object: format!("{:x}", self.hash.finalize()),
                total_bytes: self.total_bytes,
                records: self.records,
                chunks: self.chunk_digests.len(),
            },
            chunk_digests: self.chunk_digests,
            page_digests: self.page_digests,
        })
    }
}
