use crate::{CrawlError, CrawlResult};

const MAX_BYTES: usize = 128 * 1024 * 1024;
const MAX_WORK: usize = 4 * 1024 * 1024;
const MAX_LINES: usize = 20_000;
const TEXT_COPIES: usize = 32;
const LINE_BYTES: usize = 4096;

pub(super) fn check(body: &[u8]) -> CrawlResult<()> {
    limit("raw metadata work bytes", body.len(), MAX_WORK)?;
    let lines = body
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        .checked_add(1)
        .ok_or_else(overflow)?;
    limit("raw metadata lines", lines, MAX_LINES)?;
    let bytes = body
        .len()
        .checked_mul(TEXT_COPIES)
        .and_then(|bytes| {
            lines
                .checked_mul(LINE_BYTES)
                .and_then(|lines| bytes.checked_add(lines))
        })
        .ok_or_else(overflow)?;
    limit("raw metadata decoded allocation", bytes, MAX_BYTES)
}

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
        detail: "raw metadata admission overflow".into(),
    }
}
