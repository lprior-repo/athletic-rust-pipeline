use super::Options;
use crate::export::ExportDataset;
use crate::report::{io_error, ReportError, ReportResult};
use census_store::Store;
use std::path::{Path, PathBuf};

mod binding;
mod checkpoint;
mod incident;
mod install;
pub(super) mod inventory;
mod lifecycle;
mod manifest;
pub(super) mod normalized;
mod pointer;
mod protocol;
mod recovery;
mod seal;
mod sidecars;
mod stage;
mod verify_sidecars;
mod versions;
pub use manifest::{
    current_workbook, open_publication, open_publication_for_seal, verify_published,
    OpenedPublication, VerifiedPublication,
};
pub use protocol::{PublicationError, RecoveryOutcome};
pub use recovery::recover_publication;
pub(super) use sidecars::{write_json, write_sidecars};
pub(super) use stage::Stage;

const ARTIFACT_REVISION: u32 = 6;

#[cfg(test)]
pub(in crate::workbook) fn recapture_manifest(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<()> {
    let captured = manifest::Manifest::recapture(directory, dataset, options)?;
    let path = directory.join("manifest.json");
    let bytes = serde_json::to_vec(&captured).map_err(|source| ReportError::Decode {
        path: path.clone(),
        line: 0,
        source,
    })?;
    std::fs::write(&path, bytes).map_err(|source| io_error(&path, source))
}

fn staging_nonce(store_identity: &str) -> ReportResult<String> {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invariant(format!("creating publication stage: {error}")))?;
    Ok(format!(
        "{}.{}.{}",
        store_identity,
        std::process::id(),
        time.as_nanos()
    ))
}

fn lock_publication(root: &Path) -> ReportResult<std::fs::File> {
    lifecycle::preflight(root)?;
    prepare_generations(root)?;
    let lock_path = root.join(".publication.lock");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|source| io_error(&lock_path, source))?;
    acquire_lock(&lock, &lock_path)?;
    Ok(lock)
}

fn prepare_generations(root: &Path) -> ReportResult<()> {
    let generations = root.join("generations");
    census_store::fs::create_dir_all_synced(&generations)
        .map_err(|source| io_error(&generations, source))?;
    let metadata =
        std::fs::symlink_metadata(&generations).map_err(|source| io_error(&generations, source))?;
    if !metadata.file_type().is_dir() {
        return Err(invariant(
            "publication generations must be a real directory".to_string(),
        ));
    }
    Ok(())
}

fn acquire_lock(lock: &std::fs::File, path: &Path) -> ReportResult<()> {
    lock.try_lock().map_err(|error| {
        let source = match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "publication is already being finalized",
            ),
            std::fs::TryLockError::Error(source) => source,
        };
        io_error(path, source)
    })
}

fn fence(root: &Path, dataset: &ExportDataset) -> ReportResult<()> {
    let pointer = root.join("current");
    match std::fs::symlink_metadata(&pointer) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error(&pointer, source)),
        Ok(_) => {}
    }
    let (_, previous) = manifest::current_generation(root)?;
    if previous.lineage.store_identity != dataset.lineage.store_identity {
        return Err(invariant(
            "stale or foreign exporter cannot replace the published generation".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn sync_directory(path: &Path) -> ReportResult<()> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(path, source))
}

pub(super) fn invariant(detail: String) -> ReportError {
    ReportError::Invariant { detail }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fence_tests;

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod crash_tests;

#[cfg(test)]
mod input_tests;

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod pinned_run_publication_tests;

#[cfg(test)]
mod catalog_tests;

#[cfg(test)]
mod scoped_coverage_tests;
