//! Store restore implementation.
//!
//! Restore is staged and reconciled: the backup is materialised into a temporary sibling, opened and
//! counted, and only then renamed onto the requested destination. A restore that fails anywhere - a
//! digest that does not match, a restored tree that does not open, row counts that disagree with the
//! manifest - leaves the destination as it found it, so the retry is not blocked by the wreckage of
//! the attempt.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::{copy_file, ensure_parent_dir};
use super::generation::Generation;
use super::manifest::{safe_entry_path, table_row_counts};
use super::{Manifest, RestoreReport, MANIFEST_PATH, MANIFEST_VERSION, RESTORE_PREFIX};
use crate::{Store, StoreError, StoreResult, Table};

impl Store {
    /// Validate the backup at `from` and materialise it into `to`.
    ///
    /// The manifest's version must be [`MANIFEST_VERSION`], every file it lists must exist as a regular
    /// file of the recorded length, and each file is then streamed once into the staging generation
    /// with its SHA-256 checked as the bytes pass. The staged tree is opened as a store and its
    /// per-table row counts are reconciled against the manifest's before anything is renamed onto
    /// `to`, which must be absent or an empty directory.
    pub fn restore(from: &Path, to: &Path) -> StoreResult<RestoreReport> {
        check_restore_destination(from, to)?;
        let manifest = load_manifest(from)?;
        validate_manifest(from, &manifest)?;
        let generation = Generation::create(to, RESTORE_PREFIX)?;
        let (files, bytes) = materialise(from, generation.path(), &manifest)?;
        let tables = count_restored_generation(generation.path(), to)?;
        reconcile(from, &manifest, &tables)?;
        generation.publish()?;
        Ok(RestoreReport {
            from: from.display().to_string(),
            to: to.display().to_string(),
            files,
            bytes,
            tables,
        })
    }
}

/// Refuse a destination a restore cannot be published onto.
fn check_restore_destination(from: &Path, to: &Path) -> StoreResult<()> {
    if to.starts_with(from) {
        return Err(refused(format!(
            "destination {} is inside the backup at {}: a restore cannot be written into the backup it \
             is reading",
            to.display(),
            from.display()
        )));
    }
    let kind = match fs::symlink_metadata(to) {
        Ok(meta) => meta.file_type(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_err(to, source)),
    };
    if !kind.is_dir() {
        return Err(refused(format!(
            "destination {} is not a directory",
            to.display()
        )));
    }
    let mut entries = fs::read_dir(to).map_err(|source| io_err(to, source))?;
    match entries.next() {
        None => Ok(()),
        Some(_) => Err(refused(format!(
            "destination {} is not empty",
            to.display()
        ))),
    }
}

/// Read and parse `backup.json`.
fn load_manifest(from: &Path) -> StoreResult<Manifest> {
    let path = from.join(MANIFEST_PATH);
    let text = fs::read_to_string(&path).map_err(|source| io_err(&path, source))?;
    serde_json::from_str(&text).map_err(|source| StoreError::Json {
        detail: format!("the backup manifest {} is not valid json", path.display()),
        source,
    })
}

/// The manifest's own rules: a version this build reads, and entries that are regular files of the
/// recorded length.
///
/// The digest is checked while the file is copied rather than here, so the backup is read once.
fn validate_manifest(from: &Path, manifest: &Manifest) -> StoreResult<()> {
    if manifest.version != MANIFEST_VERSION {
        return Err(refused(format!(
            "the backup manifest {} is version {}, which this build does not read (it reads version \
             {}): take the backup again with this build",
            from.join(MANIFEST_PATH).display(),
            manifest.version,
            MANIFEST_VERSION
        )));
    }
    for entry in &manifest.files {
        let relative = safe_entry_path(&entry.path)?;
        let source = from.join(relative);
        let kind = fs::symlink_metadata(&source)
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => {
                    refused(format!("missing file in backup: {}", entry.path))
                }
                _ => io_err(&source, error),
            })?
            .file_type();
        if !kind.is_file() {
            return Err(refused(format!(
                "backup entry {} is {}, not a regular file",
                entry.path,
                object_kind(kind)
            )));
        }
        let length = fs::metadata(&source)
            .map_err(|source_error| io_err(&source, source_error))?
            .len();
        if length != entry.length {
            return Err(refused(format!(
                "length mismatch for {}: expected {} got {}",
                entry.path, entry.length, length
            )));
        }
    }
    Ok(())
}

/// Stream every entry into the staging generation, checking digest and length as the bytes pass.
fn materialise(from: &Path, to: &Path, manifest: &Manifest) -> StoreResult<(u64, u64)> {
    let mut files: u64 = 0;
    let mut bytes: u64 = 0;
    for entry in &manifest.files {
        let relative = safe_entry_path(&entry.path)?;
        let source = from.join(relative);
        let target = to.join(relative);
        ensure_parent_dir(&target)?;
        let streamed = copy_file(&source, &target)?;
        if streamed.bytes != entry.length {
            return Err(refused(format!(
                "length mismatch for {}: expected {} got {}",
                entry.path, entry.length, streamed.bytes
            )));
        }
        if streamed.sha256 != entry.sha256 {
            return Err(refused(format!(
                "sha256 mismatch for {}: expected {} got {}",
                entry.path, entry.sha256, streamed.sha256
            )));
        }
        files = files.saturating_add(1);
        bytes = bytes.saturating_add(streamed.bytes);
    }
    Ok((files, bytes))
}

/// Open the materialised tree as a store and count the rows its keyspace holds.
///
/// The open is the check item 21 asks for: a tree that does not open is not a restore, and it fails
/// while the destination is still untouched.
fn count_restored_generation(generation: &Path, to: &Path) -> StoreResult<BTreeMap<String, u64>> {
    let store = Store::open(generation).map_err(|error| {
        refused(format!(
            "the restored copy at {} does not open as a store, so {} was left as it was: {error}",
            generation.display(),
            to.display()
        ))
    })?;
    let counts = table_row_counts(&store)?;
    drop(store);
    Ok(counts)
}

/// The manifest's row counts against the rows the restored keyspace actually holds.
///
/// The two can only disagree if the restore lost or invented rows, which is exactly what a file digest
/// cannot see: an engine that drops an unreadable segment on recovery opens cleanly and answers with
/// fewer rows. The destination must not become a store that claims to be the backup, so a disagreement
/// is a refusal and the caller's destination is left empty.
fn reconcile(
    from: &Path,
    manifest: &Manifest,
    restored: &BTreeMap<String, u64>,
) -> StoreResult<()> {
    let path = from.join(MANIFEST_PATH);
    if manifest.tables.len() != Table::ALL.len() {
        return Err(refused(format!(
            "the backup manifest {} records {} tables; a store has {}",
            path.display(),
            manifest.tables.len(),
            Table::ALL.len()
        )));
    }
    for table in Table::ALL {
        let name = table.file();
        let expected = manifest.tables.get(name).ok_or_else(|| {
            refused(format!(
                "the backup manifest {} records no row count for table {name}",
                path.display()
            ))
        })?;
        let actual = restored.get(name).ok_or_else(|| {
            refused(format!(
                "the restored store has no count for table {name}, which the manifest records"
            ))
        })?;
        if expected != actual {
            return Err(refused(format!(
                "restored table {name} holds {actual} rows; the manifest {} records {expected}",
                path.display()
            )));
        }
    }
    Ok(())
}
