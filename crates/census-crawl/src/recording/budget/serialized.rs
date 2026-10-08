use super::{add, check, multiply, MAX_RECORDED_BYTES, MAX_RECORDED_WORK};
use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use std::io::{Error, Result as IoResult, Write};

const STORAGE_PER_TOKEN: usize = 192;

#[derive(Clone, Copy, Default)]
enum Lexical {
    #[default]
    Outside,
    String,
    Escape,
    Scalar,
}

#[derive(Clone, Copy, Default)]
struct Counter {
    bytes: usize,
    tokens: usize,
    depth: usize,
    lexical: Lexical,
}

impl Counter {
    fn byte(self, byte: u8) -> CrawlResult<Self> {
        let (lexical, token) = match (self.lexical, byte) {
            (Lexical::String, b'\\') => (Lexical::Escape, 0),
            (Lexical::String, b'"') => (Lexical::Outside, 0),
            (Lexical::String, _) | (Lexical::Escape, _) => (Lexical::String, 0),
            (Lexical::Outside, b'"') => (Lexical::String, 1),
            (
                Lexical::Outside | Lexical::Scalar,
                b'}' | b']' | b',' | b':' | b' ' | b'\n' | b'\r' | b'\t',
            ) => (Lexical::Outside, 0),
            (Lexical::Outside, b'{') => (Lexical::Outside, 4),
            (Lexical::Outside, b'[') => (Lexical::Outside, 1),
            (Lexical::Outside, _) => (Lexical::Scalar, 1),
            (Lexical::Scalar, _) => (Lexical::Scalar, 0),
        };
        let depth = match (self.lexical, byte) {
            (Lexical::Outside, b'{' | b'[') => add(self.depth, 1)?,
            (Lexical::Outside | Lexical::Scalar, b'}' | b']') => {
                self.depth.checked_sub(1).ok_or_else(|| {
                    super::resource("recorded JSON depth", usize::MAX, super::values::MAX_DEPTH)
                })?
            }
            _ => self.depth,
        };
        check("recorded JSON depth", depth, super::values::MAX_DEPTH)?;
        let tokens = add(self.tokens, token)?;
        check("recording serialization work", tokens, MAX_RECORDED_WORK)?;
        Ok(Self {
            lexical,
            tokens,
            depth,
            ..self
        })
    }

    fn accept(self, bytes: &[u8]) -> CrawlResult<Self> {
        let length = add(self.bytes, bytes.len())?;
        check("recording serialization bytes", length, MAX_RECORDED_BYTES)?;
        bytes.iter().try_fold(
            Self {
                bytes: length,
                ..self
            },
            |counter, byte| counter.byte(*byte),
        )
    }
}

#[derive(Default)]
struct Meter {
    counter: Counter,
    failure: Option<CrawlError>,
}

impl Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> IoResult<usize> {
        match self.counter.accept(bytes) {
            Ok(counter) => {
                self.counter = counter;
                Ok(bytes.len())
            }
            Err(error) => {
                self.failure = Some(error);
                Err(Error::other("recording admission failed"))
            }
        }
    }
    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

pub(crate) fn preflight<T: Serialize + ?Sized>(value: &T) -> CrawlResult<usize> {
    let mut meter = Meter::default();
    let encoded = serde_json::to_writer(&mut meter, value);
    if let Some(error) = meter.failure {
        return Err(error);
    }
    encoded.map_err(|source| CrawlError::Encode {
        table: "recording admission".into(),
        source,
    })?;
    let estimate = add(
        meter.counter.bytes,
        multiply(meter.counter.tokens, STORAGE_PER_TOKEN)?,
    )?;
    check(
        "recording serialization storage",
        estimate,
        MAX_RECORDED_BYTES,
    )?;
    Ok(estimate)
}

pub(crate) fn preflight_rows<T: Serialize>(rows: &[T]) -> CrawlResult<usize> {
    check("recorded effects", rows.len(), MAX_RECORDED_WORK)?;
    let storage = multiply(rows.len(), std::mem::size_of::<serde_json::Value>())?;
    rows.iter().try_fold(storage, |total, row| {
        let estimate = add(total, preflight(row)?)?;
        check(
            "recording serialization storage",
            estimate,
            MAX_RECORDED_BYTES,
        )?;
        Ok(estimate)
    })
}
