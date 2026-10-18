use super::{invalid, io_error, ExportDataset, ExportJobRequest, ReportResult, Store};
use census_domain::model::ProfileSelectionV1;
use std::path::{Path, PathBuf};

mod commitment;
pub(super) mod freshness;
use commitment::Commitment;

pub(super) fn present(path: &Path) -> ReportResult<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(io_error(path, source)),
    }
}

pub(super) fn pending(root: &Path) -> PathBuf {
    root.with_extension("selected-v1.pending")
}

pub(super) fn capture(
    store: &Store,
    job: &str,
    selection: &ProfileSelectionV1,
    root: &Path,
) -> ReportResult<ExportDataset> {
    let pending = pending(root);
    if present(root)? {
        if present(&pending)? {
            return Err(invalid(
                "selected export job has both retained and pending inputs",
            ));
        }
        let dataset = reopen(store, job, selection, root)?;
        super::sync_directory(
            root.parent()
                .ok_or_else(|| invalid("selected job has no parent"))?,
        )?;
        return Ok(dataset);
    }
    if !present(&pending)? {
        prepare(store, job, selection, &pending)?;
    }
    drop(reopen(store, job, selection, &pending)?);
    sync_bundle(&pending)?;
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        &pending,
        rustix::fs::CWD,
        root,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|source| io_error(root, source.into()))?;
    super::sync_directory(
        root.parent()
            .ok_or_else(|| invalid("selected job has no parent"))?,
    )?;
    reopen(store, job, selection, root)
}

fn prepare(
    store: &Store,
    job: &str,
    selection: &ProfileSelectionV1,
    pending: &Path,
) -> ReportResult<()> {
    let snapshot = store.snapshot();
    let profile = snapshot.selected_profile(selection)?;
    let freshness = freshness::capture(&snapshot, &profile)?;
    std::fs::create_dir(pending).map_err(|source| io_error(pending, source))?;
    super::sync_directory(
        pending
            .parent()
            .ok_or_else(|| invalid("pending selected job has no parent"))?,
    )?;
    let archive = pending.join("input.json");
    let objects = super::super::proof_directory(&archive)?;
    std::fs::create_dir(&objects).map_err(|source| io_error(&objects, source))?;
    let dataset =
        ExportDataset::selected_snapshot_for_job(store, &snapshot, selection, &objects, Some(job))?;
    let commitment = Commitment::new(job, selection, &dataset, freshness)?;
    dataset.save_frozen(&archive)?;
    commitment.write(&pending.join("commitment.json"))?;
    sync_bundle(pending)
}

fn reopen(
    store: &Store,
    job: &str,
    selection: &ProfileSelectionV1,
    root: &Path,
) -> ReportResult<ExportDataset> {
    require_directory(root)?;
    let commitment = Commitment::read(&root.join("commitment.json"))?;
    commitment.require_request(store, job, selection)?;
    let snapshot = store.snapshot();
    let profile = snapshot.selected_profile(selection)?;
    if freshness::capture(&snapshot, &profile)? != commitment.freshness {
        return Err(invalid(
            "selected export job authority changed; retained input is preserved",
        ));
    }
    let mut dataset = super::super::archive::reopen(
        &root.join("input.json"),
        Some(&commitment.archive_digest),
        |lineage| super::preflight(store, job, lineage),
    )?;
    if dataset.selected_profile_id() != Some(selection.profile_id())
        || dataset.selected_authority()?.profile() != &profile
    {
        return Err(invalid(
            "selected export job differs from its immutable profile selection",
        ));
    }
    dataset.selected_job_freshness = Some(commitment.freshness);
    dataset.ensure_current_selected_write()?;
    dataset.ensure_snapshot(&snapshot)?;
    sync_bundle(root)?;
    Ok(dataset)
}

fn require_directory(path: &Path) -> ReportResult<()> {
    if !std::fs::symlink_metadata(path)
        .map_err(|source| io_error(path, source))?
        .file_type()
        .is_dir()
    {
        return Err(invalid("selected export input is not an owned directory"));
    }
    Ok(())
}

fn sync_bundle(root: &Path) -> ReportResult<()> {
    require_directory(root)?;
    let mut count = 0usize;
    for entry in std::fs::read_dir(root).map_err(|source| io_error(root, source))? {
        let entry = entry.map_err(|source| io_error(root, source))?;
        count = count
            .checked_add(1)
            .ok_or_else(|| invalid("selected inventory counter exhausted"))?;
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
        match entry.file_name().to_str() {
            Some("input.json.proofs") if metadata.file_type().is_dir() => {
                super::sync_directory(&path)?
            }
            Some("input.json" | "commitment.json") if metadata.file_type().is_file() => {
                std::fs::File::open(&path)
                    .and_then(|file| file.sync_all())
                    .map_err(|source| io_error(&path, source))?;
            }
            _ => {
                return Err(invalid(
                    "selected export input contains an unexpected artifact",
                ))
            }
        }
        if count > 3 {
            return Err(invalid("selected export input inventory exceeds its bound"));
        }
    }
    if count != 3 {
        return Err(invalid("selected export input is incomplete"));
    }
    super::sync_directory(root)
}
