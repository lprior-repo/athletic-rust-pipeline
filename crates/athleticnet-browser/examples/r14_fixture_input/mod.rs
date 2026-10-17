use flate2::bufread::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::fixture_schema::{validate_manifest, Result, NORMAL_BODY};
pub use crate::fixture_schema::{
    ArtifactKind, ArtifactManifest, FixtureInputError, FixtureManifest,
};
const MANIFEST_MAX: usize = 8192;
const READ_UNIT: usize = 65_536;
const HEX: &[u8; 16] = b"0123456789abcdef";

fn verify_chunk(kind: ArtifactKind, chunk: &[u8], offset: usize) -> Result<()> {
    let valid = match kind {
        ArtifactKind::Decoded => chunk.iter().all(|byte| *byte == 0),
        ArtifactKind::Wire => {
            offset
                .checked_add(chunk.len())
                .and_then(|end| NORMAL_BODY.get(offset..end))
                == Some(chunk)
        }
    };
    if !valid {
        return Err(FixtureInputError::Invalid("decoded content"));
    }
    Ok(())
}

pub fn read_manifest(path: &Path) -> Result<FixtureManifest> {
    let bytes = bounded_file(path, MANIFEST_MAX)?;
    let manifest: FixtureManifest = serde_json::from_slice(&bytes)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn bounded_file(path: &Path, expected: usize) -> io::Result<Vec<u8>> {
    let limit = expected
        .checked_add(1)
        .ok_or_else(|| io::Error::other("fixture byte overflow"))?;
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("fixture is not a regular file"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(limit)
        .map_err(|_| io::Error::other("fixture allocation"))?;
    file.take(u64::try_from(limit).map_err(|_| io::Error::other("fixture byte conversion"))?)
        .read_to_end(&mut bytes)?;
    if bytes.len() > expected {
        return Err(io::Error::other("fixture byte limit"));
    }
    Ok(bytes)
}

pub fn read_artifact(
    path: &Path,
    artifact: &ArtifactManifest,
    kind: ArtifactKind,
) -> Result<Vec<u8>> {
    kind.validate(artifact)?;
    let expected = usize::try_from(artifact.encoded)
        .map_err(|_| FixtureInputError::Invalid("encoded size conversion"))?;
    let bytes = bounded_file(path, expected)?;
    if bytes.len() != expected || digest_hex(Sha256::digest(&bytes).as_slice())? != artifact.sha256
    {
        return Err(FixtureInputError::Invalid("encoded inventory"));
    }
    verify_decoded(&bytes, artifact, kind)?;
    Ok(bytes)
}

fn verify_decoded(bytes: &[u8], artifact: &ArtifactManifest, kind: ArtifactKind) -> Result<()> {
    let mut decoder = GzDecoder::new(bytes);
    let mut hasher = Sha256::new();
    let mut total = 0usize;
    let expected = usize::try_from(artifact.decoded)
        .map_err(|_| FixtureInputError::Invalid("decoded size conversion"))?;
    let mut buffer = [0u8; READ_UNIT];
    for _ in 0..=expected {
        let remaining = expected
            .checked_add(1)
            .and_then(|limit| limit.checked_sub(total))
            .ok_or(FixtureInputError::Invalid("decoded byte overflow"))?;
        let target = buffer
            .get_mut(..remaining.min(READ_UNIT))
            .ok_or(FixtureInputError::Invalid("decoded read buffer"))?;
        let count = decoder.read(target)?;
        if count == 0 {
            return verify_decoded_end(&decoder, artifact, hasher, total);
        }
        let chunk = target
            .get(..count)
            .ok_or(FixtureInputError::Invalid("decoded read size"))?;
        verify_chunk(kind, chunk, total)?;
        total = total
            .checked_add(count)
            .ok_or(FixtureInputError::Invalid("decoded count overflow"))?;
        if total > expected {
            return Err(FixtureInputError::Invalid("decoded byte limit"));
        }
        hasher.update(chunk);
    }
    Err(FixtureInputError::Invalid("decoded read bound"))
}

fn verify_decoded_end(
    decoder: &GzDecoder<&[u8]>,
    artifact: &ArtifactManifest,
    hasher: Sha256,
    total: usize,
) -> Result<()> {
    if u64::try_from(total).ok() != Some(artifact.decoded)
        || !decoder.get_ref().is_empty()
        || digest_hex(hasher.finalize().as_slice())? != artifact.decoded_sha256
    {
        return Err(FixtureInputError::Invalid(
            "decoded inventory or trailing bytes",
        ));
    }
    Ok(())
}

fn digest_hex(bytes: &[u8]) -> io::Result<String> {
    let mut text = String::new();
    text.try_reserve_exact(64)
        .map_err(|_| io::Error::other("digest allocation"))?;
    for byte in bytes {
        let high = HEX
            .get(usize::from(byte >> 4))
            .ok_or_else(|| io::Error::other("digest nibble"))?;
        let low = HEX
            .get(usize::from(byte & 15))
            .ok_or_else(|| io::Error::other("digest nibble"))?;
        text.push(char::from(*high));
        text.push(char::from(*low));
    }
    Ok(text)
}

#[cfg(test)]
mod tests;
