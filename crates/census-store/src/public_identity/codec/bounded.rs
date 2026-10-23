use crate::{StoreError, StoreResult};
use serde::Serialize;
use std::io::{self, Write};

pub(in crate::public_identity) fn encode_with_context<T: Serialize + ?Sized>(
    value: &T,
    max: usize,
    context: &'static str,
) -> StoreResult<Vec<u8>> {
    let mut writer = Bounded::new(
        Buffer {
            bytes: Vec::new(),
            max,
        },
        max,
        context,
    );
    let result = serde_json::to_writer(&mut writer, value);
    writer.complete(result)?;
    Ok(writer.output.bytes)
}

pub(in crate::public_identity) fn encoded_len<T: Serialize + ?Sized>(
    value: &T,
    max: usize,
    context: &'static str,
) -> StoreResult<usize> {
    let mut writer = Bounded::new(io::sink(), max, context);
    let result = serde_json::to_writer(&mut writer, value);
    writer.complete(result)?;
    Ok(writer.used)
}

pub(super) fn next(
    used: usize,
    additional: usize,
    max: usize,
    context: &'static str,
) -> StoreResult<usize> {
    let needed = used
        .checked_add(additional)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > max {
        return Err(StoreError::BudgetExceeded {
            what: context,
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(max).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(needed)
}

pub(super) struct Bounded<W> {
    output: W,
    used: usize,
    max: usize,
    context: &'static str,
    failure: Option<StoreError>,
}

impl<W> Bounded<W> {
    pub(super) fn new(output: W, max: usize, context: &'static str) -> Self {
        Self {
            output,
            used: 0,
            max,
            context,
            failure: None,
        }
    }

    pub(super) fn complete(&mut self, result: serde_json::Result<()>) -> StoreResult<()> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        result.map_err(|source| StoreError::Json {
            detail: self.context.to_owned(),
            source,
        })
    }

    fn reject(&mut self, error: StoreError) -> io::Error {
        self.failure = Some(error);
        io::Error::other("bounded public identity serialization refused")
    }

    fn reject_io(&mut self, source: io::Error) -> io::Error {
        self.reject(StoreError::Json {
            detail: self.context.to_owned(),
            source: serde_json::Error::io(source),
        })
    }
}

impl<W: Write> Write for Bounded<W> {
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other(
                "bounded public identity serialization refused",
            ));
        }
        let needed = match next(self.used, raw.len(), self.max, self.context) {
            Ok(needed) => needed,
            Err(error) => return Err(self.reject(error)),
        };
        if let Err(error) = self.output.write_all(raw) {
            return Err(self.reject_io(error));
        }
        self.used = needed;
        Ok(raw.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.failure.is_some() {
            return Err(io::Error::other(
                "bounded public identity serialization refused",
            ));
        }
        self.output.flush().map_err(|source| self.reject_io(source))
    }
}

struct Buffer {
    bytes: Vec<u8>,
    max: usize,
}

impl Write for Buffer {
    fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
        let needed = self
            .bytes
            .len()
            .checked_add(raw.len())
            .ok_or_else(|| io::Error::other("public identity byte overflow"))?;
        if needed > self.bytes.capacity() {
            let doubled = self
                .bytes
                .capacity()
                .checked_mul(2)
                .map_or(self.max, |value| value.min(self.max));
            let target = doubled.max(256).max(needed).min(self.max);
            let additional = target
                .checked_sub(self.bytes.len())
                .ok_or_else(|| io::Error::other("public identity reserve accounting"))?;
            self.bytes
                .try_reserve_exact(additional)
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(raw);
        Ok(raw.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
