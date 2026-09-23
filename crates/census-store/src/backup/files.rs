//! One file at a time: a single pass that hashes the bytes it copies, and the fsyncs that make the
//! result durable.
//!
//! Everything here is deliberately allocation-light and path-explicit. A copy step holds one
//! [`COPY_BUFFER_BYTES`] buffer whatever the file's size, and every failure names the path it failed on.
//!
//! The digest and the copy are one pass because they are one fact: a manifest that described a second
//! read of the source would be describing bytes the destination may not hold.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::errors::io_err;
use crate::StoreResult;

/// One copy step's buffer: the only allocation one file costs, however large the file is.
///
/// Visible in `store` rather than `backup` because the backup tests measure a copy in chunks too.
pub(crate) const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// What one pass over a reader saw.
///
/// Visible in `store` rather than `backup` because the backup tests read it back from
/// [`stream_copy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Streamed {
    /// Bytes that passed through.
    pub bytes: u64,
    /// SHA-256 of those bytes, hex.
    pub sha256: String,
}

/// One pass over `reader`: hash every byte, count it, and write it to `writer`.
///
/// The buffer is fixed, so a caller copies a file of any size in constant memory, and the digest is
/// taken from the same bytes the destination received rather than from a second read of the source.
/// Callers that only want the digest pass [`io::sink`].
///
/// Visible in `store` rather than `backup` because the backup tests drive it directly, to pin what a
/// copy does with a reader that fails mid-stream and a writer that refuses a chunk.
pub(crate) fn stream_copy(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<Streamed> {
    let mut hasher = Sha256::new();
    let mut bytes: u64 = 0;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        // A `Read` that reports more bytes than it was given room for has broken its contract; that is
        // an error, never a slice that could panic on it.
        let Some(filled) = buffer.get(..read) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "reader reported more bytes than the copy buffer holds",
            ));
        };
        hasher.update(filled);
        writer.write_all(filled)?;
        bytes = bytes.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
    }
    Ok(Streamed {
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

/// Copy `src` to a freshly created `dst` in one pass, returning the bytes written and their digest.
///
/// The destination is fsynced before this returns: a generation is only published after its files are
/// durable, and a rename makes an entry durable without making the file behind it durable.
pub(super) fn copy_file(src: &Path, dst: &Path) -> StoreResult<Streamed> {
    let mut reader = File::open(src).map_err(|source| io_err(src, source))?;
    let mut writer = File::create(dst).map_err(|source| io_err(dst, source))?;
    let streamed = stream_copy(&mut reader, &mut writer).map_err(|source| io_err(src, source))?;
    writer.flush().map_err(|source| io_err(dst, source))?;
    writer.sync_all().map_err(|source| io_err(dst, source))?;
    Ok(streamed)
}

/// The length and digest of `path` in one pass, without holding the file in memory.
pub(super) fn digest_file(path: &Path) -> StoreResult<Streamed> {
    let mut reader = File::open(path).map_err(|source| io_err(path, source))?;
    let mut sink = io::sink();
    stream_copy(&mut reader, &mut sink).map_err(|source| io_err(path, source))
}

/// fsync a directory, so a file created or renamed in it survives a machine crash.
#[cfg(unix)]
pub(super) fn fsync_dir(dir: &Path) -> StoreResult<()> {
    let handle = File::open(dir).map_err(|source| io_err(dir, source))?;
    handle.sync_all().map_err(|source| io_err(dir, source))
}

#[cfg(not(unix))]
pub(super) fn fsync_dir(_dir: &Path) -> StoreResult<()> {
    Ok(())
}

/// Create the directories a materialised path needs.
pub(super) fn ensure_parent_dir(dst_path: &Path) -> StoreResult<()> {
    let Some(parent) = dst_path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| io_err(parent, source))
}
