use super::{converter_bytes, converter_io, CONVERTER_READ_CHUNK};
use crate::{CrawlError, CrawlResult};
use std::path::Path;

pub(super) struct Output {
    bytes: Vec<u8>,
    limit: usize,
}

impl Output {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    pub(super) fn read_size(&self) -> CrawlResult<usize> {
        self.limit
            .checked_sub(self.bytes.len())
            .and_then(|remaining| remaining.checked_add(1))
            .map(|remaining| remaining.min(CONVERTER_READ_CHUNK))
            .ok_or_else(|| converter_bytes(self.limit))
    }

    pub(super) fn append(&mut self, bytes: &[u8], path: &Path) -> CrawlResult<()> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| converter_bytes(self.limit))?;
        if next > self.limit {
            return Err(converter_bytes(self.limit));
        }
        self.reserve(next, path)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn reserve(&mut self, next: usize, path: &Path) -> CrawlResult<()> {
        if next <= self.bytes.capacity() {
            return Ok(());
        }
        let target = self
            .bytes
            .capacity()
            .saturating_mul(2)
            .max(CONVERTER_READ_CHUNK)
            .max(next)
            .min(self.limit);
        let additional = target
            .checked_sub(self.bytes.len())
            .ok_or_else(|| converter_bytes(self.limit))?;
        self.bytes
            .try_reserve_exact(additional)
            .map_err(|error| converter_io(path, std::io::Error::other(error)))
    }

    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

pub(super) fn checked_chunk(chunk: &[u8], size: usize) -> CrawlResult<&[u8]> {
    chunk.get(..size).ok_or_else(|| CrawlError::Invariant {
        detail: "converter read exceeded its supplied buffer".to_owned(),
    })
}
