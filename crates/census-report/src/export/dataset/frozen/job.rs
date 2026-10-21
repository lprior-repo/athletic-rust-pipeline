use super::super::ExportJobRequest;
use super::{invalid, io_error, store_identity, ExportDataset, ReportResult, Store};
use std::path::Path;

mod selected;
pub(in crate::export::dataset) use selected::freshness::Freshness as SelectedJobFreshness;

pub(in crate::export::dataset) fn capture(
    store: &Store,
    job: &str,
    request: &ExportJobRequest,
) -> ReportResult<ExportDataset> {
    if job.is_empty() || job.len() > 4096 {
        return Err(invalid("export job identity must contain 1..=4096 bytes"));
    }
    let digest = super::digest(&job)?;
    let directory = store.out_dir().join("export-inputs");
    census_store::fs::create_dir_all_synced(&directory)
        .map_err(|source| io_error(&directory, source))?;
    let lock_path = directory.join(format!("{digest}.lock"));
    let _lock = super::capture_lock(&lock_path)?;
    capture_locked(store, job, &directory, &digest, request)
}

fn capture_locked(
    store: &Store,
    job: &str,
    directory: &Path,
    digest: &str,
    request: &ExportJobRequest,
) -> ReportResult<ExportDataset> {
    let archive = directory.join(format!("{digest}.json"));
    let selected_root = directory.join(format!("{digest}.selected-v1"));
    let legacy_present = selected::present(&archive)?;
    let selected_present = selected::present(&selected_root)?
        || selected::present(&selected::pending(&selected_root))?;
    if legacy_present && selected_present {
        return Err(invalid(
            "export job has conflicting retained representations",
        ));
    }
    let ExportJobRequest::SelectedPublicationV1 { selection } = request;
    if legacy_present || selected::present(&super::freeze::directory(store, job)?)? {
        return Err(invalid(
            "legacy export job cannot become selected publication",
        ));
    }
    selected::capture(store, job, selection, &selected_root)
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

fn sync_directory(path: &Path) -> ReportResult<()> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io_error(path, source))
}
