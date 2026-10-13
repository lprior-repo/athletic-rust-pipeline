use std::io::{self, Write};

use serde::Serialize;

use super::{
    checked_key, envelope, refuse_over_journal, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES,
};

pub(crate) fn encode_publication_entry<T: Serialize>(
    phase: &str,
    key: &str,
    payload: &T,
) -> StoreResult<(Vec<u8>, Vec<u8>)> {
    let row_key = checked_key(phase, key)?;
    let raw = bounded_json(phase, key, payload)?;
    let payload = serde_json::from_slice(&raw).map_err(|source| StoreError::Json {
        detail: "decoding bounded publication journal payload".to_owned(),
        source,
    })?;
    let value = bounded_json(phase, key, &envelope(key, payload))?;
    Ok((row_key, value))
}

fn bounded_json<T: Serialize>(phase: &str, key: &str, value: &T) -> StoreResult<Vec<u8>> {
    let mut writer = LimitedWriter::new(MAX_JOURNAL_VALUE_BYTES);
    if let Err(source) = serde_json::to_writer(&mut writer, value) {
        return Err(match writer.failure {
            WriteFailure::None => StoreError::Json {
                detail: "serializing a publication journal".to_owned(),
                source,
            },
            WriteFailure::Limit(bytes) => {
                refuse_over_journal(phase, key, "value", bytes, MAX_JOURNAL_VALUE_BYTES)?;
                StoreError::Invariant {
                    detail: "bounded journal writer refused an admissible size".to_owned(),
                }
            }
            WriteFailure::Allocation(detail) => StoreError::Refused {
                detail: format!("publication journal allocation refused: {detail}"),
            },
        });
    }
    Ok(writer.bytes)
}

enum WriteFailure {
    None,
    Limit(usize),
    Allocation(String),
}

struct LimitedWriter {
    bytes: Vec<u8>,
    limit: usize,
    failure: WriteFailure,
}

impl LimitedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            failure: WriteFailure::None,
        }
    }
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !matches!(self.failure, WriteFailure::None) {
            return Err(io::Error::other(
                "publication journal writer already refused input",
            ));
        }
        let needed = match self.bytes.len().checked_add(bytes.len()) {
            Some(needed) => needed,
            None => {
                self.failure = WriteFailure::Limit(usize::MAX);
                return Err(io::Error::other(
                    "publication journal byte counter overflow",
                ));
            }
        };
        if needed > self.limit {
            self.failure = WriteFailure::Limit(needed);
            return Err(io::Error::other(
                "publication journal exceeds its byte budget",
            ));
        }
        if needed > self.bytes.capacity() {
            let doubled = match self.bytes.capacity().checked_mul(2) {
                Some(capacity) => capacity.min(self.limit),
                None => self.limit,
            };
            let target = doubled.max(256).max(needed).min(self.limit);
            let additional = target
                .checked_sub(self.bytes.len())
                .ok_or_else(|| io::Error::other("publication journal reserve accounting failed"))?;
            if let Err(error) = self.bytes.try_reserve_exact(additional) {
                self.failure = WriteFailure::Allocation(error.to_string());
                return Err(io::Error::other("publication journal allocation refused"));
            }
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
