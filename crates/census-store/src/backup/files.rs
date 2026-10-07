use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::errors::io_err;
use crate::StoreResult;

pub(crate) const COPY_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Streamed {
    pub bytes: u64,
    pub sha256: String,
}

pub(crate) fn stream_copy(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<Streamed> {
    let mut hasher = Sha256::new();
    let mut bytes: u64 = 0;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let Some(filled) = buffer.get(..read) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "reader reported more bytes than the copy buffer holds",
            ));
        };
        hasher.update(filled);
        writer.write_all(filled)?;
        bytes = bytes.saturating_add(u64::try_from(read).map_or(u64::MAX, |value| value));
    }
    Ok(Streamed {
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

pub(super) fn copy_file(src: &Path, dst: &Path) -> StoreResult<Streamed> {
    let mut reader = File::open(src).map_err(|source| io_err(src, source))?;
    let mut writer = File::create(dst).map_err(|source| io_err(dst, source))?;
    let streamed = stream_copy(&mut reader, &mut writer).map_err(|source| io_err(src, source))?;
    writer.flush().map_err(|source| io_err(dst, source))?;
    writer.sync_all().map_err(|source| io_err(dst, source))?;
    Ok(streamed)
}

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn digest_file(path: &Path) -> StoreResult<Streamed> {
    let mut reader = File::open(path).map_err(|source| io_err(path, source))?;
    let mut sink = io::sink();
    stream_copy(&mut reader, &mut sink).map_err(|source| io_err(path, source))
}

pub(crate) fn sync_directories(root: &Path) -> StoreResult<u64> {
    let mut sync = crate::fs::fsync_dir;
    sync_directories_with(root, &mut sync)
}

pub(crate) fn sync_directories_with(
    root: &Path,
    sync: &mut impl FnMut(&Path) -> StoreResult<()>,
) -> StoreResult<u64> {
    let mut synced = 0_u64;
    sync_tree_directories(root, sync, &mut synced)?;
    Ok(synced)
}

fn sync_tree_directories(
    dir: &Path,
    sync: &mut impl FnMut(&Path) -> StoreResult<()>,
    synced: &mut u64,
) -> StoreResult<()> {
    let listing = fs::read_dir(dir).map_err(|source| io_err(dir, source))?;
    for entry in listing {
        let entry = entry.map_err(|source| io_err(dir, source))?;
        let path = entry.path();
        let kind = fs::symlink_metadata(&path)
            .map_err(|source| io_err(&path, source))?
            .file_type();
        if kind.is_dir() {
            sync_tree_directories(&path, sync, synced)?;
        }
    }
    sync(dir)?;
    *synced = synced.saturating_add(1);
    Ok(())
}

pub(super) fn ensure_parent_dir(dst_path: &Path) -> StoreResult<()> {
    let Some(parent) = dst_path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| io_err(parent, source))
}
