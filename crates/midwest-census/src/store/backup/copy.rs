//! Store backup implementation.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use super::{BackupReport, Manifest, ManifestEntry, MANIFEST_PATH, MANIFEST_VERSION};
use crate::store::Store;

impl Store {
    /// Copy the store's durable material into `to` and write a manifest.
    pub fn backup(&self, to: &Path) -> Result<BackupReport> {
        let start = std::time::Instant::now();
        let to = to.to_path_buf();
        self.check_backup_destination(&to)?;
        let (manifest_files, total_files, total_bytes) = self.collect_and_copy(&to)?;
        self.write_manifest(&to, &manifest_files)?;
        let tables = self.table_counts();
        Ok(BackupReport {
            to: to.display().to_string(),
            files: total_files,
            bytes: total_bytes,
            tables,
            elapsed_ms: start.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        })
    }

    fn check_backup_destination(&self, to: &Path) -> Result<()> {
        if to.exists() {
            let is_backup = to
                .join(MANIFEST_PATH)
                .try_exists()
                .context("checking destination for backup manifest")?;
            if !is_backup {
                let entries: Vec<_> = fs::read_dir(to)
                    .with_context(|| format!("listing {}", to.display()))?
                    .filter_map(|e| e.ok())
                    .collect();
                if !entries.is_empty() {
                    bail!(
                        "destination {} is not empty and is not a backup",
                        to.display()
                    );
                }
            }
        } else {
            fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
        }
        Ok(())
    }

    fn collect_and_copy(&self, to: &Path) -> Result<(Vec<ManifestEntry>, u64, u64)> {
        let mut manifest_files = Vec::new();
        let mut total_files: u64 = 0;
        let mut total_bytes: u64 = 0;
        for dir_name in &["fjall", "entities", "journal", "http", "out"] {
            let src = self.root().join(dir_name);
            if !src.exists() {
                continue;
            }
            let dst = to.join(dir_name);
            self.walk_and_copy(
                &src,
                &dst,
                &mut manifest_files,
                &mut total_files,
                &mut total_bytes,
            )?;
        }
        Ok((manifest_files, total_files, total_bytes))
    }

    fn write_manifest(&self, to: &Path, manifest_files: &[ManifestEntry]) -> Result<()> {
        let tables = self.table_counts();
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            written_at: chrono::Utc::now().to_rfc3339(),
            files: manifest_files.to_vec(),
            tables,
        };
        let manifest_path = to.join(MANIFEST_PATH);
        let json =
            serde_json::to_string_pretty(&manifest).context("serialising backup manifest")?;
        fs::write(&manifest_path, json)
            .with_context(|| format!("writing {}", manifest_path.display()))
    }
}
