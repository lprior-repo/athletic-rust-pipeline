use std::io::{Read, Write};
use std::path::Path;

use std::sync::atomic::{AtomicU64, Ordering};
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
    let (tmp, mut file) = create_temp(path)?;
    let staged = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| io(&tmp, source));
    drop(file);
    staged?;

    let tmp_bytes = load(&tmp, u64::MAX)?;
    if tmp_bytes != bytes {
        let _ = std::fs::remove_file(&tmp);
        return Err(refused(
            &tmp,
            "atomic write verification failed: bytes differ after write",
        ));
    }

    match std::fs::hard_link(&tmp, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&tmp);
            fsync_parent(path)?;
            Ok(())
        }
        Err(source) => {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                if identical(path, bytes)? {
                    let _ = std::fs::remove_file(&tmp);
                    return Ok(());
                }
                let recovered = quarantine_and_retry(path, &tmp, bytes);
                if recovered.is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
                recovered
            } else {
                let _ = std::fs::remove_file(&tmp);
                Err(io(path, source))
            }
        }
    }
}

pub(super) fn publish(path: &Path, bytes: &[u8]) -> Result<(), ReplayError> {
    let (tmp, mut file) = create_temp(path)?;
    let staged = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| io(&tmp, source));
    drop(file);
    staged?;

    let tmp_bytes = load(&tmp, u64::MAX)?;
    if tmp_bytes != bytes {
        let _ = std::fs::remove_file(&tmp);
        return Err(refused(
            &tmp,
            "atomic write verification failed: bytes differ after write",
        ));
    }

    if let Err(source) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(path, source));
    }
    fsync_parent(path)
}

fn create_temp(path: &Path) -> Result<(std::path::PathBuf, std::fs::File), ReplayError> {
    loop {
        let tmp = atomic_temp_path(path);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((tmp, file)),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(io(&tmp, source)),
        }
    }
}

fn quarantine_and_retry(
    path: &Path,
    tmp: &Path,
    bytes: &[u8],
) -> Result<(), ReplayError> {
    let observed = load(path, u64::MAX);
    let observed_len = observed.as_ref().map_or(0_u64, |bytes| bytes.len() as u64);
    let observed_digest = observed
        .as_ref()
        .map_or_else(|_| "unreadable".to_string(), |bytes| hex(bytes));
    let (quarantine, label) =
        quarantine_link(path, observed.as_ref().ok().map(|bytes| bytes.as_slice()))?;
    let sidecar_path = path.with_extension(format!("{label}.sidecar"));
    let sidecar_content = format!(
        "quarantine: {}\nexpected_len: {}\nexpected_digest: {}\nobserved_len: {}\nobserved_digest: {}\nreason: bytes differ after install\n",
        quarantine.display(),
        bytes.len(),
        hex(bytes),
        observed_len,
        observed_digest
    );
    write_sidecar(&sidecar_path, sidecar_content.as_bytes())?;
    std::fs::remove_file(path).map_err(|source| io(path, source))?;
    fsync_parent(path)?;

    match std::fs::hard_link(tmp, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(tmp);
            fsync_parent(path)?;
            Ok(())
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(path, bytes)? {
                let _ = std::fs::remove_file(tmp);
                Ok(())
            } else {
                Err(io(path, source))
            }
        }
        Err(source) => Err(io(path, source)),
    }
}

fn quarantine_link(
    path: &Path,
    observed: Option<&[u8]>,
) -> Result<(std::path::PathBuf, String), ReplayError> {
    let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let mut attempt = 0_u32;
    loop {
        let label = match (observed, attempt) {
            (Some(bytes), 0) => format!("recovered-{}", hex(bytes)),
            (Some(bytes), attempt) => format!("recovered-{}-{attempt}", hex(bytes)),
            (None, 0) => format!("recovered-{stamp}"),
            (None, attempt) => format!("recovered-{stamp}-{attempt}"),
        };
        let candidate = path.with_extension(&label);
        match std::fs::hard_link(path, &candidate) {
            Ok(()) => return Ok((candidate, label)),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                if observed.is_some_and(|bytes| identical(&candidate, bytes).unwrap_or(false)) {
                    return Ok((candidate, label));
                }
                attempt = attempt.saturating_add(1);
                if attempt > QUARANTINE_ATTEMPTS {
                    return Err(refused(path, "quarantine destination is occupied"));
                }
            }
            Err(source) => return Err(io(&candidate, source)),
        }
    }
}

fn write_sidecar(path: &Path, content: &[u8]) -> Result<(), ReplayError> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(content).map_err(|source| io(path, source))?;
            file.sync_all().map_err(|source| io(path, source))
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            if identical(path, content)? {
                Ok(())
            } else {
                Err(refused(path, "quarantine evidence is occupied"))
            }
        }
        Err(source) => Err(io(path, source)),
    }
}

fn fsync_parent(path: &Path) -> Result<(), ReplayError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    let dir = std::fs::File::open(parent).map_err(|source| io(parent, source))?;
    dir.sync_all().map_err(|source| io(parent, source))
}

const QUARANTINE_ATTEMPTS: u32 = 32;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn atomic_temp_path(path: &Path) -> std::path::PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    path.with_extension(format!("tmp.{}.{}", pid, counter))
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

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
