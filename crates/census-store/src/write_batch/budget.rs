use crate::{StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::ops::Range;

pub const MAX_OBSERVATION_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_OBSERVATION_BATCH_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_OBSERVATION_BATCH_ROWS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationChunk {
    pub rows: Range<usize>,
    pub encoded_bytes: usize,
}

pub struct ObservationBudget;

impl ObservationBudget {
    pub fn record_bytes<T: Serialize + ?Sized>(record: &T) -> StoreResult<usize> {
        measure(record, Limit::Record)
    }

    pub fn request_bytes<T: Serialize + ?Sized>(request: &T) -> StoreResult<usize> {
        measure(request, Limit::Batch)
    }

    pub fn admit<'a, T: Serialize + 'a>(
        records: impl IntoIterator<Item = &'a T>,
        held_rows: usize,
        held_bytes: usize,
    ) -> StoreResult<(usize, usize)> {
        let mut records = records.into_iter();
        let minimum_rows = held_rows
            .checked_add(records.size_hint().0)
            .ok_or(StoreError::CounterOverflow)?;
        check(
            "observation batch rows",
            minimum_rows,
            MAX_OBSERVATION_BATCH_ROWS,
        )?;
        check(
            "observation batch bytes",
            held_bytes,
            MAX_OBSERVATION_BATCH_BYTES,
        )?;
        records.try_fold((held_rows, held_bytes), |(rows, bytes), record| {
            let rows = rows.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            check("observation batch rows", rows, MAX_OBSERVATION_BATCH_ROWS)?;
            let bytes = bytes
                .checked_add(Self::record_bytes(record)?)
                .ok_or(StoreError::CounterOverflow)?;
            check(
                "observation batch bytes",
                bytes,
                MAX_OBSERVATION_BATCH_BYTES,
            )?;
            Ok((rows, bytes))
        })
    }

    pub fn chunks<T: Serialize>(records: &[T]) -> StoreResult<Vec<ObservationChunk>> {
        check("observation manifest rows", records.len(), 50_000)?;
        let mut chunks = Vec::new();
        let mut chunk = ObservationChunk {
            rows: 0..0,
            encoded_bytes: 0,
        };
        for (index, record) in records.iter().enumerate() {
            let bytes = Self::record_bytes(record)?;
            let total = chunk
                .encoded_bytes
                .checked_add(bytes)
                .ok_or(StoreError::CounterOverflow)?;
            if total > MAX_OBSERVATION_BATCH_BYTES || chunk.rows.len() == MAX_OBSERVATION_BATCH_ROWS
            {
                push(&mut chunks, chunk)?;
                chunk = ObservationChunk {
                    rows: index..index,
                    encoded_bytes: 0,
                };
            }
            chunk.rows.end = index.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            chunk.encoded_bytes = chunk
                .encoded_bytes
                .checked_add(bytes)
                .ok_or(StoreError::CounterOverflow)?;
        }
        if !chunk.rows.is_empty() {
            push(&mut chunks, chunk)?;
        }
        Ok(chunks)
    }
}

fn push(chunks: &mut Vec<ObservationChunk>, chunk: ObservationChunk) -> StoreResult<()> {
    chunks.try_reserve(1).map_err(|_| StoreError::Refused {
        detail: "allocating bounded observation manifest".to_string(),
    })?;
    chunks.push(chunk);
    Ok(())
}

pub(super) fn check(what: &'static str, needed: usize, available: usize) -> StoreResult<()> {
    if needed > available {
        return Err(StoreError::BudgetExceeded {
            what,
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(available).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

enum Limit {
    Record,
    Batch,
}

impl Limit {
    fn max(&self) -> usize {
        match self {
            Self::Record => MAX_OBSERVATION_RECORD_BYTES,
            Self::Batch => MAX_OBSERVATION_BATCH_BYTES,
        }
    }

    fn refusal(&self, bytes: usize) -> StoreResult<()> {
        match self {
            Self::Record => Err(StoreError::RowTooLarge {
                bytes,
                max: self.max(),
            }),
            Self::Batch => check("observation request bytes", bytes, self.max()),
        }
    }
}

fn measure<T: Serialize + ?Sized>(record: &T, limit: Limit) -> StoreResult<usize> {
    let mut counter = Counter {
        bytes: 0,
        limit,
        refusal: None,
    };
    let encoded = serde_json::to_writer(&mut counter, record);
    if let Some(error) = counter.refusal {
        return Err(error);
    }
    encoded.map_err(|source| StoreError::Json {
        detail: "measuring observation bytes".to_string(),
        source,
    })?;
    Ok(counter.bytes)
}
struct Counter {
    bytes: usize,
    limit: Limit,
    refusal: Option<StoreError>,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let total = self.bytes.checked_add(bytes.len());
        match total {
            Some(total) if total <= self.limit.max() => {
                self.bytes = total;
                Ok(bytes.len())
            }
            Some(bytes) => {
                self.refusal = self.limit.refusal(bytes).err();
                Err(io::Error::other(
                    "observation record byte capacity exceeded",
                ))
            }
            None => {
                self.refusal = Some(StoreError::CounterOverflow);
                Err(io::Error::other("observation byte counter overflow"))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode<T: Serialize>(record: &T, available: usize) -> StoreResult<Vec<u8>> {
    let mut buffer = Buffer {
        bytes: Vec::new(),
        available,
        refusal: None,
    };
    let encoded = serde_json::to_writer(&mut buffer, record);
    if let Some(error) = buffer.refusal {
        return Err(error);
    }
    encoded.map_err(|source| StoreError::Json {
        detail: "encoding an admitted observation".to_string(),
        source,
    })?;
    Ok(buffer.bytes)
}

struct Buffer {
    bytes: Vec<u8>,
    available: usize,
    refusal: Option<StoreError>,
}

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let result = self.accept(bytes);
        if let Err(error) = result {
            self.refusal = Some(error);
            return Err(io::Error::other("admitted observation encoding refused"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Buffer {
    fn accept(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let total = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if total > MAX_OBSERVATION_RECORD_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: total,
                max: MAX_OBSERVATION_RECORD_BYTES,
            });
        }
        check("observation batch bytes", total, self.available)?;
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(|_| StoreError::Refused {
                detail: "allocating admitted observation bytes".to_string(),
            })?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}
