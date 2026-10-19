use std::io::{self, Write};

use census_domain::model::serialized_digest;
use restate_sdk::prelude::{HandlerError, TerminalError};

use crate::restate_services::wire::{
    OpenWorkReply, SourceObjectPageReply, SourceObjectPageRequest,
};

const MAX_PAGE: u64 = 4096;
const DEFAULT_PAGE: u64 = 256;
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

fn validate_limit(page: Option<&SourceObjectPageRequest>) -> Result<u64, TerminalError> {
    if let Some(p) = page {
        if let Some(requested) = p.limit {
            let requested = u64::from(requested);
            if !(1..=MAX_PAGE).contains(&requested) {
                return Err(TerminalError::new_with_code(400, "invalid page limit"));
            }
            return Ok(requested);
        }
    }
    Ok(DEFAULT_PAGE)
}

fn resolve_offset(
    expected_digest: &str,
    page: Option<&SourceObjectPageRequest>,
) -> Result<u64, TerminalError> {
    if let Some(p) = page {
        if let Some(after) = &p.after {
            let parts = after
                .split_once(':')
                .ok_or_else(|| TerminalError::new_with_code(400, "malformed paging cursor"))?;
            let (cursor_digest, offset_str) = parts;
            if cursor_digest != expected_digest {
                return Err(TerminalError::new_with_code(
                    400,
                    "paging cursor scope mismatch",
                ));
            }
            let offset: u64 = offset_str
                .parse()
                .map_err(|_| TerminalError::new_with_code(400, "malformed paging cursor"))?;
            return Ok(offset);
        }
    }
    Ok(0)
}

struct BoundedCounter {
    written: u64,
    limit: u64,
    overflow: bool,
}

impl BoundedCounter {
    fn new(limit: u64) -> Self {
        Self {
            written: 0,
            limit,
            overflow: false,
        }
    }

    fn is_overflow(&self) -> bool {
        self.overflow
    }

    fn exact_count(&self) -> u64 {
        self.written
    }
}

impl Write for BoundedCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_len = u64::try_from(buf.len()).map_err(io::Error::other)?;
        self.written = self
            .written
            .checked_add(buf_len)
            .ok_or_else(|| io::Error::other("diagnostic byte counter exhausted"))?;
        if self.written > self.limit {
            self.overflow = true;
            return Err(io::Error::other("exceeded byte limit"));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn count_response_bytes(reply: &OpenWorkReply, limit: u64) -> Result<u64, TerminalError> {
    let mut counter = BoundedCounter::new(limit);
    serde_json::to_writer(&mut counter, reply).map_err(|e| {
        if counter.is_overflow() {
            TerminalError::new_with_code(
                413,
                format!(
                    "diagnostic response exceeds {limit} encoded bytes; request a smaller page"
                ),
            )
        } else {
            TerminalError::new_with_code(500, format!("response serialization failed: {e}"))
        }
    })?;
    Ok(counter.exact_count())
}

pub(in crate::restate_services) fn bounded_reply(
    reply: OpenWorkReply,
    page: Option<&SourceObjectPageRequest>,
) -> Result<OpenWorkReply, HandlerError> {
    bounded_with_bytes(reply, page, MAX_RESPONSE_BYTES).map_err(Into::into)
}

fn bounded_with_bytes(
    mut reply: OpenWorkReply,
    page: Option<&SourceObjectPageRequest>,
    max_response_bytes: u64,
) -> Result<OpenWorkReply, TerminalError> {
    let limit = validate_limit(page)?;

    let season = reply.season.as_str();
    let revision = reply.revision;
    let total = u64::try_from(reply.endpoints.len())
        .map_err(|_| TerminalError::new_with_code(500, "diagnostic inventory counter exhausted"))?;

    let expected_digest =
        serialized_digest(&(season, revision, &reply.endpoints)).map_err(|e| {
            TerminalError::new_with_code(500, format!("endpoint serialization failed: {e}"))
        })?;

    let offset = resolve_offset(&expected_digest, page)?;

    if offset > total {
        return Err(TerminalError::new_with_code(
            400,
            "paging cursor offset beyond inventory",
        ));
    }

    let actual_limit = std::cmp::min(limit, total - offset);
    let slice_start = usize::try_from(offset)
        .map_err(|_| TerminalError::new_with_code(500, "offset usize conversion failed"))?;
    let end = offset
        .checked_add(actual_limit)
        .ok_or_else(|| TerminalError::new_with_code(500, "diagnostic page counter exhausted"))?;
    let slice_end = usize::try_from(end)
        .map_err(|_| TerminalError::new_with_code(500, "slice end usize conversion failed"))?;
    let selected_count = usize::try_from(actual_limit)
        .map_err(|_| TerminalError::new_with_code(500, "page size usize conversion failed"))?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(selected_count)
        .map_err(|_| TerminalError::new_with_code(500, "diagnostic page allocation refused"))?;

    selected.extend(reply.endpoints.drain(slice_start..slice_end));

    let next = if end < total {
        Some(format!("{expected_digest}:{end}"))
    } else {
        None
    };

    reply.endpoints = selected;
    reply.endpoint_page = Some(SourceObjectPageReply {
        total,
        offset,
        returned: actual_limit,
        next,
    });

    count_response_bytes(&reply, max_response_bytes)?;

    Ok(reply)
}

#[cfg(test)]
mod tests;
