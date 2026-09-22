//! Store restore implementation.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use super::helpers::{ensure_parent_dir, safe_entry_path, sha256_of};
use super::{Manifest, RestoreReport};
use crate::store::Store;

impl Store {
    /// Validate the backup manifest and materialise into `to`.
    pub fn restore(from: &Path, to: &Path) -> Result<RestoreReport> {
        let from = from.to_path_buf();
        let to = to.to_path_buf();
        Self::check_restore_destination(&to)?;
        let manifest = Self::load_manifest(&from)?;
        Self::validate_manifest(&from, &manifest)?;
        let (files, bytes) = Self::materialise(&from, &to, &manifest)?;
        let restored = Store::open(&to).context("opening restored store")?;
        let tables = restored.table_counts();
        Ok(RestoreReport {
            from: from.display().to_string(),
            to: to.display().to_string(),
            files,
            bytes,
            tables,
        })
    }

    fn check_restore_destination(to: &Path) -> Result<()> {
        if to.exists() {
            let is_empty = fs::read_dir(to)
                .with_context(|| format!("reading {}", to.display()))?
                .next()
                .is_none();
            if !is_empty {
                bail!("destination {} is not empty", to.display());
            }
        } else {
            fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
        }
        Ok(())
    }

    fn load_manifest(from: &Path) -> Result<Manifest> {
        let manifest_path = from.join("backup.json");
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading backup.json from {}", from.display()))?;
        serde_json::from_str(&manifest_text).context("parsing backup manifest")
    }

    fn validate_manifest(from: &Path, manifest: &Manifest) -> Result<()> {
        for entry in &manifest.files {
            let relative = safe_entry_path(&entry.path)?;
            let src = from.join(relative);
            let meta = fs::metadata(&src)
                .with_context(|| format!("missing file in backup: {}", entry.path))?;
            if meta.len() != entry.length {
                bail!(
                    "length mismatch for {}: expected {} got {}",
                    entry.path,
                    entry.length,
                    meta.len()
                );
            }
            let digest = sha256_of(&src)?;
            if digest != entry.sha256 {
                bail!(
                    "sha256 mismatch for {}: expected {} got {}",
                    entry.path,
                    entry.sha256,
                    digest
                );
            }
        }
        Ok(())
    }

    fn materialise(from: &Path, to: &Path, manifest: &Manifest) -> Result<(u64, u64)> {
        let mut total_files: u64 = 0;
        let mut total_bytes: u64 = 0;
        for entry in &manifest.files {
            let relative = safe_entry_path(&entry.path)?;
            let src = from.join(relative);
            let dst = to.join(relative);
            ensure_parent_dir(&dst)?;
            fs::copy(&src, &dst)
                .with_context(|| format!("copying {} to {}", entry.path, to.display()))?;
            total_files = total_files.saturating_add(1);
            total_bytes = total_bytes.saturating_add(entry.length);
        }
        Ok((total_files, total_bytes))
    }
}
