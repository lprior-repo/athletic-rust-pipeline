use super::codec::{
    check_entity_id, checked_add, corrupt, count, sha256_hex, FrozenCodecError, FrozenCodecResult,
    CHUNK_BYTES_LIMIT, CODEC_REVISION, LINE_BYTES_LIMIT, MAX_CHUNKS_PER_STREAM,
};
use super::inventory::{ChunkDescriptor, StreamInventory};
use super::line::write_line;
use super::stream::{check_identity_order, StreamKind};
use census_store::Entity;

enum Append {
    Stored,
    TooLargeForChunk(u64),
}

pub struct ChunkObject {
    pub index: u64,
    pub bytes: Vec<u8>,
}

pub struct ChunkWriter {
    stream: StreamKind,
    descriptors: Vec<ChunkDescriptor>,
    buffer: Vec<u8>,
    next_ordinal: u64,
    chunk_first: u64,
    chunk_first_key: Option<String>,
    chunk_rows: u64,
    rows: u64,
    bytes: u64,
    previous_key: Option<String>,
}

impl ChunkWriter {
    pub fn new(stream: StreamKind) -> Self {
        Self {
            stream,
            descriptors: Vec::new(),
            buffer: Vec::new(),
            next_ordinal: 0,
            chunk_first: 0,
            chunk_first_key: None,
            chunk_rows: 0,
            rows: 0,
            bytes: 0,
            previous_key: None,
        }
    }

    pub fn rows(&self) -> u64 {
        self.rows
    }

    pub fn push<T: Entity>(
        &mut self,
        ordinal: u64,
        payload: &T,
    ) -> FrozenCodecResult<Option<ChunkObject>> {
        if ordinal != self.next_ordinal {
            return Err(corrupt(
                "frozen stream ordinals",
                &format!(
                    "expected ordinal {} but received {ordinal}",
                    self.next_ordinal
                ),
            ));
        }
        let entity_id = payload.entity_id();
        check_entity_id(entity_id)?;
        check_identity_order(self.stream, self.previous_key.as_deref(), entity_id)?;
        let mut sealed = None;
        match self.append(ordinal, entity_id, payload)? {
            Append::Stored => {}
            Append::TooLargeForChunk(_) => {
                sealed = self.seal()?;
                match self.append(ordinal, entity_id, payload)? {
                    Append::Stored => {}
                    Append::TooLargeForChunk(bytes) => {
                        return Err(FrozenCodecError::BudgetExceeded {
                            what: "frozen chunk",
                            needed: bytes,
                            available: CHUNK_BYTES_LIMIT,
                        });
                    }
                }
            }
        }
        if self.chunk_rows == 0 {
            self.chunk_first = ordinal;
            self.chunk_first_key = Some(entity_id.to_string());
        }
        self.chunk_rows = checked_add("frozen chunk rows", self.chunk_rows, 1)?;
        self.rows = checked_add("frozen stream rows", self.rows, 1)?;
        self.next_ordinal = checked_add("frozen stream ordinals", self.next_ordinal, 1)?;
        self.previous_key = Some(entity_id.to_string());
        Ok(sealed)
    }

    pub fn finish(self) -> FrozenCodecResult<(Option<ChunkObject>, StreamInventory)> {
        let mut writer = self;
        let last = writer.seal()?;
        let inventory =
            StreamInventory::new(writer.stream, writer.descriptors, writer.rows, writer.bytes)?;
        Ok((last, inventory))
    }

    fn append<T: Entity>(
        &mut self,
        ordinal: u64,
        entity_id: &str,
        payload: &T,
    ) -> FrozenCodecResult<Append> {
        let start = self.buffer.len();
        if let Err(error) = write_line(&mut self.buffer, ordinal, entity_id, payload) {
            self.buffer.truncate(start);
            return Err(error);
        }
        let written = self.buffer.len().checked_sub(start).ok_or_else(|| {
            corrupt(
                "frozen chunk",
                "the chunk buffer shrank while a line was appended",
            )
        })?;
        let line_total = checked_add("frozen line", count(written)?, 1)?;
        if line_total > LINE_BYTES_LIMIT {
            self.buffer.truncate(start);
            return Err(FrozenCodecError::RowTooLarge {
                what: "frozen line including its newline",
                bytes: line_total,
                max: LINE_BYTES_LIMIT,
            });
        }
        let chunk_total = checked_add("frozen chunk", count(self.buffer.len())?, 1)?;
        if chunk_total > CHUNK_BYTES_LIMIT {
            self.buffer.truncate(start);
            return Ok(Append::TooLargeForChunk(chunk_total));
        }
        self.buffer.push(b'\n');
        Ok(Append::Stored)
    }

    fn seal(&mut self) -> FrozenCodecResult<Option<ChunkObject>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }
        let index = count(self.descriptors.len())?;
        if index >= MAX_CHUNKS_PER_STREAM {
            return Err(FrozenCodecError::NeedsPartition {
                what: "stream chunk count",
                bytes: index.saturating_add(1),
                max: MAX_CHUNKS_PER_STREAM,
            });
        }
        let bytes = std::mem::take(&mut self.buffer);
        let length = count(bytes.len())?;
        let first_key = self
            .chunk_first_key
            .take()
            .ok_or_else(|| corrupt("frozen chunk", "a chunk buffer is open without a first key"))?;
        let last_key = self
            .previous_key
            .clone()
            .ok_or_else(|| corrupt("frozen chunk", "a chunk buffer is open without a last key"))?;
        let descriptor = ChunkDescriptor {
            index,
            first_ordinal: self.chunk_first,
            rows: self.chunk_rows,
            bytes: length,
            sha256: sha256_hex(&bytes),
            first_key,
            last_key,
            codec_revision: CODEC_REVISION,
        };
        descriptor.validate(index)?;
        self.descriptors
            .try_reserve(1)
            .map_err(|_| FrozenCodecError::BudgetExceeded {
                what: "stream chunk descriptors",
                needed: index.saturating_add(1),
                available: MAX_CHUNKS_PER_STREAM,
            })?;
        self.descriptors.push(descriptor);
        self.bytes = checked_add("frozen stream bytes", self.bytes, length)?;
        self.chunk_rows = 0;
        self.chunk_first = 0;
        self.chunk_first_key = None;
        Ok(Some(ChunkObject { index, bytes }))
    }
}
