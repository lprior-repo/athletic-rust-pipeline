//! Backup helpers: directory walking, file counting, path safety, table counts.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use super::ManifestEntry;

use crate::store::{Store, Table};

impl Store {
    /// Per-table row counts from the sequence counters.
    pub(super) fn table_counts(&self) -> BTreeMap<String, u64> {
        let mut tables = BTreeMap::new();
        for table in Table::ALL {
            tables.insert(table.file().to_string(), self.sequences.appended(table));
        }
        tables
    }

    /// Walk a directory tree and copy files, collecting manifest entries.
    pub(super) fn walk_and_copy(
        &self,
        src: &Path,
        dst: &Path,
        manifest_files: &mut Vec<ManifestEntry>,
        total_files: &mut u64,
        total_bytes: &mut u64,
    ) -> Result<()> {
        if !src.exists() {
            return Ok(());
        }
        fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
            let entry = entry.with_context(|| format!("reading entry in {}", src.display()))?;
            let src_path = entry.path();
            let manifest_rel = src_path
                .strip_prefix(self.root())
                .with_context(|| format!("computing relative path for {}", src_path.display()))?;
            let copy_rel = src_path
                .strip_prefix(src)
                .with_context(|| format!("copying {}", src_path.display()))?;
            if src_path.is_dir() {
                let subdir_name = src_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let new_dst = dst.join(&subdir_name);
                self.walk_and_copy(
                    &src_path,
                    &new_dst,
                    manifest_files,
                    total_files,
                    total_bytes,
                )?;
            } else {
                let len = entry_meta_len(&src_path)?;
                let hex = sha256_of(&src_path)?;
                let manifest_str = manifest_rel.to_string_lossy().to_string();
                let dst_path = dst.join(copy_rel);
                ensure_parent_dir(&dst_path)?;
                fs::write(&dst_path, &buf_of(&src_path)?)
                    .with_context(|| format!("writing {}", dst_path.display()))?;
                manifest_files.push(ManifestEntry {
                    path: manifest_str,
                    length: len,
                    sha256: hex,
                });
                *total_files = total_files.saturating_add(1);
                *total_bytes = total_bytes.saturating_add(len);
            }
        }
        Ok(())
    }
}
/// Get file metadata length.
pub(super) fn entry_meta_len(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path)
        .with_context(|| format!("statting {}", path.display()))?
        .len())
}

/// Compute SHA-256 hex digest of a file.
pub(super) fn sha256_of(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(&buf_of(path)?);
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

/// Read entire file contents.
pub(super) fn buf_of(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("reading {}", path.display()))
}

/// Create parent directories if they do not exist.
pub(super) fn ensure_parent_dir(dst_path: &Path) -> Result<()> {
    if let Some(parent) = dst_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    Ok(())
}

/// Count lines in a file; returns 0 if the file does not exist.
#[allow(dead_code)]
pub(super) fn count_file_rows(path: &Path) -> u64 {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return 0,
        Err(_) => return u64::MAX,
    };
    let reader = BufReader::new(file);
    u64::try_from(reader.lines().count()).unwrap_or(u64::MAX)
}

/// A manifest entry path that stays inside the backup, or an error naming it.
pub(super) fn safe_entry_path(path: &str) -> Result<&Path> {
    let relative = Path::new(path);
    let escapes = relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)));
    if escapes {
        bail!("backup entry {path} escapes the backup directory");
    }
    Ok(relative)
}
