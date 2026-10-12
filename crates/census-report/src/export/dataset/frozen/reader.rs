use super::codec::{
    checked_add, corrupt, count, FrozenCodecError, FrozenCodecResult, CHUNK_BYTES_LIMIT,
    LINE_BYTES_LIMIT,
};
use super::line::{decode_line, DecodedLine};
use super::stream::{check_identity_order, StreamKind};
use census_store::Entity;

pub struct ChunkReader<'a> {
    stream: StreamKind,
    bytes: &'a [u8],
    position: usize,
    next_ordinal: u64,
    previous_key: Option<String>,
    rows: u64,
}

impl<'a> ChunkReader<'a> {
    pub fn open(
        stream: StreamKind,
        bytes: &'a [u8],
        first_ordinal: u64,
    ) -> FrozenCodecResult<Self> {
        let size = count(bytes.len())?;
        if size > CHUNK_BYTES_LIMIT {
            return Err(FrozenCodecError::RowTooLarge {
                what: "frozen chunk",
                bytes: size,
                max: CHUNK_BYTES_LIMIT,
            });
        }
        if bytes.is_empty() {
            return Err(corrupt("frozen chunk", "a chunk object must not be empty"));
        }
        if let Err(source) = std::str::from_utf8(bytes) {
            return Err(corrupt(
                "frozen chunk",
                &format!("a chunk object must be valid utf-8: {source}"),
            ));
        }
        let mut segments = bytes.split(|byte| *byte == b'\n').peekable();
        while let Some(line) = segments.next() {
            if segments.peek().is_none() {
                if !line.is_empty() {
                    return Err(corrupt(
                        "frozen chunk",
                        "a chunk object must end with exactly one newline",
                    ));
                }
                break;
            }
            if line.is_empty() {
                return Err(corrupt(
                    "frozen chunk",
                    "a chunk object must not hold blank lines",
                ));
            }
            let total = checked_add("frozen line", count(line.len())?, 1)?;
            if total > LINE_BYTES_LIMIT {
                return Err(FrozenCodecError::RowTooLarge {
                    what: "frozen line including its newline",
                    bytes: total,
                    max: LINE_BYTES_LIMIT,
                });
            }
        }
        Ok(Self {
            stream,
            bytes,
            position: 0,
            next_ordinal: first_ordinal,
            previous_key: None,
            rows: 0,
        })
    }

    pub fn rows(&self) -> u64 {
        self.rows
    }

    fn peek_line(&self) -> FrozenCodecResult<Option<&'a [u8]>> {
        if self.position >= self.bytes.len() {
            return Ok(None);
        }
        let rest = self
            .bytes
            .get(self.position..)
            .ok_or_else(|| corrupt("frozen chunk", "the chunk cursor left the object"))?;
        let end = rest.iter().position(|byte| *byte == b'\n').ok_or_else(|| {
            corrupt(
                "frozen chunk",
                "a chunk object must end with exactly one newline",
            )
        })?;
        let line = rest
            .get(..end)
            .ok_or_else(|| corrupt("frozen chunk", "the chunk cursor left the object"))?;
        Ok(Some(line))
    }

    pub fn next_line(&mut self) -> FrozenCodecResult<Option<&'a [u8]>> {
        let Some(line) = self.peek_line()? else {
            return Ok(None);
        };
        let consumed = line
            .len()
            .checked_add(1)
            .ok_or_else(|| corrupt("frozen chunk", "a chunk offset exceeded usize"))?;
        self.position = self
            .position
            .checked_add(consumed)
            .ok_or_else(|| corrupt("frozen chunk", "the chunk cursor overflowed"))?;
        self.next_ordinal = checked_add("frozen stream ordinals", self.next_ordinal, 1)?;
        self.rows = checked_add("frozen chunk rows", self.rows, 1)?;
        Ok(Some(line))
    }

    pub fn next_record<T: Entity>(&mut self) -> FrozenCodecResult<Option<DecodedLine<T>>> {
        let expected = self.next_ordinal;
        let Some(line) = self.next_line()? else {
            return Ok(None);
        };
        let decoded = decode_line::<T>(line)?;
        if decoded.ordinal != expected {
            return Err(corrupt(
                "frozen stream ordinals",
                &format!("expected ordinal {expected} but read {}", decoded.ordinal),
            ));
        }
        check_identity_order(
            self.stream,
            self.previous_key.as_deref(),
            &decoded.entity_id,
        )?;
        self.previous_key = Some(decoded.entity_id.clone());
        Ok(Some(decoded))
    }
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod window_tests;

#[path = "window.rs"]
mod window;
pub(super) use window::EncodedWindowLimit;

#[path = "cursor.rs"]
mod cursor;
pub(super) use cursor::TableCursor;
