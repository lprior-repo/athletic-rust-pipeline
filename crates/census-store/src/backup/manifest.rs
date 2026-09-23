//! What a finished generation claims: every file with its length and SHA-256, and the rows each table
//! holds - and the code that takes those two measurements.
//!
//! The digests and the counts sit beside the manifest that carries them on purpose: both are read off
//! the generation *after* the last thing that writes into it, so the claim and its measurement are the
//! same fact, and the restore side compares against that same measurement rather than re-inventing it.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::{digest_file, fsync_dir};
use super::{Manifest, ManifestEntry, MANIFEST_PATH};
use crate::rows::count_rows;
use crate::{Store, StoreError, StoreResult, Table};

/// Every regular file under `root`, with its length and the digest of the bytes on disk.
///
/// This runs after the last thing that writes into a generation, so the digests describe exactly the
/// bytes a restore will read back. The listing is sorted, so two backups of the same bytes produce the
/// same manifest.
pub(super) fn digest_tree(root: &Path) -> StoreResult<Vec<ManifestEntry>> {
    let mut entries = Vec::new();
    collect_digests(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn collect_digests(root: &Path, dir: &Path, entries: &mut Vec<ManifestEntry>) -> StoreResult<()> {
    let listing = fs::read_dir(dir).map_err(|source| io_err(dir, source))?;
    for entry in listing {
        let entry = entry.map_err(|source| io_err(dir, source))?;
        let path = entry.path();
        let kind = fs::symlink_metadata(&path)
            .map_err(|source| io_err(&path, source))?
            .file_type();
        if kind.is_dir() {
            collect_digests(root, &path, entries)?;
        } else if kind.is_file() {
            let relative = path.strip_prefix(root).map_err(|_| {
                refused(format!("{} is outside {}", path.display(), root.display()))
            })?;
            let streamed = digest_file(&path)?;
            entries.push(ManifestEntry {
                path: relative.to_string_lossy().to_string(),
                length: streamed.bytes,
                sha256: streamed.sha256,
            });
        } else {
            return Err(refused(format!(
                "backup refuses {}: it is {}, and a generation carries regular files and directories only",
                path.display(),
                object_kind(kind)
            )));
        }
    }
    Ok(())
}

/// Per-table row counts as the keyspace holds them: every key under a table's prefix is one row.
///
/// The sequence counter is deliberately not used here - it is a pointer, not a count (see
/// `store/sequences.rs`), and for a derived table it says nothing at all. Counted on both sides of a
/// restore, this is what makes the manifest's `tables` an assertion about contents rather than a
/// restatement of the writer's own bookkeeping.
pub(super) fn table_row_counts(store: &Store) -> StoreResult<BTreeMap<String, u64>> {
    let mut counts = BTreeMap::new();
    for table in Table::ALL {
        counts.insert(
            table.file().to_string(),
            count_rows(&store.entities, table)?,
        );
    }
    Ok(counts)
}

/// Serialise `manifest` into the finished generation and make it durable before anything is published.
///
/// The manifest is written last and fsynced, together with the directory that holds it: it is the
/// statement "this generation is complete", so it must not become durable before the files it lists.
pub(super) fn write_manifest(generation: &Path, manifest: &Manifest) -> StoreResult<()> {
    let path = generation.join(MANIFEST_PATH);
    let json = serde_json::to_string_pretty(manifest).map_err(|source| StoreError::Json {
        detail: format!("serialising the backup manifest {}", path.display()),
        source,
    })?;
    let mut file = File::create(&path).map_err(|source| io_err(&path, source))?;
    file.write_all(json.as_bytes())
        .map_err(|source| io_err(&path, source))?;
    file.flush().map_err(|source| io_err(&path, source))?;
    file.sync_all().map_err(|source| io_err(&path, source))?;
    fsync_dir(generation)
}

/// A manifest entry path that stays inside the backup, or a refusal naming it.
pub(super) fn safe_entry_path(path: &str) -> StoreResult<&Path> {
    let relative = Path::new(path);
    let escapes = relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)));
    if escapes {
        return Err(refused(format!(
            "backup entry {path} escapes the backup directory"
        )));
    }
    Ok(relative)
}
