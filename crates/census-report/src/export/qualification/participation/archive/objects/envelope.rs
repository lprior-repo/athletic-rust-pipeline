use super::{encoding, invalid, proof_objects, ReportResult, CHUNK_BYTES};
use serde::{Deserialize, Serialize};
use std::path::Path;

const LEGACY_DESCRIPTOR_BYTES: usize = 32 * 1024;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Descriptor {
    pub(super) version: u32,
    pub(super) bytes: usize,
    pub(super) sha256: String,
    pub(super) chunks: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Envelope {
    pub(super) version: u32,
    pub(super) max_bytes: usize,
    pub(super) max_chunks: usize,
    pub(super) descriptor_bytes: usize,
}

impl Envelope {
    pub(super) fn for_version(version: u32) -> ReportResult<Self> {
        let max_bytes = match version {
            1..=3 => 512 * 1024 * 1024,
            4 => 16 * 1024 * 1024 * 1024,
            _ => {
                return Err(invalid(
                    "unsupported participation replay descriptor version",
                ))
            }
        };
        let max_chunks = chunk_count(max_bytes)?;
        let descriptor_bytes = if version == 4 {
            descriptor_limit(max_chunks)?
        } else {
            LEGACY_DESCRIPTOR_BYTES
        };
        Ok(Self {
            version,
            max_bytes,
            max_chunks,
            descriptor_bytes,
        })
    }

    pub(super) fn admit_bytes(&self, current: usize, added: usize) -> ReportResult<usize> {
        let needed = current
            .checked_add(added)
            .ok_or_else(|| invalid("participation replay length overflow"))?;
        if needed > self.max_bytes {
            return Err(invalid(if self.version == 4 {
                "participation replay exceeds 16 GiB"
            } else {
                "participation replay exceeds 512 MiB"
            }));
        }
        Ok(needed)
    }

    pub(super) fn check(&self, descriptor: &Descriptor, max_bytes: usize) -> ReportResult<()> {
        if descriptor.version != self.version
            || descriptor.bytes == 0
            || descriptor.bytes > max_bytes
            || descriptor.chunks.len() > self.max_chunks
            || descriptor.chunks.len() != chunk_count(descriptor.bytes)?
        {
            return Err(invalid(
                "participation replay descriptor shape or budget differs",
            ));
        }
        Ok(())
    }

    pub(super) fn encode(&self, descriptor: &Descriptor) -> ReportResult<Vec<u8>> {
        self.check(descriptor, self.max_bytes)?;
        if !valid_digest(&descriptor.sha256)
            || descriptor.chunks.iter().any(|digest| !valid_digest(digest))
        {
            return Err(invalid(
                "participation replay descriptor shape or budget differs",
            ));
        }
        let bytes = serde_json::to_vec(descriptor).map_err(encoding)?;
        if bytes.len() > self.descriptor_bytes {
            return Err(invalid(
                "participation replay descriptor shape or budget differs",
            ));
        }
        Ok(bytes)
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn descriptor_limit(chunks: usize) -> ReportResult<usize> {
    chunks
        .checked_mul(67)
        .and_then(|bytes| bytes.checked_add(512))
        .ok_or_else(|| invalid("participation replay descriptor limit overflow"))
}

pub(super) fn chunk_count(bytes: usize) -> ReportResult<usize> {
    if bytes == 0 {
        return Err(invalid(
            "participation replay descriptor shape or budget differs",
        ));
    }
    let whole = bytes / CHUNK_BYTES;
    whole
        .checked_add(usize::from(!bytes.is_multiple_of(CHUNK_BYTES)))
        .ok_or_else(|| invalid("participation replay chunk limit overflow"))
}

pub(super) fn read(
    directory: &Path,
    anchored_descriptor: &str,
    version: u32,
    max_bytes: usize,
    expected_bytes: Option<u64>,
) -> ReportResult<Descriptor> {
    let envelope = Envelope::for_version(version)?;
    if max_bytes > envelope.max_bytes {
        return Err(invalid("participation replay exceeds absolute maximum"));
    }
    let ceiling = u64::try_from(envelope.descriptor_bytes)
        .map_err(|_| invalid("participation replay descriptor limit overflow"))?;
    let bytes = proof_objects::read(directory, anchored_descriptor, ceiling)?;
    let descriptor: Descriptor = serde_json::from_slice(&bytes).map_err(encoding)?;
    envelope.check(&descriptor, max_bytes)?;
    if serde_json::to_vec(&descriptor).map_err(encoding)? != bytes {
        return Err(invalid(
            "participation replay descriptor shape or budget differs",
        ));
    }
    if let Some(expected) = expected_bytes {
        let actual = u64::try_from(descriptor.bytes)
            .map_err(|_| invalid("participation replay payload length overflow"))?;
        if actual != expected {
            return Err(invalid("participation replay payload length differs"));
        }
    }
    Ok(descriptor)
}

pub(super) fn version(directory: &Path, anchored_descriptor: &str) -> ReportResult<u32> {
    let ceiling = u64::try_from(Envelope::for_version(4)?.descriptor_bytes)
        .map_err(|_| invalid("participation replay descriptor limit overflow"))?;
    let bytes = proof_objects::read(directory, anchored_descriptor, ceiling)?;
    if let Ok(descriptor) = serde_json::from_slice::<Descriptor>(&bytes) {
        if descriptor.version == 4 {
            return Ok(4);
        }
    }
    let bytes = proof_objects::read(directory, anchored_descriptor, 32 * 1024)?;
    let descriptor: Descriptor = serde_json::from_slice(&bytes).map_err(encoding)?;
    Ok(descriptor.version)
}

#[cfg(test)]
mod tests;
