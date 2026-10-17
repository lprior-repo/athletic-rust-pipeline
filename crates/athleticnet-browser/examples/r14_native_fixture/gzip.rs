use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::{Compression, GzBuilder};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub(crate) const NORMAL_BODY: &str = "wire-smoke-ok";
pub(crate) const WIRE_ENCODED: u64 = 33_554_471;
pub(crate) const WIRE_DECODED: u64 = 13;
pub(crate) const DECODED_ENCODED_MAX: u64 = 33_554_432;
pub(crate) const DECODED_DECODED: u64 = 33_554_433;
pub(crate) const SCOPE: &str =
    "Synthetic transport fixture only; no population, capture authority or runtime acceptance";

const ZERO_CHUNK: usize = 65_536;
const SHA256_BYTES: usize = 32;
const SHA256_HEX_CHARS: usize = 64;
const HEX: &[u8; 16] = b"0123456789abcdef";

pub(crate) struct Summary {
    pub(crate) encoded: u64,
    pub(crate) decoded: u64,
    pub(crate) sha256: String,
    pub(crate) decoded_sha256: String,
}

pub(crate) fn other(message: &'static str) -> io::Error {
    io::Error::other(message)
}

pub(crate) fn hex_digest(hasher: Sha256) -> io::Result<String> {
    let digest = hasher.finalize();
    let bytes = digest.as_slice();
    if bytes.len() != SHA256_BYTES {
        return Err(other("sha256 digest length"));
    }
    let mut text = String::new();
    text.try_reserve(SHA256_HEX_CHARS)
        .map_err(|_| other("sha256 allocation"))?;
    for byte in bytes.iter().copied() {
        let high_nibble = byte
            .checked_shr(4)
            .ok_or_else(|| other("sha256 high nibble shift"))?;
        let high = HEX
            .get(usize::from(high_nibble))
            .copied()
            .ok_or_else(|| other("sha256 high nibble"))?;
        let low = HEX
            .get(usize::from(byte & 0x0fu8))
            .copied()
            .ok_or_else(|| other("sha256 low nibble"))?;
        text.push(char::from(high));
        text.push(char::from(low));
    }
    Ok(text)
}

pub(crate) struct MeasureWriter<W> {
    inner: W,
    hasher: Sha256,
    encoded: u64,
    limit: u64,
}

impl<W> MeasureWriter<W> {
    pub(crate) fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            encoded: 0,
            limit,
        }
    }

    pub(crate) fn finish(self) -> io::Result<(W, u64, String)> {
        let Self {
            inner,
            hasher,
            encoded,
            limit,
        } = self;
        if encoded > limit {
            return Err(other("encoded count exceeds limit"));
        }
        Ok((inner, encoded, hex_digest(hasher)?))
    }
}

impl<W: Write> Write for MeasureWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let requested = u64::try_from(buffer.len()).map_err(|_| other("encoded write length"))?;
        let prospective = self
            .encoded
            .checked_add(requested)
            .ok_or_else(|| other("encoded count overflow"))?;
        if prospective > self.limit {
            return Err(other("encoded write exceeds limit"));
        }
        let written = self.inner.write(buffer)?;
        let piece = buffer
            .get(..written)
            .ok_or_else(|| other("encoded short write"))?;
        self.hasher.update(piece);
        let added = u64::try_from(written).map_err(|_| other("encoded written length"))?;
        self.encoded = self
            .encoded
            .checked_add(added)
            .ok_or_else(|| other("encoded count overflow"))?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) fn create_private(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o777 != 0o600 {
        return Err(other("fixture file mode or type"));
    }
    Ok(file)
}

fn write_zeros<W: Write>(writer: &mut W, total: u64, limit: u64) -> io::Result<()> {
    if total > limit {
        return Err(other("decoded count exceeds limit"));
    }
    let zeros = [0u8; ZERO_CHUNK];
    let chunk = u64::try_from(ZERO_CHUNK).map_err(|_| other("zero chunk length"))?;
    let mut remaining = total;
    let mut written = 0u64;
    while remaining >= chunk {
        writer.write_all(&zeros)?;
        remaining = remaining
            .checked_sub(chunk)
            .ok_or_else(|| other("zero count underflow"))?;
        written = written
            .checked_add(chunk)
            .ok_or_else(|| other("zero count overflow"))?;
    }
    let tail = usize::try_from(remaining).map_err(|_| other("zero tail length"))?;
    let slice = zeros.get(..tail).ok_or_else(|| other("zero tail bounds"))?;
    writer.write_all(slice)?;
    written = written
        .checked_add(remaining)
        .ok_or_else(|| other("zero count overflow"))?;
    if written != total {
        return Err(other("zero count mismatch"));
    }
    Ok(())
}

pub(crate) fn write_decoded(path: &Path) -> io::Result<Summary> {
    let file = create_private(path)?;
    let measured = MeasureWriter::new(file, DECODED_ENCODED_MAX);
    let mut encoder: GzEncoder<MeasureWriter<File>> = GzBuilder::new()
        .mtime(0)
        .write(measured, Compression::best());
    write_zeros(&mut encoder, DECODED_DECODED, DECODED_DECODED)?;
    let measured = encoder.finish()?;
    let (mut file, encoded, sha256) = measured.finish()?;
    if encoded > DECODED_ENCODED_MAX {
        return Err(other("decoded encoded limit"));
    }
    file.flush()?;
    file.sync_all()?;
    let decoded_sha256 = verify_decoded(path)?;
    Ok(Summary {
        encoded,
        decoded: DECODED_DECODED,
        sha256,
        decoded_sha256,
    })
}

fn verify_decoded(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut buffer = [0u8; ZERO_CHUNK];
    let mut total = 0u64;
    let mut hasher = Sha256::new();
    for _ in 0..=DECODED_DECODED {
        let read = decoder.read(&mut buffer)?;
        if read == 0 {
            if total != DECODED_DECODED {
                return Err(other("decoded count mismatch"));
            }
            return hex_digest(hasher);
        }
        let slice = buffer
            .get(..read)
            .ok_or_else(|| other("decoded read bounds"))?;
        if slice.iter().any(|byte| *byte != 0) {
            return Err(other("decoded value is not zero"));
        }
        let added = u64::try_from(read).map_err(|_| other("decoded read length"))?;
        total = total
            .checked_add(added)
            .ok_or_else(|| other("decoded count overflow"))?;
        if total > DECODED_DECODED {
            return Err(other("decoded count exceeds limit"));
        }
        hasher.update(slice);
    }
    Err(other("decoded read bound"))
}
