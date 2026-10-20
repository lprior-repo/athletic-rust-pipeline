use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path};

const MAX_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;

pub(super) struct VerifiedBytes {
    pub(super) bytes: Vec<u8>,
    pub(super) sha256: String,
    pub(super) byte_length: u64,
}

pub(super) fn read_verified(
    root: &Path,
    artifact: &super::NcesSelectedArtifactV1,
) -> Result<VerifiedBytes> {
    if artifact.path.is_absolute()
        || artifact
            .path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("NCES input path must be a contained relative path");
    }
    let root = std::fs::canonicalize(root).context("canonicalizing NCES input root")?;
    let path = root.join(&artifact.path);
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("reading NCES input metadata at {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.len() > MAX_ARTIFACT_BYTES
    {
        bail!("NCES input must be a nonempty bounded regular file");
    }
    let canonical = std::fs::canonicalize(&path)?;
    if !canonical.starts_with(&root) {
        bail!("NCES input escapes its declared source root");
    }
    let file = std::fs::File::open(&canonical)?;
    let mut bytes = Vec::new();
    let capacity =
        usize::try_from(metadata.len()).context("NCES input size is not representable")?;
    bytes
        .try_reserve_exact(capacity)
        .context("reserving bounded NCES input")?;
    file.take(MAX_ARTIFACT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let byte_length = u64::try_from(bytes.len())?;
    if byte_length == 0 || byte_length > MAX_ARTIFACT_BYTES {
        bail!("NCES input grew beyond its byte limit");
    }
    let sha256 = sha256_hex(&bytes);
    if sha256 != artifact.sha256 {
        bail!("NCES input SHA-256 does not match its selected digest");
    }
    Ok(VerifiedBytes {
        bytes,
        sha256,
        byte_length,
    })
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
