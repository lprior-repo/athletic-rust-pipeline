use super::super::{OwnedPerformance, OwnedRejection, MAX_OWNED_BODY_BYTES, MAX_OWNED_ROWS};
use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use std::io::{self, Write};

pub(super) const MAX_DECODE_BYTES: usize = 128 * 1024 * 1024;
const MAX_WORK: usize = 2_000_000;
const MAX_DEPTH: usize = 128;
const VALUE_NODE_BYTES: usize = 512;
const TEXT_STORAGE_COPIES: usize = 32;

#[derive(Clone, Copy, Default)]
struct Frame {
    array: bool,
    next: bool,
    values: usize,
}

struct Admission {
    frames: [Frame; MAX_DEPTH],
    depth: usize,
    quoted: bool,
    escaped: bool,
    tokens: usize,
}

pub(super) fn check(body: &[u8]) -> CrawlResult<()> {
    limit("owned capture bytes", body.len(), MAX_OWNED_BODY_BYTES)?;
    let state = Admission {
        frames: [Frame::default(); MAX_DEPTH],
        depth: 0,
        quoted: false,
        escaped: false,
        tokens: 0,
    };
    let admitted = body.iter().try_fold(state, |mut state, byte| {
        state.accept(*byte)?;
        Ok::<_, CrawlError>(state)
    })?;
    let text = body
        .len()
        .checked_mul(TEXT_STORAGE_COPIES)
        .ok_or_else(overflow)?;
    let nodes = admitted
        .tokens
        .checked_mul(VALUE_NODE_BYTES)
        .ok_or_else(overflow)?;
    let vectors = MAX_OWNED_ROWS
        .checked_mul(
            std::mem::size_of::<OwnedPerformance>()
                .saturating_add(std::mem::size_of::<OwnedRejection>()),
        )
        .ok_or_else(overflow)?;
    let requested = text
        .checked_add(nodes)
        .and_then(|size| size.checked_add(vectors))
        .ok_or_else(overflow)?;
    limit("owned decoded allocation", requested, MAX_DECODE_BYTES)
}

impl Admission {
    fn accept(&mut self, byte: u8) -> CrawlResult<()> {
        if matches!(byte, b'[' | b']' | b'{' | b'}' | b':' | b',' | b'"') {
            self.tokens = self.tokens.checked_add(1).ok_or_else(overflow)?;
            limit("owned decode work", self.tokens, MAX_WORK)?;
        }
        if self.quoted {
            self.string(byte);
            return Ok(());
        }
        self.value(byte)?;
        match byte {
            b'"' => self.quoted = true,
            b'[' | b'{' => self.open(byte == b'[')?,
            b']' | b'}' => self.depth = self.depth.saturating_sub(1),
            b',' => {
                if let Some(frame) = self.top() {
                    frame.next = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn string(&mut self, byte: u8) {
        if self.escaped {
            self.escaped = false;
        } else if byte == b'\\' {
            self.escaped = true;
        } else if byte == b'"' {
            self.quoted = false;
        }
    }

    fn value(&mut self, byte: u8) -> CrawlResult<()> {
        let Some(frame) = self.top() else {
            return Ok(());
        };
        if frame.array && frame.next && !byte.is_ascii_whitespace() && !matches!(byte, b']' | b',')
        {
            frame.values = frame.values.checked_add(1).ok_or_else(overflow)?;
            limit("owned array values", frame.values, MAX_OWNED_ROWS)?;
            frame.next = false;
        }
        Ok(())
    }

    fn open(&mut self, array: bool) -> CrawlResult<()> {
        let requested = self.depth.checked_add(1).ok_or_else(overflow)?;
        limit("owned JSON depth", requested, MAX_DEPTH)?;
        let frame = self.frames.get_mut(self.depth).ok_or_else(overflow)?;
        *frame = Frame {
            array,
            next: true,
            values: 0,
        };
        self.depth = requested;
        Ok(())
    }

    fn top(&mut self) -> Option<&mut Frame> {
        self.depth
            .checked_sub(1)
            .and_then(|index| self.frames.get_mut(index))
    }
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
