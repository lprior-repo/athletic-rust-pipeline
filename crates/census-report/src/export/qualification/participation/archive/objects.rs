use super::ParticipationReplayV1;
use crate::export::proof_objects;
use crate::report::{ReportError, ReportResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

mod envelope;
use envelope::{Descriptor, Envelope};

const CHUNK_BYTES: usize = 4 * 1024 * 1024;

impl ParticipationReplayV1 {
    pub(crate) fn install(&self, directory: &Path) -> ReportResult<String> {
        install(self, directory, 1)
    }
    pub(crate) fn load(directory: &Path, anchored_descriptor: &str) -> ReportResult<Self> {
        load(directory, anchored_descriptor, 1)
    }
}

pub(super) fn install<T: Serialize>(
    value: &T,
    directory: &Path,
    version: u32,
) -> ReportResult<String> {
    let envelope = Envelope::for_version(version)?;
    let mut writer = Chunks::new(directory, envelope)?;
    let encoded = serde_json::to_writer(&mut writer, value);
    if let Some(error) = writer.failure.take() {
        return Err(error);
    }
    encoded.map_err(encoding)?;
    let descriptor = writer.finish()?;
    let bytes = envelope.encode(&descriptor)?;
    proof_objects::install(directory, &bytes)
}

pub(super) fn load<T: serde::de::DeserializeOwned + Serialize>(
    directory: &Path,
    anchored_descriptor: &str,
    version: u32,
) -> ReportResult<T> {
    let payload = load_bytes(directory, anchored_descriptor, version)?;
    let value: T = serde_json::from_slice(&payload).map_err(encoding)?;
    if matches!(version, 2..=4) {
        super::canonical::require(&value, &payload)?;
    }
    Ok(value)
}

pub(super) fn load_bytes(
    directory: &Path,
    anchored_descriptor: &str,
    version: u32,
) -> ReportResult<Vec<u8>> {
    let envelope = Envelope::for_version(version)?;
    load_bytes_with_limits(
        directory,
        anchored_descriptor,
        version,
        None,
        envelope.max_bytes,
    )
}

pub(super) fn load_bounded_bytes(
    directory: &Path,
    anchored_descriptor: &str,
    version: u32,
    expected_bytes: u64,
    max_bytes: usize,
) -> ReportResult<Vec<u8>> {
    load_bytes_with_limits(
        directory,
        anchored_descriptor,
        version,
        Some(expected_bytes),
        max_bytes,
    )
}

fn chunk_expected(total: usize, index: usize) -> ReportResult<usize> {
    let consumed = index
        .checked_mul(CHUNK_BYTES)
        .ok_or_else(|| invalid("participation replay chunk limit overflow"))?;
    total
        .checked_sub(consumed)
        .ok_or_else(|| invalid("participation replay chunk boundary differs"))
}

fn load_bytes_with_limits(
    directory: &Path,
    anchored_descriptor: &str,
    version: u32,
    expected_bytes: Option<u64>,
    max_bytes: usize,
) -> ReportResult<Vec<u8>> {
    let descriptor = envelope::read(
        directory,
        anchored_descriptor,
        version,
        max_bytes,
        expected_bytes,
    )?;
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(descriptor.bytes)
        .map_err(|_| invalid("participation replay allocation refused"))?;
    descriptor
        .chunks
        .iter()
        .enumerate()
        .try_for_each(|(index, digest)| {
            let chunk_limit = u64::try_from(CHUNK_BYTES)
                .map_err(|_| invalid("participation replay chunk limit overflow"))?;
            let chunk = proof_objects::read(directory, digest, chunk_limit)?;
            let expected = chunk_expected(descriptor.bytes, index)?.min(CHUNK_BYTES);
            if chunk.len() != expected {
                return Err(invalid("participation replay chunk boundary differs"));
            }
            payload.extend_from_slice(&chunk);
            Ok::<(), ReportError>(())
        })?;
    if hash(&payload) != descriptor.sha256 {
        return Err(invalid("participation replay payload digest differs"));
    }
    Ok(payload)
}

pub(super) fn version(directory: &Path, anchored_descriptor: &str) -> ReportResult<u32> {
    envelope::version(directory, anchored_descriptor)
}

struct Chunks<'a> {
    directory: &'a Path,
    envelope: Envelope,
    buffer: Vec<u8>,
    chunks: Vec<String>,
    hash: Sha256,
    bytes: usize,
    failure: Option<ReportError>,
}

impl<'a> Chunks<'a> {
    fn new(directory: &'a Path, envelope: Envelope) -> ReportResult<Self> {
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(CHUNK_BYTES)
            .map_err(|_| invalid("participation proof chunk allocation refused"))?;
        Ok(Self {
            directory,
            envelope,
            buffer,
            chunks: Vec::new(),
            hash: Sha256::new(),
            bytes: 0,
            failure: None,
        })
    }

    fn store(&mut self) -> ReportResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        if self.chunks.len() >= self.envelope.max_chunks {
            return Err(invalid("participation replay chunk count exceeded"));
        }
        self.chunks
            .try_reserve_exact(1)
            .map_err(|_| invalid("participation replay chunk inventory allocation refused"))?;
        self.chunks
            .push(proof_objects::install(self.directory, &self.buffer)?);
        self.buffer.clear();
        Ok(())
    }

    fn accept(&mut self, bytes: &[u8]) -> ReportResult<()> {
        let needed = self.envelope.admit_bytes(self.bytes, bytes.len())?;
        let room = CHUNK_BYTES
            .checked_sub(self.buffer.len())
            .ok_or_else(|| invalid("participation replay length overflow"))?;
        let (first, rest) = bytes.split_at(bytes.len().min(room));
        self.buffer.extend_from_slice(first);
        if self.buffer.len() == CHUNK_BYTES {
            self.store()?;
        }
        rest.chunks(CHUNK_BYTES).try_for_each(|chunk| {
            self.buffer.extend_from_slice(chunk);
            if self.buffer.len() == CHUNK_BYTES {
                self.store()?;
            }
            Ok::<(), ReportError>(())
        })?;
        self.hash.update(bytes);
        self.bytes = needed;
        Ok(())
    }

    fn finish(mut self) -> ReportResult<Descriptor> {
        self.store()?;
        let descriptor = Descriptor {
            version: self.envelope.version,
            bytes: self.bytes,
            sha256: format!("{:x}", self.hash.finalize()),
            chunks: self.chunks,
        };
        self.envelope.check(&descriptor, self.envelope.max_bytes)?;
        Ok(descriptor)
    }
}

impl Write for Chunks<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.accept(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(std::io::Error::other("participation proof stream refused"))
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encoding(source: serde_json::Error) -> ReportError {
    census_store::StoreError::Json {
        detail: "participation replay object".into(),
        source,
    }
    .into()
}
fn invalid(detail: &str) -> ReportError {
    super::super::invalid(detail).into()
}
pub(in crate::export::qualification::participation) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
