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
    pub fn restore(from: &Path, to: &Path) -> StoreResult<RestoreReport> {
        validate_source_path(from)?;
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

fn validate_source_path(path: &Path) -> StoreResult<()> {
    let mut checked = std::path::PathBuf::new();
    for component in path.components() {
        checked.push(component.as_os_str());
        let kind = fs::symlink_metadata(&checked)
            .map_err(|source| io_err(&checked, source))?
            .file_type();
        if kind.is_symlink() {
            return Err(refused(format!(
                "backup source {} contains a symbolic link at {}",
                path.display(),
                checked.display()
            )));
        }
    }
    Ok(())
}

fn load_manifest(from: &Path) -> StoreResult<Manifest> {
    let path = from.join(MANIFEST_PATH);
    validate_source_path(&path)?;
    let text = fs::read_to_string(&path).map_err(|source| io_err(&path, source))?;
    serde_json::from_str(&text).map_err(|source| StoreError::Json {
        detail: format!("the backup manifest {} is not valid json", path.display()),
        source,
    })
}

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
        validate_source_path(&source)?;
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
