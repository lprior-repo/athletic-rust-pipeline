use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::limits::ObjectLimits;
use super::{invalid, ObjectRefV2, MAX_RECORD_BYTES};
use crate::public_identity::context::wire::{Entry, Payload};
use crate::{StoreError, StoreResult};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<T> {
    key: String,
    at: String,
    payload: TypedPayload<T>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedPayload<T> {
    kind: String,
    value: T,
}

pub(super) fn encode<T: Serialize>(key: &str, kind: &str, value: &T) -> StoreResult<Vec<u8>> {
    let maximum = MAX_RECORD_BYTES
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    let mut bytes = crate::public_identity::codec::encode(
        &Entry {
            key,
            at: "",
            payload: Payload { kind, value },
        },
        maximum,
    )?;
    bytes
        .try_reserve_exact(1)
        .map_err(|_| invalid("record allocation"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

enum Source<'a, 's> {
    Offline(&'a [u8]),
    Retained(crate::read::JournalRawRecords<'a, 's>),
}

pub(super) struct Records<'a, 's> {
    source: Source<'a, 's>,
    remaining_records: u32,
    sequence: usize,
    offset: usize,
}

impl<'a, 's> Records<'a, 's> {
    pub fn new(bytes: &'a [u8], object: &ObjectRefV2, limits: ObjectLimits) -> StoreResult<Self> {
        object.verify_bytes(bytes, limits)?;
        Ok(Self {
            source: Source::Offline(bytes),
            remaining_records: object.records(),
            sequence: 0,
            offset: 0,
        })
    }

    pub fn retained(
        object: &'a crate::read::JournalObject<'s>,
        reference: &ObjectRefV2,
        limits: ObjectLimits,
    ) -> StoreResult<Self> {
        reference.check_bounds(limits)?;
        if object.object_id() != reference.root().as_str()
            || u64::try_from(object.total_bytes()).map_err(|_| StoreError::CounterOverflow)?
                != u64::from(reference.bytes())
            || u64::try_from(object.record_count()).map_err(|_| StoreError::CounterOverflow)?
                != u64::from(reference.records())
        {
            return Err(invalid("retained object manifest differs from reference"));
        }
        if object.retained_physical_bytes(reference.physical_bytes())? != reference.physical_bytes()
        {
            return Err(invalid("retained physical bytes differ from reference"));
        }
        Ok(Self {
            source: Source::Retained(object.raw_records()),
            remaining_records: reference.records(),
            sequence: 0,
            offset: 0,
        })
    }

    pub fn next<T: DeserializeOwned + Serialize>(
        &mut self,
        key: &str,
        kind: &str,
    ) -> StoreResult<T> {
        if self.remaining_records == 0 {
            return Err(invalid("missing JSONL record"));
        }
        let line = match &mut self.source {
            Source::Offline(remaining) => {
                let boundary = remaining
                    .iter()
                    .take(MAX_RECORD_BYTES)
                    .position(|byte| *byte == b'\n')
                    .and_then(|at| at.checked_add(1))
                    .ok_or_else(|| invalid("record length or missing LF"))?;
                let (line, tail) = remaining.split_at(boundary);
                *remaining = tail;
                std::borrow::Cow::Borrowed(line)
            }
            Source::Retained(records) => {
                let record = records
                    .next()
                    .ok_or_else(|| invalid("missing JSONL record"))??;
                if record.sequence() != self.sequence
                    || record.offset() != self.offset
                    || record.length() != record.bytes().len()
                {
                    return Err(invalid("retained record position"));
                }
                std::borrow::Cow::Owned(record.into_bytes())
            }
        };
        let value = decode_line(&line, key, kind)?;
        self.offset = self
            .offset
            .checked_add(line.len())
            .ok_or(StoreError::CounterOverflow)?;
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        self.remaining_records = self
            .remaining_records
            .checked_sub(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(value)
    }

    pub fn finish(mut self) -> StoreResult<()> {
        let exhausted = match &mut self.source {
            Source::Offline(bytes) => bytes.is_empty(),
            Source::Retained(records) => records.next().transpose()?.is_none(),
        };
        if !exhausted || self.remaining_records != 0 {
            return Err(invalid("extra or missing JSONL records"));
        }
        Ok(())
    }
}

fn decode_line<T: DeserializeOwned + Serialize>(
    line: &[u8],
    key: &str,
    kind: &str,
) -> StoreResult<T> {
    if line.len() > MAX_RECORD_BYTES || line.last() != Some(&b'\n') {
        return Err(invalid("record length or missing LF"));
    }
    if line.contains(&b'\r') {
        return Err(invalid("CR in JSONL record"));
    }
    let record: Envelope<T> =
        serde_json::from_slice(line).map_err(|source| StoreError::Decode {
            key: key.to_owned(),
            source,
        })?;
    if record.key != key || !record.at.is_empty() || record.payload.kind != kind {
        return Err(invalid("typed record grammar"));
    }
    if encode(key, kind, &record.payload.value)?.as_slice() != line {
        return Err(invalid("noncanonical typed JSONL bytes"));
    }
    Ok(record.payload.value)
}
