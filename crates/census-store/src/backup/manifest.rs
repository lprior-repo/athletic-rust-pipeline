use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::{digest_file, fsync_dir};
use super::{Manifest, ManifestEntry, MANIFEST_PATH};
use crate::keys;
use crate::{StoreError, StoreResult, Table};

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

pub(super) fn table_row_counts(root: &Path) -> StoreResult<BTreeMap<String, u64>> {
    let (db, entities, _journal, _meta, _receipts) =
        crate::open_keyspaces(root, crate::format::DEFAULT_CACHE_BYTES)?;
    let mut counts = BTreeMap::new();
    for table in Table::ALL {
        let mut rows = count_prefix(&entities, &keys::observation_prefix(table))?;
        if table.generation_partitioned() {
            rows =
                rows.saturating_add(count_prefix(&entities, &keys::derived_table_prefix(table))?);
        }
        counts.insert(table.file().to_string(), rows);
    }
    drop(entities);
    drop(db);
    Ok(counts)
}

fn count_prefix(entities: &fjall::Keyspace, prefix: &[u8]) -> StoreResult<u64> {
    let mut count = 0_u64;
    for guard in entities.prefix(prefix) {
        guard.key().map_err(|source| StoreError::Read { source })?;
        count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(count)
}

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
