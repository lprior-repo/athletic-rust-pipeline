//! The JSONL snapshot file: how a consolidated table is written, published and read back.
//!
//! The snapshot is the read model every report and spreadsheet consumes, so the two rules this file
//! keeps are the ones a reader depends on: it is published by rename and never written in place, and
//! the reader tolerates exactly one unparseable row (a truncated tail can lose one trailing row and
//! nothing else).

use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::super::{Entity, StoreError, StoreResult};
use tracing::debug;

/// Write one JSON object per row, newline-terminated, beside the store; report the mailboxes the
/// merge withheld on the way out. The directory is created here so a fresh store root works.
///
/// The snapshot is published by rename, never written in place. Two writers can hold the same
/// snapshot path at once — the durable national run consolidates one jurisdiction per object, and
/// `midwest-serve` runs several of those concurrently — and a reader (`report`, `bests`, `workbook`,
/// or an operator reading the file) may be reading the path while the next pass replaces it. A
/// temporary file in the destination directory plus a rename means every reader sees one complete
/// snapshot or the other; the previous code truncated in place, so a reader could catch a half-file
/// and two writers could interleave into one.
pub(in crate::store) fn write_snapshot<T: Entity>(path: &Path, rows: &[T]) -> StoreResult<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| io_failure(parent, source))?;
    }
    let temporary = temporary_path(path);
    let withheld = write_rows(&temporary, rows)?;
    match std::fs::rename(&temporary, path) {
        Ok(()) => Ok(withheld),
        Err(source) => {
            // Best-effort cleanup: the rename is what failed, and a stale temporary beside the
            // snapshot is litter rather than state.
            if let Err(cleanup) = std::fs::remove_file(&temporary) {
                debug!(
                    ?cleanup,
                    ?temporary,
                    "snapshot temporary could not be removed"
                );
            }
            Err(io_failure(path, source))
        }
    }
}

/// Write every row into `temporary` and flush it to the operating system.
fn write_rows<T: Entity>(temporary: &Path, rows: &[T]) -> StoreResult<usize> {
    let file = std::fs::File::create(temporary).map_err(|source| io_failure(temporary, source))?;
    let mut writer = BufWriter::new(file);
    let mut withheld = 0_usize;
    for record in rows {
        withheld = withheld.saturating_add(record.withheld_mailboxes());
        serde_json::to_writer(&mut writer, record)
            .map_err(|source| json_failure(temporary, source))?;
        writer
            .write_all(b"\n")
            .map_err(|source| io_failure(temporary, source))?;
    }
    writer
        .flush()
        .map_err(|source| io_failure(temporary, source))?;
    Ok(withheld)
}

/// The temporary a snapshot is written to before it is published.
///
/// Unique per call, not per process: several consolidations run inside one `midwest-serve`, so a
/// name built from the process id alone would have two of them writing the same temporary.
fn temporary_path(path: &Path) -> PathBuf {
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
pub(in crate::store) fn sweep_stale_temporaries(root: &Path) -> StoreResult<usize> {
    let entities = root.join("entities");
    let entries = match std::fs::read_dir(&entities) {
        Ok(entries) => entries,
        // A store that has never consolidated has no entities directory; nothing to sweep.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(io_failure(&entities, source)),
    };
    let mut removed = 0_usize;
    for entry in entries {
        let entry = entry.map_err(|source| io_failure(&entities, source))?;
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

/// A filesystem refusal while writing `path`.
fn io_failure(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// An encode refusal while writing `path`: a row that never encoded has no key to name.
fn json_failure(path: &Path, source: serde_json::Error) -> StoreError {
    StoreError::Json {
        detail: format!("writing {}", path.display()),
        source,
    }
}
/// Read a JSONL snapshot file into typed rows.
///
/// A single unparseable row is tolerated; a second one is a [`StoreError::Json`], because a
/// truncated tail can lose one trailing row and nothing else.
pub fn read_rows<T: for<'de> serde::Deserialize<'de>>(
    path: &std::path::Path,
) -> StoreResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut rows = Vec::new();
    let mut unparseable = 0usize;
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| StoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(row) => rows.push(row),
            Err(error) => {
                unparseable = unparseable.saturating_add(1);
                if unparseable > 1 {
                    return Err(StoreError::Json {
                        detail: format!("{}:{}", path.display(), index.saturating_add(1)),
                        source: error,
                    });
                }
            }
        }
    }
    Ok(rows)
}
