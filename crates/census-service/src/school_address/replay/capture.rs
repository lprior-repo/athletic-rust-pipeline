use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::ReplayError;

pub(super) const MAX_CAPTURE_BYTES: u64 = 128 * 1024 * 1024;

pub(super) fn verify_retained(archive: &Path) -> Result<(), ReplayError> {
    let manifest_path = archive.join("manifest.json");
    let bytes = load(&manifest_path, super::MAX_MANIFEST_BYTES)?;
    if archive.file_name().and_then(|name| name.to_str()) != Some(hex(&bytes).as_str()) {
        return Err(refused(&manifest_path, "retained manifest sha256 mismatch"));
    }
    let manifest: super::Manifest = serde_json::from_slice(&bytes)?;
    for capture in manifest.captures {
        let path = archive.join(&capture.sha256);
        if hash_file(&path)? != capture.sha256 {
            return Err(refused(&path, "retained capture sha256 mismatch"));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, ReplayError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.is_file() || metadata.len() > MAX_CAPTURE_BYTES {
        return Err(refused(path, "capture must be a bounded regular file"));
    }
    let mut file = std::fs::File::open(path)
        .map_err(|source| io(path, source))?
        .take(MAX_CAPTURE_BYTES.saturating_add(1));
    let mut hash = Sha256::new();
    let mut buffer = [0; 8192];
    let mut size = 0_u64;
    loop {
        let length = file.read(&mut buffer).map_err(|source| io(path, source))?;
        if length == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        let added = u64::try_from(length)
            .map_err(|_| refused(path, "capture size is not representable"))?;
        size = size
            .checked_add(added)
            .ok_or_else(|| refused(path, "capture size overflow"))?;
        if size > MAX_CAPTURE_BYTES {
            return Err(refused(path, "capture grew beyond its byte limit"));
        }
        hash.update(
            buffer
                .get(..length)
                .ok_or_else(|| refused(path, "capture hashing buffer is too small"))?,
        );
    }
}

pub(super) fn load(path: &Path, limit: u64) -> Result<Vec<u8>, ReplayError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(refused(path, "capture must be a bounded regular file"));
    }
    let file = std::fs::File::open(path).map_err(|source| io(path, source))?;
    let mut bytes = Vec::new();
    let size = usize::try_from(metadata.len())
        .map_err(|_| refused(path, "capture size is not representable"))?;
    bytes
        .try_reserve_exact(size)
        .map_err(|error| refused(path, &error.to_string()))?;
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io(path, source))?;
    let size = u64::try_from(bytes.len())
        .map_err(|_| refused(path, "capture size is not representable"))?;
    if size > limit {
        return Err(refused(path, "capture grew beyond its byte limit"));
    }
    Ok(bytes)
}

pub(super) fn preserve(path: &Path, bytes: &[u8]) -> Result<(), ReplayError> {
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(path, bytes)? {
                return Ok(());
            }
            return Err(refused(
                path,
                "retained capture differs; original bytes are not overwritten",
            ));
        }
        Err(source) => return Err(io(path, source)),
    };
    file.write_all(bytes).map_err(|source| io(path, source))?;
    file.sync_all().map_err(|source| io(path, source))
}

fn identical(path: &Path, bytes: &[u8]) -> Result<bool, ReplayError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    let expected_len = u64::try_from(bytes.len())
        .map_err(|_| refused(path, "capture size is not representable"))?;
    if !metadata.is_file() || metadata.len() != expected_len {
        return Ok(false);
    }
    let mut file = std::fs::File::open(path).map_err(|source| io(path, source))?;
    let mut buffer = [0; 8192];
    for expected in bytes.chunks(8192) {
        let actual = buffer
            .get_mut(..expected.len())
            .ok_or_else(|| refused(path, "capture comparison buffer is too small"))?;
        file.read_exact(actual).map_err(|source| io(path, source))?;
        if actual != expected {
            return Ok(false);
        }
    }
    let mut extra = [0; 1];
    Ok(file.read(&mut extra).map_err(|source| io(path, source))? == 0)
}

pub(super) fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn refused(path: &Path, detail: &str) -> ReplayError {
    ReplayError::Refused {
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}

pub(super) fn io(path: &Path, source: std::io::Error) -> ReplayError {
    ReplayError::Io {
        path: path.to_path_buf(),
        source,
    }
}
