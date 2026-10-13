use super::*;
use census_store::InputRecord;

#[allow(clippy::duplicate_mod)]
#[path = "../coverage_tests/version_snapshot.rs"]
mod retained_evidence;

#[derive(Clone, Copy)]
enum BindingHistory {
    UnboundThenBound,
    AlreadyBound,
}

fn job_reuse(
    store: &Store,
    job: &str,
    original: &crate::export::DatasetLineage,
    history: &BindingHistory,
) -> TestResult {
    match history {
        BindingHistory::AlreadyBound => {
            check!(eq; ExportDataset::for_job(store, job)?.lineage, *original);
        }
        BindingHistory::UnboundThenBound => {
            check!(matches!(
                ExportDataset::for_job(store, job),
                Err(ReportError::Invariant { ref detail }) if detail.contains("stale or foreign export input")
            ));
        }
    }
    Ok(())
}

fn archive_reuse(history: BindingHistory, job: &str) -> TestResult {
    let seeded = fixtures::seeded_store()?;
    let store = seeded.store;
    let identity = store.store_identity()?;
    let bound = manifest(identity.as_str())?;
    if matches!(history, BindingHistory::AlreadyBound) {
        store.bind_run(&bound)?;
    }
    let original = ExportDataset::for_job(&store, job)?;
    let lineage = original.lineage.clone();
    let root = super::super::freeze_v2(&store, job)?;
    let archive = store
        .out_dir()
        .join("export-inputs")
        .join(format!("{}.json", serialized_digest(&job)?));
    let archive_bytes = std::fs::read(&archive)?;
    if matches!(history, BindingHistory::UnboundThenBound) {
        store.bind_run(&bound)?;
    }
    let before =
        retained_evidence::RetainedEvidence::capture(&store, root.directory(), &[root.attempt()])?;
    for _ in 0..2 {
        let retained = super::super::freeze_v2(&store, job)?;
        check!(eq; retained, root);
        check!(eq; ExportDataset::reopen_frozen(&archive)?.lineage, lineage);
        check!(eq; super::super::to_export_dataset(&retained, &store)?.lineage, lineage);
        job_reuse(&store, job, &lineage, &history)?;
        check!(eq; std::fs::read(&archive)?, archive_bytes);
        before.unchanged(&store, root.directory())?;
    }
    drop(original);
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    check!(eq; reopened.run_manifest()?, Some(bound));
    for _ in 0..2 {
        check!(eq; super::super::freeze_v2(&reopened, job)?, root);
        check!(eq; ExportDataset::reopen_frozen(&archive)?.lineage, lineage);
        job_reuse(&reopened, job, &lineage, &history)?;
        check!(eq; std::fs::read(&archive)?, archive_bytes);
        before.unchanged(&reopened, root.directory())?;
    }
    Ok(())
}

#[test]
fn historical_unbound_json_and_v2_inputs_stay_unbound_after_live_binding_and_reopen() -> TestResult
{
    archive_reuse(
        BindingHistory::UnboundThenBound,
        "pinned-run:historical-unbound",
    )
}

#[test]
fn historical_bound_json_and_v2_inputs_retain_the_same_job_root_and_bytes() -> TestResult {
    archive_reuse(BindingHistory::AlreadyBound, "pinned-run:historical-bound")
}

#[derive(Clone, Copy)]
enum HistoricalToken {
    UnboundWithUnreconstructibleDigest,
    BoundWithoutDigest,
}

fn install_supported_root(
    store: &Store,
    current: &crate::export::FrozenRoot,
    historical: &super::super::manifest::FrozenManifest,
) -> TestResult<crate::export::FrozenRoot> {
    let directory = current.directory();
    let objects_dir = objects::directory(directory);
    let encoded = historical.encode()?;
    let digest = objects::install(&objects_dir, &encoded)?;
    let inventory = super::super::verify::inventory_of(&objects_dir, &digest, historical)?;
    let attempt = serialized_digest(&("pinned-run-historical-token", &digest))?;
    let bytes = super::super::root::payload_bytes_of(historical)?;
    let record = InputRecord::new(&attempt, bytes, &inventory)?;
    store.put_input_record(&record)?;
    let (epoch, claimed) = store.input_lease_acquire(
        &attempt,
        record.revision(),
        "pinned-run-historical-token",
        "2999-12-31",
    )?;
    let freezing = store.input_freeze(&attempt, claimed.revision, epoch, &inventory)?;
    store.input_frozen(&attempt, freezing.revision, epoch, bytes, &inventory)?;
    store.lease_release(&attempt, epoch)?;
    super::super::root::write_retained(
        directory,
        &super::super::root::Retained {
            attempt: attempt.clone(),
            manifest: digest.clone(),
        },
    )?;
    Ok(crate::export::FrozenRoot::new(
        directory.to_path_buf(),
        historical.clone(),
        digest,
        attempt,
        bytes,
    ))
}

fn historical_token_preserved(shape: HistoricalToken, job: &str) -> TestResult {
    let seeded = fixtures::seeded_store()?;
    let store = seeded.store;
    let identity = store.store_identity()?;
    let bound = manifest(identity.as_str())?;
    if matches!(shape, HistoricalToken::BoundWithoutDigest) {
        store.bind_run(&bound)?;
    }
    let current = super::super::freeze_v2(&store, job)?;
    let mut historical = current.manifest().clone();
    historical.input_token.run_manifest_digest = match shape {
        HistoricalToken::UnboundWithUnreconstructibleDigest => Some("9".repeat(64)),
        HistoricalToken::BoundWithoutDigest => None,
    };
    historical
        .input_token
        .validate_against(&historical.lineage)?;
    let root = install_supported_root(&store, &current, &historical)?;
    if matches!(shape, HistoricalToken::UnboundWithUnreconstructibleDigest) {
        store.bind_run(&bound)?;
        check!(ne; root.manifest().input_token.run_manifest_digest, Some(serialized_digest(&bound)?));
    }
    let before = retained_evidence::RetainedEvidence::capture(
        &store,
        root.directory(),
        &[current.attempt(), root.attempt()],
    )?;
    for _ in 0..2 {
        check!(eq; super::super::freeze_v2(&store, job)?, root);
        check!(eq; super::super::to_export_dataset(&root, &store)?.lineage, historical.lineage);
        before.unchanged(&store, root.directory())?;
    }
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    for _ in 0..2 {
        check!(eq; super::super::freeze_v2(&reopened, job)?, root);
        check!(eq; super::super::to_export_dataset(&root, &reopened)?.lineage, historical.lineage);
        before.unchanged(&reopened, root.directory())?;
    }
    Ok(())
}

#[test]
fn historical_unbound_token_digest_is_preserved_without_reconstructing_live_manifest_bytes(
) -> TestResult {
    historical_token_preserved(
        HistoricalToken::UnboundWithUnreconstructibleDigest,
        "pinned-run:historical-digest",
    )
}

#[test]
fn historical_bound_token_without_a_manifest_digest_keeps_the_existing_reader_meaning() -> TestResult
{
    historical_token_preserved(
        HistoricalToken::BoundWithoutDigest,
        "pinned-run:historical-absence",
    )
}
