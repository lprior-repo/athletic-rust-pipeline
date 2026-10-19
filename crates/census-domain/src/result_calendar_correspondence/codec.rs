use super::*;
use crate::model::write_serialized;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const SERIALIZER_WRITES_PER_STEP: usize = 32;

pub(super) fn with_grant<T>(
    grant: &mut AttachmentReplayGrantV1,
    read: impl FnOnce(&mut AttachmentReplayGrantV1) -> Result<T>,
) -> Result<T> {
    grant.ensure_available()?;
    let result = read(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn admit(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    serialized(value, io::sink(), grant, None)?;
    Ok(())
}

pub(super) fn encode(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    let (_, size) = serialized(value, io::sink(), grant, None)?;
    grant.charge_bytes(size)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| ResultCalendarCorrespondenceError::Capacity)?;
    let sink = ReservedVec { bytes, limit: size };
    let (sink, _) = serialized(value, sink, grant, Some(size))?;
    Ok(sink.bytes)
}

pub(super) fn digest(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarDigestV1> {
    let (sink, _) = serialized(value, HashWriter(Sha256::new()), grant, None)?;
    let digest = sink.0.finalize();
    let mut encoded = [0_u8; 64];
    for (pair, byte) in encoded.chunks_exact_mut(2).zip(digest) {
        let alphabet = b"0123456789abcdef";
        let upper = alphabet.get(usize::from(byte >> 4)).ok_or(
            ResultCalendarCorrespondenceError::Invalid("digest hexadecimal"),
        )?;
        let lower = alphabet.get(usize::from(byte & 15)).ok_or(
            ResultCalendarCorrespondenceError::Invalid("digest hexadecimal"),
        )?;
        pair.copy_from_slice(&[*upper, *lower]);
    }
    let text = std::str::from_utf8(&encoded)
        .map_err(|_| ResultCalendarCorrespondenceError::Invalid("digest hexadecimal"))?;
    Ok(ResultCalendarDigestV1::parse(text)?)
}

pub(super) fn require_canonical(
    value: &(impl Serialize + ?Sized),
    expected: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    let (sink, _) = serialized(
        value,
        CompareWriter {
            expected,
            offset: 0,
        },
        grant,
        None,
    )?;
    if sink.offset != expected.len() {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "noncanonical record",
        ));
    }
    Ok(())
}

fn serialized<W: Write>(
    value: &(impl Serialize + ?Sized),
    sink: W,
    grant: &mut AttachmentReplayGrantV1,
    prepaid: Option<usize>,
) -> Result<(W, usize)> {
    grant.charge_steps(1)?;
    let mut writer = GrantedWriter {
        sink,
        grant,
        prepaid,
        bytes: 0,
        writes: 0,
        failure: None,
    };
    let result = write_serialized(&mut writer, value);
    if let Some(error) = writer.failure.take() {
        return Err(error);
    }
    result?;
    if writer.prepaid.is_some_and(|remaining| remaining != 0) {
        return Err(ResultCalendarCorrespondenceError::Invalid(
            "canonical size changed",
        ));
    }
    Ok((writer.sink, writer.bytes))
}

struct GrantedWriter<'a, W> {
    sink: W,
    grant: &'a mut AttachmentReplayGrantV1,
    prepaid: Option<usize>,
    bytes: usize,
    writes: usize,
    failure: Option<ResultCalendarCorrespondenceError>,
}

impl<W> GrantedWriter<'_, W> {
    fn refuse(&mut self, error: ResultCalendarCorrespondenceError) -> io::Error {
        self.failure = Some(error);
        io::ErrorKind::InvalidData.into()
    }

    fn charge(&mut self, size: usize) -> io::Result<(usize, usize)> {
        let Some(bytes) = self
            .bytes
            .checked_add(size)
            .filter(|bytes| *bytes <= MAX_RESULT_CALENDAR_RECORD_BYTES)
        else {
            return Err(self.refuse(ResultCalendarCorrespondenceError::Capacity));
        };
        let Some(writes) = self.writes.checked_add(1) else {
            return Err(self.refuse(ResultCalendarCorrespondenceError::Capacity));
        };
        if self.writes.is_multiple_of(SERIALIZER_WRITES_PER_STEP) {
            if let Err(error) = self.grant.charge_steps(1) {
                return Err(self.refuse(error.into()));
            }
        }
        if let Some(remaining) = self.prepaid {
            let Some(remaining) = remaining.checked_sub(size) else {
                return Err(self.refuse(ResultCalendarCorrespondenceError::Invalid(
                    "canonical size changed",
                )));
            };
            self.prepaid = Some(remaining);
        } else if let Err(error) = self.grant.charge_bytes(size) {
            return Err(self.refuse(error.into()));
        }
        Ok((bytes, writes))
    }
}

impl<W: Write> Write for GrantedWriter<'_, W> {
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

struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        self.0.update(value);
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct CompareWriter<'a> {
    expected: &'a [u8],
    offset: usize,
}
impl Write for CompareWriter<'_> {
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
