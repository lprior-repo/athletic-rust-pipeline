use super::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const ENROLLMENT_BYTES: usize = 2 * MAX_RESULT_CALENDAR_RECORD_BYTES;
const BUNDLE_BYTES: usize = 64 * 1024 * 1024;
const WRITES_PER_STEP: usize = 32;

pub(super) fn encode(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_bounded(value, ENROLLMENT_BYTES, grant)
}

pub(super) fn encode_bundle(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_bounded(value, BUNDLE_BYTES, grant)
}

pub(super) fn encode_source_closure(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    encode_bounded(value, MAX_RESULT_CALENDAR_SOURCE_PROOF_BYTES, grant)
}

fn encode_bounded(
    value: &(impl Serialize + ?Sized),
    limit: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let (_, size) = serialize(value, io::sink(), limit, None, grant)?;
    grant.charge_bytes(size).map_err(grant_error)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::BudgetExceeded)?;
    let (sink, _) = serialize(
        value,
        ReservedVec { bytes, limit: size },
        limit,
        Some(size),
        grant,
    )?;
    Ok(sink.bytes)
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<T> {
    decode_bounded(bytes, ENROLLMENT_BYTES, grant)
}

pub(super) fn decode_bundle<'a, T: Deserialize<'a> + Serialize>(
    bytes: &'a [u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<T> {
    decode_bounded(bytes, BUNDLE_BYTES, grant)
}

fn decode_bounded<'a, T: Deserialize<'a> + Serialize>(
    bytes: &'a [u8],
    limit: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<T> {
    if bytes.len() > limit {
        return Err(Error::BudgetExceeded.into());
    }
    grant.charge_bytes(bytes.len()).map_err(grant_error)?;
    grant
        .charge_steps(bytes.len().div_ceil(512).max(1))
        .map_err(grant_error)?;
    let value = serde_json::from_slice(bytes).map_err(|_| Error::Corrupt("record decoding"))?;
    let (sink, _) = serialize(
        &value,
        Compare {
            expected: bytes,
            offset: 0,
        },
        limit,
        None,
        grant,
    )?;
    if sink.offset != bytes.len() {
        return Err(Error::Corrupt("noncanonical record").into());
    }
    Ok(value)
}

pub(super) fn digest(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultCalendarDigestV1> {
    grant.charge_bytes(bytes.len()).map_err(grant_error)?;
    grant
        .charge_steps(bytes.len().div_ceil(512).max(1))
        .map_err(grant_error)?;
    finish_digest(Sha256::digest(bytes))
}

fn finish_digest(hash: sha2::digest::Output<Sha256>) -> StoreResult<ResultCalendarDigestV1> {
    let mut hex = [0_u8; 64];
    for (pair, byte) in hex.chunks_exact_mut(2).zip(hash) {
        let alphabet = b"0123456789abcdef";
        let upper = alphabet
            .get(usize::from(byte >> 4))
            .ok_or(Error::Corrupt("digest hexadecimal"))?;
        let lower = alphabet
            .get(usize::from(byte & 15))
            .ok_or(Error::Corrupt("digest hexadecimal"))?;
        pair.copy_from_slice(&[*upper, *lower]);
    }
    let text = std::str::from_utf8(&hex).map_err(|_| Error::Corrupt("digest hexadecimal"))?;
    ResultCalendarDigestV1::parse(text).map_err(grant_error)
}

pub(super) fn serialized_digest(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ResultCalendarDigestV1, usize)> {
    let (writer, bytes) = serialize(value, HashWriter(Sha256::new()), BUNDLE_BYTES, None, grant)?;
    Ok((finish_digest(writer.0.finalize())?, bytes))
}

pub(super) fn raw(
    bytes: Vec<u8>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Box<RawValue>> {
    if bytes.len() > ENROLLMENT_BYTES {
        return Err(Error::BudgetExceeded.into());
    }
    grant.charge_bytes(bytes.len()).map_err(grant_error)?;
    grant
        .charge_steps(bytes.len().div_ceil(512).max(1))
        .map_err(grant_error)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::Corrupt("proof UTF-8"))?;
    RawValue::from_string(text).map_err(|_| Error::Corrupt("proof JSON").into())
}

fn serialize<W: Write>(
    value: &(impl Serialize + ?Sized),
    sink: W,
    limit: usize,
    prepaid: Option<usize>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(W, usize)> {
    grant.charge_steps(1).map_err(grant_error)?;
    let mut writer = Granted {
        sink,
        grant,
        limit,
        prepaid,
        bytes: 0,
        writes: 0,
        failure: None,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if let Some(error) = writer.failure.take() {
        return Err(error);
    }
    result.map_err(|_| Error::Corrupt("record encoding or comparison"))?;
    if writer.prepaid.is_some_and(|remaining| remaining != 0) {
        return Err(Error::Corrupt("encoded size changed").into());
    }
    Ok((writer.sink, writer.bytes))
}

struct Granted<'a, W> {
    sink: W,
    grant: &'a mut AttachmentReplayGrantV1,
    limit: usize,
    prepaid: Option<usize>,
    bytes: usize,
    writes: usize,
    failure: Option<StoreError>,
}

impl<W> Granted<'_, W> {
    fn refuse(&mut self, error: StoreError) -> io::Error {
        self.failure = Some(error);
        io::ErrorKind::InvalidData.into()
    }
    fn charge(&mut self, size: usize) -> io::Result<(usize, usize)> {
        let Some(bytes) = self
            .bytes
            .checked_add(size)
            .filter(|bytes| *bytes <= self.limit)
        else {
            return Err(self.refuse(Error::BudgetExceeded.into()));
        };
        let Some(writes) = self.writes.checked_add(1) else {
            return Err(self.refuse(Error::BudgetExceeded.into()));
        };
        if self.writes.is_multiple_of(WRITES_PER_STEP) {
            if let Err(error) = self.grant.charge_steps(1) {
                return Err(self.refuse(grant_error(error)));
            }
        }
        if let Some(remaining) = self.prepaid {
            let Some(remaining) = remaining.checked_sub(size) else {
                return Err(self.refuse(Error::Corrupt("encoded size changed").into()));
            };
            self.prepaid = Some(remaining);
        } else if let Err(error) = self.grant.charge_bytes(size) {
            return Err(self.refuse(grant_error(error)));
        }
        Ok((bytes, writes))
    }
}

impl<W: Write> Write for Granted<'_, W> {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let (bytes, writes) = self.charge(value.len())?;
        self.sink.write_all(value)?;
        self.bytes = bytes;
        self.writes = writes;
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

struct ReservedVec {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for ReservedVec {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let end = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(io::ErrorKind::InvalidData)?;
        if end > self.limit || end > self.bytes.capacity() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.bytes.extend_from_slice(value);
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Compare<'a> {
    expected: &'a [u8],
    offset: usize,
}
impl Write for Compare<'_> {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or(io::ErrorKind::InvalidData)?;
        if self.expected.get(self.offset..end) != Some(value) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.offset = end;
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
