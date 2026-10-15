use std::io::{self, Write};

use serde::Serialize;

use super::{bounded, refused, reserved, CONTEXT_RECORD_BYTES};
use crate::{StoreError, StoreResult};

pub(super) struct RecordBuffer {
    bytes: Vec<u8>,
    exceeded: Option<usize>,
}

impl RecordBuffer {
    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    pub(super) fn new() -> StoreResult<Self> {
        Ok(Self {
            bytes: reserved(CONTEXT_RECORD_BYTES)?,
            exceeded: None,
        })
    }

    pub(super) fn empty() -> Self {
        Self {
            bytes: Vec::new(),
            exceeded: None,
        }
    }

    pub(super) fn encode<T: Serialize>(&mut self, record: &T) -> StoreResult<&[u8]> {
        self.bytes.clear();
        self.exceeded = None;
        let encoded = serde_json::to_writer(&mut *self, record);
        if let Some(needed) = self.exceeded {
            bounded("context record bytes", needed, CONTEXT_RECORD_BYTES)?;
        }
        encoded.map_err(|source| StoreError::Json {
            detail: "serializing bounded context record".to_string(),
            source,
        })?;
        if self.bytes.iter().any(|byte| matches!(byte, b'\n' | b'\r')) {
            return Err(refused("record contains an unescaped JSONL line break"));
        }
        serde_json::from_slice::<serde::de::IgnoredAny>(&self.bytes).map_err(|source| {
            StoreError::Json {
                detail: "validating serialized context record".to_string(),
                source,
            }
        })?;
        self.bytes.push(b'\n');
        Ok(&self.bytes)
    }
}

impl Write for RecordBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let needed = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .and_then(|length| length.checked_add(1))
            .ok_or_else(|| io::Error::other("context record byte counter overflow"))?;
        if needed > CONTEXT_RECORD_BYTES {
            self.exceeded = Some(needed);
            return Err(io::Error::other("context record byte budget exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
