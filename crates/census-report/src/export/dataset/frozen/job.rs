use super::super::{freeze_v2, to_export_dataset};
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
    let _lock = super::capture_lock(&lock_path)?;
    capture_locked(store, job, &directory, &digest)
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
    let root = freeze_v2(store, job)?;
    let dataset = to_export_dataset(&root, store)?;
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
    let retained = super::archive::reopen(archive, |lineage| preflight(store, job, lineage))?;
    retained.ensure_current(store)?;
    sync_directory(directory)?;
    Ok(retained)
}

fn preflight(store: &Store, job: &str, lineage: &super::DatasetLineage) -> ReportResult<()> {
    super::ensure_current_policy(lineage)?;
    if lineage.store_identity != store_identity(store)? {
        return Err(invalid("frozen input belongs to a different census store"));
    }
    if lineage.export_job.as_deref() != Some(job) {
        return Err(invalid(
            "retained export input belongs to a different logical job",
        ));
    }
    let current = super::lineage(store, &store.snapshot())?;
    if lineage.source_digest != current.source_digest
        || lineage.run != current.run
        || lineage.cohort != current.cohort
    {
        return Err(invalid(
            "stale or foreign export input cannot publish changed source evidence",
        ));
    }
    Ok(())
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
