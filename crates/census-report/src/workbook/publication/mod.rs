use super::Options;
use crate::export::ExportDataset;
use crate::report::{io_error, ReportError, ReportResult};
use census_store::Store;
use std::path::{Path, PathBuf};

mod lifecycle;
mod manifest;
mod seal;
mod sidecars;
mod verify_sidecars;
pub use manifest::{current_workbook, verify_for_seal, verify_published, VerifiedPublication};
pub(super) use sidecars::write_sidecars;

pub(super) struct Stage<'s> {
    root: PathBuf,
    directory: PathBuf,
    pointer: PathBuf,
    _lock: std::fs::File,
    store: &'s Store,
}

impl<'s> Stage<'s> {
    pub(super) fn begin(
        dataset: &ExportDataset,
        store: &'s Store,
        options: &Options,
    ) -> ReportResult<Self> {
        dataset.ensure_current(store)?;
        let root = match options.out.clone() {
            Some(value) => value,
            None => store.out_dir().join("publication"),
        };
        let lock = lock_publication(&root)?;
        fence(&root, dataset)?;
        lifecycle::sweep(&root, &dataset.lineage.store_identity)?;
        let nonce = staging_nonce(&dataset.lineage.store_identity)?;
        let directory = root.join(format!(".staging.{nonce}"));
        let pointer = root.join(format!(".current.{nonce}.tmp"));
        std::fs::create_dir(&directory).map_err(|source| io_error(&directory, source))?;
        Ok(Self {
            root,
            directory,
            pointer,
            _lock: lock,
            store,
        })
    }

    pub(super) fn publish(
        self,
        dataset: &ExportDataset,
        options: &Options,
        render: impl FnOnce(&Path) -> ReportResult<()>,
    ) -> ReportResult<PathBuf> {
        let result = render(&self.directory).and_then(|()| self.commit(dataset, options));
        lifecycle::finish(result, &self.root, &self.directory, &self.pointer)
    }

    fn commit(&self, dataset: &ExportDataset, options: &Options) -> ReportResult<PathBuf> {
        let manifest = manifest::Manifest::capture(&self.directory, dataset, options)?;
        sidecars::write_json(&self.directory.join("manifest.json"), &manifest)?;
        sync_directory(&self.directory)?;
        manifest::verify_prepared(&self.directory, dataset, options)?;
        fence(&self.root, dataset)?;
        let target = self
            .root
            .join("generations")
            .join(&manifest.generation_digest);
        self.retain_generation(&target, &manifest.generation_digest)?;
        sync_directory(&self.root.join("generations"))?;
        self.switch_pointer(dataset, &manifest.generation_digest)?;
        Ok(target.join("workbook.xlsx"))
    }

    fn retain_generation(&self, target: &Path, digest: &str) -> ReportResult<()> {
        if target.exists() {
            let retained = manifest::verify_directory(target)?;
            if retained.generation_digest != digest {
                return Err(invariant("publication generation collision".to_string()));
            }
            std::fs::remove_dir_all(&self.directory)
                .map_err(|source| io_error(&self.directory, source))?;
        } else {
            std::fs::rename(&self.directory, target).map_err(|source| io_error(target, source))?;
        }
        Ok(())
    }

    fn switch_pointer(&self, dataset: &ExportDataset, digest: &str) -> ReportResult<()> {
        std::os::unix::fs::symlink(Path::new("generations").join(digest), &self.pointer)
            .map_err(|source| io_error(&self.pointer, source))?;
        let source_fence = self.store.fenced_snapshot();
        dataset.ensure_snapshot(source_fence.view())?;
        std::fs::rename(&self.pointer, self.root.join("current"))
            .map_err(|source| io_error(&self.root.join("current"), source))?;
        sync_directory(&self.root)
    }
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
