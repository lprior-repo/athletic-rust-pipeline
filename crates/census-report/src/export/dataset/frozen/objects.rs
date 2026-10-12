use super::codec::{
    count, is_digest, sha256_hex, CHUNK_BYTES_LIMIT, INVENTORY_BYTES_LIMIT, MANIFEST_BYTES_LIMIT,
};
use super::{codec_error, invalid};
use crate::report::{io_error, ReportError, ReportResult};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const CHUNK_CEILING: u64 = CHUNK_BYTES_LIMIT;
pub(super) const PAGE_CEILING: u64 = INVENTORY_BYTES_LIMIT;
pub(super) const MANIFEST_CEILING: u64 = MANIFEST_BYTES_LIMIT;

pub(super) fn directory(root: &Path) -> PathBuf {
    root.join("objects")
}

pub(super) fn install(directory: &Path, bytes: &[u8]) -> ReportResult<String> {
    let digest = sha256_hex(bytes);
    let length = count(bytes.len()).map_err(codec_error)?;
    let path = directory.join(&digest);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            reuse(&path, &digest, length)?;
            return Ok(digest);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error(&path, source)),
    }
    let staging = staging_path(directory, "object")?;
    let linked =
        write_staged(&staging, bytes).and_then(|()| link(&staging, &path, &digest, length));
    super::io::remove_owned(&staging, linked)?;
    Ok(digest)
}

fn link(staging: &Path, path: &Path, digest: &str, length: u64) -> ReportResult<()> {
    match std::fs::hard_link(staging, path) {
        Ok(()) => {
            #[cfg(test)]
            super::freeze::checkpoint("object-linked")?;
            sync_parent(path)?;
            #[cfg(test)]
            super::freeze::checkpoint("object-directory-synced")?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            reuse(path, digest, length)
        }
        Err(source) => Err(io_error(path, source)),
    }
}

fn reuse(path: &Path, digest: &str, length: u64) -> ReportResult<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() || metadata.len() != length {
        return Err(no_replace(path, digest));
    }
    if sha256_hex(&read_path(path, length)?) != digest {
        return Err(no_replace(path, digest));
    }
    Ok(())
}

pub(super) fn read(directory: &Path, digest: &str, ceiling: u64) -> ReportResult<Vec<u8>> {
    if !is_digest(digest) {
        return Err(invalid(&format!(
            "frozen object name {digest} is not a lowercase sha256 digest"
        )));
    }
    let path = directory.join(digest);
    let metadata = std::fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
    if !metadata.file_type().is_file() {
        return Err(artifact(&path, "a frozen object must be a regular file"));
    }
    if metadata.len() > ceiling {
        return Err(artifact(
            &path,
            &format!("the frozen object exceeds its {ceiling}-byte ceiling"),
        ));
    }
    let bytes = read_path(&path, ceiling)?;
    if sha256_hex(&bytes) != digest {
        return Err(artifact(
            &path,
            "the frozen object does not hash to its name",
        ));
    }
    Ok(bytes)
}

fn read_path(path: &Path, ceiling: u64) -> ReportResult<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|source| io_error(path, source))?;
    let limit = ceiling
        .checked_add(1)
        .ok_or_else(|| invalid("a frozen object ceiling overflowed u64"))?;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if count(bytes.len()).map_err(codec_error)? > ceiling {
        return Err(artifact(path, "the frozen object grew past its ceiling"));
    }
    Ok(bytes)
}

pub(super) fn sweep(directory: &Path) -> ReportResult<u64> {
    let mut removed: u64 = 0;
    for entry in std::fs::read_dir(directory).map_err(|source| io_error(directory, source))? {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
        if !metadata.file_type().is_file() {
            continue;
        }
        std::fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
        removed = removed
            .checked_add(1)
            .ok_or_else(|| invalid("the swept object count overflowed u64"))?;
    }
    sync(directory)?;
    Ok(removed)
}

pub(super) fn write_staged(path: &Path, bytes: &[u8]) -> ReportResult<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io_error(path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error(path, source))?;
    #[cfg(test)]
    super::freeze::checkpoint("object-written")?;
    file.flush().map_err(|source| io_error(path, source))?;
    #[cfg(test)]
    super::freeze::checkpoint("object-flushed")?;
    file.sync_all().map_err(|source| io_error(path, source))?;
    #[cfg(test)]
    super::freeze::checkpoint("object-synced")?;
    Ok(())
}

pub(super) fn staging_path(directory: &Path, tag: &str) -> ReportResult<PathBuf> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| invalid(&format!("reading the frozen staging clock: {error}")))?;
    Ok(directory.join(format!(
        ".staging.{tag}.{}.{}",
        std::process::id(),
        time.as_nanos()
    )))
}

pub(super) fn sync(directory: &Path) -> ReportResult<()> {
    std::fs::File::open(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(directory, source))
}

fn sync_parent(path: &Path) -> ReportResult<()> {
    let directory = path
        .parent()
        .ok_or_else(|| invalid("a frozen object path has no directory"))?;
    sync(directory)
}

fn no_replace(path: &Path, digest: &str) -> ReportError {
    ReportError::Invariant {
        detail: format!(
            "frozen object {} already exists with different bytes; refusing to replace {digest}",
            path.display()
        ),
    }
}

pub(super) fn artifact(path: &Path, detail: &str) -> ReportError {
    ReportError::ArtifactCorrupt {
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}
