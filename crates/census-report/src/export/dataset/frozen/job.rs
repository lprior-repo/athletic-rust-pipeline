use super::{invalid, io_error, store_identity, ExportDataset, ReportResult, Store};
use std::path::{Path, PathBuf};

pub(in crate::export::dataset) fn capture(store: &Store, job: &str) -> ReportResult<ExportDataset> {
    if job.is_empty() || job.len() > 4096 {
        return Err(invalid("export job identity must contain 1..=4096 bytes"));
    }
    let digest = super::digest(&job)?;
    let directory = store.out_dir().join("export-inputs");
    census_store::fs::create_dir_all_synced(&directory)
        .map_err(|source| io_error(&directory, source))?;
    let lock_path = directory.join(format!("{digest}.lock"));
    let _lock = capture_lock(&lock_path)?;
    capture_locked(store, job, &directory, &digest)
}

fn capture_lock(lock_path: &Path) -> ReportResult<std::fs::File> {
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|source| io_error(lock_path, source))?;
    lock.try_lock().map_err(|error| {
        let source = match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "export input is already being captured",
            ),
            std::fs::TryLockError::Error(source) => source,
        };
        io_error(lock_path, source)
    })?;
    Ok(lock)
}

fn capture_locked(
    store: &Store,
    job: &str,
    directory: &Path,
    digest: &str,
) -> ReportResult<ExportDataset> {
    let archive = directory.join(format!("{digest}.json"));
    match std::fs::symlink_metadata(&archive) {
        Ok(_) => {
            return reopen_job(store, job, directory, &archive);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error(&archive, source)),
    }
    create_job(store, job, directory, digest, &archive)
}

fn create_job(
    store: &Store,
    job: &str,
    directory: &Path,
    digest: &str,
    archive: &Path,
) -> ReportResult<ExportDataset> {
    let mut dataset = ExportDataset::load(store)?;
    dataset.lineage.export_job = Some(job.to_string());
    super::bind(&mut dataset)?;
    let temporary = temporary_path(directory, digest)?;
    dataset.save_frozen(&temporary)?;
    let publication = std::fs::hard_link(&temporary, archive)
        .map_err(|source| io_error(archive, source))
        .and_then(|()| sync_directory(directory));
    super::io::remove_owned(&temporary, publication)?;
    Ok(dataset)
}

fn reopen_job(
    store: &Store,
    job: &str,
    directory: &Path,
    archive: &Path,
) -> ReportResult<ExportDataset> {
    let retained = ExportDataset::reopen_frozen(archive)?;
    ensure_store(&retained, store)?;
    if retained.lineage.export_job.as_deref() != Some(job) {
        return Err(invalid(
            "retained export input belongs to a different logical job",
        ));
    }
    retained.ensure_current(store)?;
    sync_directory(directory)?;
    Ok(retained)
}

pub(in crate::export::dataset) fn ensure_store(
    dataset: &ExportDataset,
    store: &Store,
) -> ReportResult<()> {
    if dataset.lineage.store_identity != store_identity(store)? {
        return Err(invalid("frozen input belongs to a different census store"));
    }
    Ok(())
}

fn temporary_path(directory: &Path, digest: &str) -> ReportResult<PathBuf> {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| invalid(&format!("creating frozen input stage: {error}")))?;
    Ok(directory.join(format!(
        ".staging.{digest}.{}.{}.json",
        std::process::id(),
        time.as_nanos()
    )))
}

fn sync_directory(path: &Path) -> ReportResult<()> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(path, source))
}
