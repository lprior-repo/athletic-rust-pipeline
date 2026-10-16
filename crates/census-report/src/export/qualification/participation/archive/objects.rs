use super::ParticipationReplayV1;
use crate::export::proof_objects;
use crate::report::{ReportError, ReportResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

const CHUNK_BYTES: usize = 4 * 1024 * 1024;
const MAX_BYTES: usize = 512 * 1024 * 1024;
const MAX_CHUNKS: usize = MAX_BYTES / CHUNK_BYTES;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    version: u32,
    bytes: usize,
    sha256: String,
    chunks: Vec<String>,
}

impl ParticipationReplayV1 {
    pub(crate) fn install(&self, directory: &Path) -> ReportResult<String> {
        let mut writer = Chunks::new(directory)?;
        let encoded = serde_json::to_writer(&mut writer, self);
        if let Some(error) = writer.failure.take() {
            return Err(error);
        }
        encoded.map_err(encoding)?;
        let descriptor = writer.finish()?;
        let bytes = serde_json::to_vec(&descriptor).map_err(encoding)?;
        proof_objects::install(directory, &bytes)
    }

    pub(crate) fn load(directory: &Path, anchored_descriptor: &str) -> ReportResult<Self> {
        let bytes = proof_objects::read(directory, anchored_descriptor, 32 * 1024)?;
        let descriptor: Descriptor = serde_json::from_slice(&bytes).map_err(encoding)?;
        if descriptor.version != 1
            || descriptor.bytes == 0
            || descriptor.bytes > MAX_BYTES
            || descriptor.chunks.len() > MAX_CHUNKS
            || descriptor.chunks.len() != descriptor.bytes.div_ceil(CHUNK_BYTES)
            || serde_json::to_vec(&descriptor).map_err(encoding)? != bytes
        {
            return Err(invalid(
                "participation replay descriptor shape or budget differs",
            ));
        }
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
                let expected = (descriptor.bytes - index * CHUNK_BYTES).min(CHUNK_BYTES);
                if chunk.len() != expected {
                    return Err(invalid("participation replay chunk boundary differs"));
                }
                payload.extend_from_slice(&chunk);
                Ok::<(), ReportError>(())
            })?;
        if hash(&payload) != descriptor.sha256 {
            return Err(invalid("participation replay payload digest differs"));
        }
        serde_json::from_slice(&payload).map_err(encoding)
    }
}

struct Chunks<'a> {
    directory: &'a Path,
    buffer: Vec<u8>,
    chunks: Vec<String>,
    hash: Sha256,
    bytes: usize,
    failure: Option<ReportError>,
}

impl<'a> Chunks<'a> {
    fn new(directory: &'a Path) -> ReportResult<Self> {
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(CHUNK_BYTES)
            .map_err(|_| invalid("participation proof chunk allocation refused"))?;
        Ok(Self {
            directory,
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
        if self.chunks.len() >= MAX_CHUNKS {
            return Err(invalid("participation replay chunk count exceeded"));
        }
        self.chunks
            .push(proof_objects::install(self.directory, &self.buffer)?);
        self.buffer.clear();
        Ok(())
    }

    fn accept(&mut self, bytes: &[u8]) -> ReportResult<()> {
        let needed = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("participation replay length overflow"))?;
        if needed > MAX_BYTES {
            return Err(invalid("participation replay exceeds 512 MiB"));
        }
        let (first, rest) = bytes.split_at(bytes.len().min(CHUNK_BYTES - self.buffer.len()));
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
        Ok(Descriptor {
            version: 1,
            bytes: self.bytes,
            sha256: format!("{:x}", self.hash.finalize()),
            chunks: self.chunks,
        })
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
