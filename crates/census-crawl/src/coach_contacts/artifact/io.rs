use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const MAX_CSV_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_JSONL_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024;
pub const MAX_ROWS: usize = 200_000;
pub const CSV_FILE: &str = "contacts.csv";
pub const EVIDENCE_FILE: &str = "contacts.csv.evidence.jsonl";
pub const MANIFEST_FILE: &str = "manifest.json";

pub struct BoundedHashReader<R> {
    reader: R,
    hash: Sha256,
    remaining: u64,
}

impl<R: Read> BoundedHashReader<R> {
    pub fn new(reader: R, limit: u64) -> Self {
        Self {
            reader,
            hash: Sha256::new(),
            remaining: limit,
        }
    }

    pub fn digest(self) -> String {
        format!("{:x}", self.hash.finalize())
    }
}

impl<R: Read> Read for BoundedHashReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return match self.reader.read(&mut [0u8; 1])? {
                0 => Ok(0),
                _ => Err(invalid("file exceeds size limit")),
            };
        }
        let available = self
            .remaining
            .min(u64::try_from(buf.len()).map_err(io::Error::other)?);
        let available = usize::try_from(available).map_err(io::Error::other)?;
        let target = buf
            .get_mut(..available)
            .ok_or_else(|| invalid("reader returned an invalid length"))?;
        let count = self.reader.read(target)?;
        let bytes = buf
            .get(..count)
            .ok_or_else(|| invalid("reader returned an invalid length"))?;
        self.remaining = self
            .remaining
            .checked_sub(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| invalid("file exceeds size limit"))?;
        self.hash.update(bytes);
        Ok(count)
    }
}

pub struct BoundedHashWriter<W> {
    writer: W,
    hash: Sha256,
    remaining: u64,
}

impl<W: Write> BoundedHashWriter<W> {
    pub fn new(writer: W, limit: u64) -> Self {
        Self {
            writer,
            hash: Sha256::new(),
            remaining: limit,
        }
    }

    pub fn finish(self) -> (W, String) {
        (self.writer, format!("{:x}", self.hash.finalize()))
    }
}

impl<W: Write> Write for BoundedHashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let requested = u64::try_from(buf.len()).map_err(io::Error::other)?;
        if requested > self.remaining {
            return Err(invalid("file exceeds size limit"));
        }
        let count = self.writer.write(buf)?;
        let bytes = buf
            .get(..count)
            .ok_or_else(|| invalid("writer returned an invalid length"))?;
        self.remaining = self
            .remaining
            .checked_sub(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| invalid("file exceeds size limit"))?;
        self.hash.update(bytes);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub(super) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) fn record_buffer() -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_RECORD_BYTES)
        .map_err(io::Error::other)?;
    bytes.resize(MAX_RECORD_BYTES, 0);
    Ok(bytes)
}
