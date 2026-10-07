use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::{copy_file, digest_bytes, ensure_parent_dir};
use super::generation::Generation;
use super::manifest::{safe_entry_path, table_row_counts};
use super::tree::store_relative_link;
use super::{
    Manifest, ManifestEntry, RestoreReport, MANIFEST_PATH, MANIFEST_VERSION, RESTORE_PREFIX,
};
use crate::{Store, StoreError, StoreResult, Table};

impl Store {
    pub fn restore(from: &Path, to: &Path) -> StoreResult<RestoreReport> {
        validate_source_path(from)?;
        check_restore_destination(from, to)?;
        let manifest = load_manifest(from)?;
        validate_manifest(from, &manifest)?;
        let generation = Generation::create(to, RESTORE_PREFIX)?;
        let (files, bytes, links) = materialise(from, generation.path(), &manifest)?;
        let tables = count_restored_generation(generation.path(), to)?;
        reconcile(from, &manifest, &tables)?;
        generation.publish()?;
        Ok(RestoreReport {
            from: from.display().to_string(),
            to: to.display().to_string(),
            files,
            bytes,
            links,
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
    validate_source_components(path, false)
}

fn validate_source_components(path: &Path, allow_final_link: bool) -> StoreResult<()> {
    let total = path.components().count();
    let mut checked = std::path::PathBuf::new();
    for (position, component) in path.components().enumerate() {
        checked.push(component.as_os_str());
        let kind = fs::symlink_metadata(&checked)
            .map_err(|source| io_err(&checked, source))?
            .file_type();
        if kind.is_symlink() && !(allow_final_link && position.saturating_add(1) == total) {
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
        validate_entry(from, entry)?;
    }
    Ok(())
}

fn validate_entry(from: &Path, entry: &ManifestEntry) -> StoreResult<()> {
    let relative = safe_entry_path(&entry.path)?;
    let source = from.join(relative);
    validate_source_components(&source, entry.target.is_some())?;
    let kind = fs::symlink_metadata(&source)
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => refused(format!("missing file in backup: {}", entry.path)),
            _ => io_err(&source, error),
        })?
        .file_type();
    match &entry.target {
        None => {
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
            Ok(())
        }
        Some(target) => validate_link_entry(entry, target, relative, &source, kind),
    }
}

fn validate_link_entry(
    entry: &ManifestEntry,
    target: &str,
    relative: &Path,
    source: &Path,
    kind: fs::FileType,
) -> StoreResult<()> {
    if !kind.is_symlink() {
        return Err(refused(format!(
            "backup entry {} is {}, not the symlink the manifest records",
            entry.path,
            object_kind(kind)
        )));
    }
    if entry.length != 0 {
        return Err(refused(format!(
            "backup entry {} records length {} for the symlink it holds",
            entry.path, entry.length
        )));
    }
    let parent = match relative.parent() {
        Some(parent) => parent,
        None => Path::new(""),
    };
    if !store_relative_link(parent, Path::new(target)) {
        return Err(refused(format!(
            "backup entry {} records the symlink target {target}, which leaves the backup",
            entry.path
        )));
    }
    let actual = fs::read_link(source).map_err(|source_error| io_err(source, source_error))?;
    if actual.to_string_lossy() != target {
        return Err(refused(format!(
            "symlink target mismatch for {}: the manifest records {target} and the link holds {}",
            entry.path,
            actual.display()
        )));
    }
    let digest = digest_bytes(target.as_bytes());
    if digest != entry.sha256 {
        return Err(refused(format!(
            "sha256 mismatch for {}: expected {} got {}",
            entry.path, entry.sha256, digest
        )));
    }
    Ok(())
}

fn materialise(from: &Path, to: &Path, manifest: &Manifest) -> StoreResult<(u64, u64, u64)> {
    let mut files: u64 = 0;
    let mut bytes: u64 = 0;
    let mut links: u64 = 0;
    for entry in &manifest.files {
        let relative = safe_entry_path(&entry.path)?;
        let source = from.join(relative);
        let target = to.join(relative);
        ensure_parent_dir(&target)?;
        match &entry.target {
            Some(link_target) => {
                create_relative_link(link_target, &target)?;
                links = links.saturating_add(1);
            }
            None => {
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
        }
    }
    Ok((files, bytes, links))
}

#[cfg(unix)]
fn create_relative_link(link_target: &str, target: &Path) -> StoreResult<()> {
    std::os::unix::fs::symlink(link_target, target).map_err(|source| io_err(target, source))
}

#[cfg(not(unix))]
fn create_relative_link(_link_target: &str, target: &Path) -> StoreResult<()> {
    Err(refused(format!(
        "the backup records a symlink at {}, and this host cannot create one",
        target.display()
    )))
}

fn count_restored_generation(generation: &Path, to: &Path) -> StoreResult<BTreeMap<String, u64>> {
    table_row_counts(generation).map_err(|error| {
        refused(format!(
            "the restored copy at {} does not open as a database, so {} was left as it was: {error}",
            generation.display(),
            to.display()
        ))
    })
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
