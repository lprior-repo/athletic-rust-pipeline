//! The JSONL snapshot file: how a consolidated table — and every artifact published from it, JSONL or
//! CSV — is written, published and read back.
//!
//! The snapshot is the read model every report and spreadsheet consumes, so the two rules this file
//! keeps are the ones a reader depends on: it is published by rename and never written in place, and
//! a read refuses the whole file the moment one row does not decode.
//!
//! [`publish_atomically`] is the one publication every repository artifact goes through: the
//! consolidated tables, the `best-results-<cohort>` sidecars and the CSVs `export-data` writes. An
//! in-place writer beside an atomic one is the same tear with a different extension — a reader that
//! opens the CSV mid-write sees half a file, and a crash leaves a short one that every later read
//! takes for complete.
//!
//! The two rules are one rule. Publication by rename means a reader only ever opens a file some
//! completed pass wrote, so no row a reader sees is half-written, and a row that does not decode is
//! damage rather than a racing writer. The reader used to tolerate one unparseable row as a
//! "truncated tail", which made a row damaged in the middle of the file a silent entity loss: the row
//! was skipped and the canonical school it held vanished from the read model. No tolerant read path
//! remains here. A snapshot is a derived artifact — `Store::consolidate` republishes a damaged one
//! from the store — so the operator's repair is a consolidate pass, not a dropped row.

use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::super::{StoreError, StoreResult};
use tracing::debug;

/// Publish any serializable rows as this directory's JSONL snapshot: one JSON object per line,
/// newline-terminated. Every `out/*.jsonl` a reader opens goes through this one path — the
/// consolidated tables and the `best-results-<cohort>` sidecars emitted beside them.
pub fn write_snapshot_rows<T: Serialize>(path: &Path, rows: &[T]) -> StoreResult<()> {
    publish_atomically(path, |temporary| write_rows(temporary, path, rows))
}

/// Publish `path` by writing a temporary in its directory and renaming over the name, and make the
/// result durable.
///
/// The destination is never written in place. Two writers can hold the same path at once — the
/// durable national run consolidates one jurisdiction per object, `midwest-serve` runs several of
/// those concurrently, and a re-consolidation replaces a table while a `report`, `bests` or
/// `workbook` pass is reading it — so a temporary in the destination directory plus one rename is
/// what makes every reader see one complete artifact or the other rather than a half-written one. The
/// temporary sits *beside* the destination rather than in the system temporary directory because a
/// rename is only atomic within one filesystem. The directory is created here so a fresh store root
/// works.
///
/// `body` writes the bytes and is responsible for nothing else: this function pushes them onto the
/// disk before the rename ([`sync_file`]) and the directory entry after it
/// ([`sync_parent_directory`]), so a publication that returned has left an artifact a machine loss
/// cannot take back — not a name that resolves to bytes still sitting in the page cache. A temporary
/// left by a refused body or a refused rename is removed; the sweep at store open reclaims the one a
/// killed process leaves.
pub fn publish_atomically(
    path: &Path,
    body: impl FnOnce(&Path) -> StoreResult<()>,
) -> StoreResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| io_failure(parent, source))?;
    }
    let temporary = temporary_path(path);
    let staged = body(&temporary).and_then(|()| sync_file(&temporary, path));
    if let Err(failure) = staged {
        discard_temporary(&temporary);
        return Err(failure);
    }
    match std::fs::rename(&temporary, path) {
        Ok(()) => sync_parent_directory(path),
        Err(source) => {
            discard_temporary(&temporary);
            Err(io_failure(path, source))
        }
    }
}

/// Write every row into `temporary`, the file [`publish_atomically`] renames to `published`.
///
/// Failures name `published`: the staged name is an implementation detail of the publication, and the
/// operator acting on a refused row or a full disk needs the artifact's name rather than the name it
/// was staged under.
///
/// This is [`open_snapshot_writer`] plus one [`SnapshotRowWriter::push`] per row. A caller that
/// produces its rows one at a time — a streaming merge, an export — opens the writer itself and never
/// holds them all; both go through the same serializer.
fn write_rows<T: Serialize>(temporary: &Path, published: &Path, rows: &[T]) -> StoreResult<()> {
    let mut writer = open_snapshot_writer(temporary, published)?;
    for row in rows {
        writer.push(row)?;
    }
    writer.finish()
}

/// Stage `published` and hand back a writer that appends one row at a time.
///
/// `temporary` is the staged path [`publish_atomically`] renames to `published`, and `published` is
/// the name every failure reports: an operator acting on a refused row or a full disk needs the
/// artifact's name, not the name the row was staged under. The destination's parent directory is
/// [`publish_atomically`]'s to create and durability is its to make once this writer is finished, so
/// this function creates one file and nothing else.
pub(in crate::store) fn open_snapshot_writer<'a>(
    temporary: &Path,
    published: &'a Path,
) -> StoreResult<SnapshotRowWriter<'a>> {
    let file = std::fs::File::create(temporary).map_err(|source| io_failure(published, source))?;
    Ok(SnapshotRowWriter {
        writer: BufWriter::new(file),
        published,
    })
}

/// A snapshot being written a row at a time: the staged file, and the published name a failure has to
/// report.
///
/// One serializer for every artifact here: [`write_snapshot_rows`] feeds it a slice, a consolidation
/// feeds it the rows its merge streams, and neither knows how the other got them.
pub(in crate::store) struct SnapshotRowWriter<'a> {
    writer: BufWriter<std::fs::File>,
    published: &'a Path,
}

impl SnapshotRowWriter<'_> {
    /// Append one row, JSON-encoded and newline-terminated.
    pub(in crate::store) fn push<T: Serialize>(&mut self, row: &T) -> StoreResult<()> {
        serde_json::to_writer(&mut self.writer, row)
            .map_err(|source| json_failure(self.published, source))?;
        self.writer
            .write_all(b"\n")
            .map_err(|source| io_failure(self.published, source))
    }

    /// Flush the staged bytes. Durability is [`publish_atomically`]'s — it syncs this file and then the
    /// directory entry the rename installs — so a second sync here would be one guarantee with two
    /// owners and a wasted syscall per artifact.
    pub(in crate::store) fn finish(mut self) -> StoreResult<()> {
        self.writer
            .flush()
            .map_err(|source| io_failure(self.published, source))
    }
}

/// Push the bytes `body` wrote onto the disk before the name that publishes them exists.
///
/// A body's `flush` only empties user-space buffers into the page cache, which is not where a
/// published artifact is allowed to live: a rename publishes a name, and a machine loss between the
/// rename and the writeback would leave that name over a file whose bytes never reached the disk. This
/// handle is opened after the body closed its own, so no format's writer has to remember to sync.
fn sync_file(temporary: &Path, published: &Path) -> StoreResult<()> {
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(temporary)
        .map_err(|source| io_failure(published, source))?;
    handle
        .sync_all()
        .map_err(|source| io_failure(published, source))
}

/// Remove the temporary of a publication that did not finish: litter beside the destination rather
/// than state, and the sweep at store open reclaims whatever a killed process left.
fn discard_temporary(temporary: &Path) {
    if let Err(cleanup) = std::fs::remove_file(temporary) {
        debug!(
            ?cleanup,
            ?temporary,
            "snapshot temporary could not be removed"
        );
    }
}

/// Make the directory entry the rename installed in `path`'s parent durable.
///
/// A rename is a change to a directory rather than to a file, and a POSIX filesystem only promises
/// that change after the directory's own handle is synced. Without this sync a machine loss can take
/// the rename while the synced bytes sit under the temporary's name — the reader gets the previous
/// snapshot, or no name at all, and the consolidation that returned has lost its artifact.
///
/// A bare file name (`schools.jsonl`) has no directory entry this code created, so there is nothing
/// to sync and the publication stands as the writer's `sync_all` left it.
fn sync_parent_directory(path: &Path) -> StoreResult<()> {
    let Some(directory) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    let handle = std::fs::File::open(directory).map_err(|source| io_failure(directory, source))?;
    handle
        .sync_all()
        .map_err(|source| io_failure(directory, source))
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
///
/// Two directories hold one: `entities/` for the consolidated tables and `out/` for the snapshots and
/// the sidecars published beside them, so a kill during either leaves litter that outlives every
/// reopen. A directory that does not exist yet — a store that has never consolidated — holds nothing.
pub(in crate::store) fn sweep_stale_temporaries(root: &Path) -> StoreResult<usize> {
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

/// A `csv` refusal while publishing `published`: a failure of the file keeps the artifact's path and
/// its source, and a row the writer refused is reported with the message the writer raised.
pub fn csv_failure(published: &Path, error: csv::Error) -> StoreError {
    let detail = error.to_string();
    match error.into_kind() {
        csv::ErrorKind::Io(source) => StoreError::Io {
            path: published.to_path_buf(),
            source,
        },
        _ => StoreError::Invariant {
            detail: format!("writing {}: {detail}", published.display()),
        },
    }
}
/// Read a JSONL snapshot file into typed rows.
///
/// Every non-blank line has to decode. One row that does not is a [`StoreError::SnapshotRow`] naming
/// the file and the line that holds it, and the read hands back no rows at all: a caller working from
/// a snapshot that dropped a canonical entity is the failure mode this refuses.
///
/// An absent file is an empty table rather than an error — the adapters read `out/schools.jsonl`
/// before a `consolidate` pass has ever run — and a line that is not valid UTF-8 fails as a
/// [`StoreError::Io`] naming the file, the same as any other refusal to read its bytes.
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
            Err(source) => {
                return Err(StoreError::SnapshotRow {
                    path: path.to_path_buf(),
                    line: index.saturating_add(1),
                    source,
                });
            }
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests;
