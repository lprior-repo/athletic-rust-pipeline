use super::{paths, shell};
use crate::net::cache::CacheMeta;
use crate::net::FetchError;
use census_store::{RecoveryArchiveIo, RecoveryFsOperation as Op};
use sha2::Digest;
use std::fs::{File, OpenOptions};
use std::path::Path;

#[cfg(test)]
pub(in crate::net) mod testing;

pub(super) enum Failure {
    Before(FetchError),
    During(FetchError),
}

pub(super) fn cache_key(
    metadata: &CacheMeta,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<String, FetchError> {
    io.reserve_owned(32)?;
    let chunks = metadata
        .representation
        .entries()
        .iter()
        .flat_map(|(name, value)| {
            [
                name.as_bytes(),
                b"=".as_slice(),
                value.as_bytes(),
                b"\x1f".as_slice(),
            ]
        });
    let hasher = super::super::cache_hash("GET", &metadata.url, chunks);
    let digest = hasher.finalize();
    let mut encoded = String::new();
    encoded
        .try_reserve_exact(32)
        .map_err(|_| shell::limit("allocation"))?;
    use std::fmt::Write;
    for byte in digest.iter().take(16) {
        write!(&mut encoded, "{byte:02x}").map_err(|_| shell::limit("cache key encoding"))?;
    }
    Ok(encoded)
}

pub(super) fn lock(metadata: &Path, io: &mut RecoveryArchiveIo<'_>) -> Result<File, FetchError> {
    let root = metadata
        .parent()
        .ok_or_else(|| shell::limit("cache ancestry"))?;
    let name = metadata
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| shell::limit("cache name"))?;
    let path = paths::suffixed(root, name, ".json.lock", io)?;
    shell::regular(&path, io)?;
    io.direct_operation(Op::Create)?;
    io.reserve_owned(
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| shell::limit("diagnostics"))?,
    )?;
    let file = match OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(source) => return Err(shell::retained_error(&path, source, io)?),
    };
    io.direct_operation(Op::Open)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            let detail = "cache publication is already in progress";
            let bytes = path
                .as_os_str()
                .len()
                .checked_add(detail.len())
                .and_then(|n| n.checked_add(512))
                .ok_or_else(|| shell::limit("diagnostic bytes"))?;
            io.diagnostic_bytes(bytes)?;
            return Err(FetchError::Cache {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::WouldBlock, detail),
            });
        }
        Err(std::fs::TryLockError::Error(source)) => {
            return Err(shell::retained_error(&path, source, io)?)
        }
    }
    io.release_owned(
        path.as_os_str()
            .len()
            .checked_add(512)
            .ok_or_else(|| shell::limit("diagnostics"))?,
    )?;
    io.release_owned(path.as_os_str().len())?;
    #[cfg(test)]
    super::super::archive::lock_tests::checkpoint(
        &root.join("archive-capacity.lock"),
        super::super::archive::lock_tests::LockEvent::EligibleAcquired,
    )?;
    Ok(file)
}

pub(super) fn eligible(
    stage: &Path,
    archived_body: &Path,
    body_path: &Path,
    metadata_path: &Path,
    encoded: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), Failure> {
    let body = paths::child(stage, "cache.body", io).map_err(Failure::Before)?;
    let metadata = paths::child(stage, "cache.meta.json", io).map_err(Failure::Before)?;
    shell::link(archived_body, &body, io).map_err(Failure::Before)?;
    let mut file = shell::create(&metadata, io).map_err(Failure::Before)?;
    shell::write(&mut file, &metadata, encoded, io).map_err(Failure::Before)?;
    shell::sync(&file, &metadata, io).map_err(Failure::Before)?;
    let root = body_path
        .parent()
        .ok_or_else(|| Failure::Before(shell::limit("cache ancestry")))?;
    shell::rename(&body, body_path, io).map_err(Failure::During)?;
    shell::sync_directory(root, io).map_err(Failure::During)?;
    #[cfg(test)]
    {
        io.reserve_owned(
            metadata_path
                .as_os_str()
                .len()
                .checked_add(512)
                .ok_or_else(|| Failure::During(shell::limit("allocation")))?,
        )
        .map_err(|error| Failure::During(error.into()))?;
        if let Err(source) = testing::reached() {
            let error =
                shell::retained_error(metadata_path, source, io).map_err(Failure::During)?;
            return Err(Failure::During(error));
        }
    }
    shell::rename(&metadata, metadata_path, io).map_err(Failure::During)?;
    shell::sync_directory(root, io).map_err(Failure::During)
}
