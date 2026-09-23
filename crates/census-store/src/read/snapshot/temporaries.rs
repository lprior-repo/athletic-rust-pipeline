//! The `.part` temporaries a snapshot is staged in: the unique name a writer is handed, and the
//! sweep that reclaims what a killed writer left behind.
//!
//! A temporary is only visible to a reader between its creation and its rename, and the writer holds
//! the store's exclusive lock for its whole life. That is what makes the name per call rather than
//! per process ([`temporary_path`]) and what makes every one of them found at store open litter from
//! a process that died mid-write ([`sweep_stale_temporaries`]).

use std::path::{Path, PathBuf};

use crate::StoreResult;

use super::io_failure;

/// The temporary a snapshot is written to before it is published.
///
/// Unique per call, not per process: several consolidations run inside one `census-serve`, so a
/// name built from the process id alone would have two of them writing the same temporary.
pub(super) fn temporary_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "snapshot".to_string());
    path.with_file_name(format!(".{name}.{}.{sequence}.part", std::process::id()))
}

/// Remove the temporaries a dead writer left behind, and report how many were reclaimed.
///
/// A temporary is only visible to a reader between its creation and its rename, and the writer holds
/// the store's exclusive lock for its whole life — so any temporary present when the store opens
/// belongs to a process that died mid-write, and every one of them is a partial copy of a table that
/// is still in the store. Without this sweep a crash during a national consolidation leaks a
/// gigabyte per in-flight jurisdiction: the crash drill in this repository's run evidence left a
/// 377 MB and an 863 MB `.part` behind.
///
/// Two directories hold one: `entities/` for the consolidated tables and `out/` for the snapshots and
/// the sidecars published beside them, so a kill during either leaves litter that outlives every
/// reopen. A directory that does not exist yet — a store that has never consolidated — holds nothing.
pub(crate) fn sweep_stale_temporaries(root: &Path) -> StoreResult<usize> {
    let mut removed = 0_usize;
    for directory in [root.join("entities"), root.join("out")] {
        removed = removed.saturating_add(sweep_temporaries(&directory)?);
    }
    Ok(removed)
}

/// Remove the `.<name>.<process>.<sequence>.part` files one directory holds.
fn sweep_temporaries(directory: &Path) -> StoreResult<usize> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(io_failure(directory, source)),
    };
    let mut removed = 0_usize;
    for entry in entries {
        let entry = entry.map_err(|source| io_failure(directory, source))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with('.') || !name.ends_with(".part") {
            continue;
        }
        let path = entry.path();
        std::fs::remove_file(&path).map_err(|source| io_failure(&path, source))?;
        removed = removed.saturating_add(1);
    }
    Ok(removed)
}
