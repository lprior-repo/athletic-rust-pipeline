use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use std::io::{self, Write};

pub(super) const WINDOW_ROWS: usize = 64;
pub(super) const WINDOW_BYTES: usize = 8 * 1024 * 1024;
pub(super) const WINDOW_WORK: usize = 131_072;
pub(super) const METADATA_LABELS: usize = 4096;
pub(super) const SCHOOL_ROWS: usize = 65_536;
pub(super) const SCHOOL_BYTES: usize = 64 * 1024 * 1024;
const COPY_BOUND: usize = 64;
const ROW_OVERHEAD: usize = 32 * 1024;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Footprint {
    pub(super) bytes: usize,
    pub(super) work: usize,
}

impl Write for Footprint {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self.bytes.checked_add(bytes.len()).ok_or_else(exhausted)?;
        self.work = self.work.checked_add(1).ok_or_else(exhausted)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Footprint {
    pub(super) fn of(value: &impl Serialize) -> CrawlResult<Self> {
        let mut footprint = Self::default();
        serde_json::to_writer(&mut footprint, value).map_err(|source| CrawlError::Encode {
            table: "result admission".into(),
            source,
        })?;
        Ok(footprint)
    }

    pub(super) fn projection(self, context: Self) -> CrawlResult<Self> {
        let source = self.checked_add(context)?;
        Ok(Self {
            bytes: source
                .bytes
                .checked_mul(COPY_BOUND)
                .and_then(|bytes| bytes.checked_add(ROW_OVERHEAD))
                .ok_or_else(overflow)?,
            work: source.work.checked_mul(COPY_BOUND).ok_or_else(overflow)?,
        })
    }

    pub(super) fn checked_add(self, other: Self) -> CrawlResult<Self> {
        Ok(Self {
            bytes: self.bytes.checked_add(other.bytes).ok_or_else(overflow)?,
            work: self.work.checked_add(other.work).ok_or_else(overflow)?,
        })
    }

    pub(super) fn fits(self) -> bool {
        self.bytes <= WINDOW_BYTES && self.work <= WINDOW_WORK
    }
}

fn exhausted() -> io::Error {
    io::Error::other("result admission counter overflow")
}

fn overflow() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "result admission counter overflow".into(),
    }
}

pub(super) fn reserve(error: std::collections::TryReserveError) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("result buffer allocation refused: {error}"),
    }
}
