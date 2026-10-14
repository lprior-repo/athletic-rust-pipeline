use super::{MAX_DECODE_BYTES, VALUE_NODE_BYTES};
use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use std::io::{self, Write};

fn limit(resource: &'static str, requested: usize, maximum: usize) -> CrawlResult<()> {
    if requested > maximum {
        return Err(CrawlError::Resource {
            resource,
            requested,
            limit: maximum,
        });
    }
    Ok(())
}

fn overflow() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "owned decode admission overflow".into(),
    }
}

#[derive(Default)]
struct Encoded {
    bytes: usize,
    work: usize,
}

impl Write for Encoded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("owned recording byte overflow"))?;
        self.work = self
            .work
            .checked_add(1)
            .ok_or_else(|| io::Error::other("owned recording work overflow"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(in crate::milesplit::owned) fn recording(
    ctx: &crate::AdapterContext<'_>,
    source: &impl Serialize,
) -> CrawlResult<()> {
    let mut size = Encoded::default();
    serde_json::to_writer(&mut size, source).map_err(|source| CrawlError::Encode {
        table: "owned recording admission".into(),
        source,
    })?;
    let nodes = size
        .work
        .checked_mul(VALUE_NODE_BYTES)
        .ok_or_else(overflow)?;
    let bytes = size
        .bytes
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(nodes))
        .and_then(|bytes| bytes.checked_add(16 * 1024))
        .ok_or_else(overflow)?;
    limit("owned interpretation allocation", bytes, MAX_DECODE_BYTES)?;
    if let Some(recording) = ctx.recording {
        recording.admit(bytes, 1)?;
    }
    Ok(())
}
