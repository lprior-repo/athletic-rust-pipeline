//! The store's directory footprint: the recursive byte total `stats` reports, and the number an
//! operator sizes a copy or a backup by.
//!
//! It sits beside the reads rather than inside them because a directory's byte total is a different
//! question from anything the store's own ledger answers: fjall's `disk_space()` counts LSM-tree level
//! sizes, so the two disagree on any store whose newest batch still lives in the write-ahead journal.

use std::path::Path;

use super::super::{StoreError, StoreResult};

/// Recursive byte total of a directory.
///
/// This is the number to size a copy or a backup by. fjall's own `disk_space()` counts LSM-tree
/// level sizes, so a store whose newest batch still lives in the write-ahead journal reports a
/// figure far below what a copy has to carry — a drill measured `bytes_on_disk 0` on a store whose
/// directory held 229 KB, 223 KB of it journal.
///
/// A subtree that cannot be read is an error, never a zero: a store holding 500 GB behind one
/// unreadable directory would otherwise report as an empty one, and the copy sized from that number
/// would be sized from a lie. Every read failure here — the directory, an entry, an entry's
/// metadata — is the caller's to handle.
pub(crate) fn directory_bytes(root: &Path) -> StoreResult<u64> {
    let entries = std::fs::read_dir(root).map_err(|source| StoreError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut total = 0_u64;
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })?;
        let bytes = if metadata.is_dir() {
            directory_bytes(&path)?
        } else {
            metadata.len()
        };
        total = total.saturating_add(bytes);
    }
    Ok(total)
}
