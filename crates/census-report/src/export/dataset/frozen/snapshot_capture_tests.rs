use super::super::fixtures::{school, seeded_store, TestResult};
use super::super::{materialize, pinned_run_metadata_tests, INPUT_TABLES};
use super::{capture, capture_at};
use crate::report::{ExportResourceError, ReportError};
use census_domain::model::{serialized_digest, CanonicalSchool};
use census_store::{Store, StoreError, StoreLineageError, Table};

#[test]
fn snapshot_capture_keeps_rows_metadata_and_extension_reads_on_the_owned_view() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let identity = store.store_identity()?;
    store.journal_done("snapshot-seam-fixture", "before", &"original")?;
    let snapshot = store.snapshot();
    let source = snapshot.tables_digest(&INPUT_TABLES)?;
    let bound = pinned_run_metadata_tests::manifest(identity.as_str())?;
    store.bind_run(&bound)?;
    store.append(Table::Schools, &school("Observed After Snapshot"))?;
    store.journal_done("snapshot-seam-fixture", "after", &"new")?;
    let sequence = store.snapshot().sequence();
    let directory = tempfile::tempdir()?;
    let captured = capture_at(
        &store.clone(),
        &snapshot,
        "snapshot-seam:old",
        directory.path(),
    )?;
    check!(eq; captured.lineage.store_identity, identity.as_str());
    check!(eq; captured.lineage.snapshot_sequence, snapshot.sequence());
    check!(eq; captured.lineage.source_digest, source);
    check!(eq; captured.lineage.run, None);
    check!(eq; captured.lineage.cohort, None);
    check!(eq; captured.lineage.export_job.as_deref(), Some("snapshot-seam:old"));
    check!(eq; captured.input_token.run_manifest_digest, None);
    captured.input_token.validate_against(&captured.lineage)?;
    let loaded = materialize::read_all(directory.path(), &captured)?;
    check!(eq; loaded.schools.len(), 1);
    check!(eq; loaded.schools.first().map(|row| row.name.as_str()), Some("Frozen High"));
    check!(eq; loaded.athletes.len(), 2);
    check!(eq; snapshot.journal_payload("snapshot-seam-fixture", "before")?, Some(serde_json::json!("original")));
    check!(eq; snapshot.journal_payload("snapshot-seam-fixture", "after")?, None);
    let current = store.snapshot();
    check!(eq; current.sequence(), sequence);
    check!(eq; current.run_manifest()?, Some(bound));
    check!(eq; current.scan::<CanonicalSchool>(Table::Schools)?.len(), 2);
    check!(eq; current.journal_payload("snapshot-seam-fixture", "after")?, Some(serde_json::json!("new")));
    check!(ne; current.tables_digest(&INPUT_TABLES)?, captured.lineage.source_digest);
    for table in INPUT_TABLES {
        let fingerprint = captured
            .fingerprint(table)
            .ok_or("missing base fingerprint")?;
        check!(eq; fingerprint.digest, snapshot.tables_digest(&[table])?);
    }
    Ok(())
}

#[test]
fn snapshot_capture_matches_legacy_streams_without_advancing_the_store() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let identity = store.store_identity()?;
    let bound = pinned_run_metadata_tests::manifest(identity.as_str())?;
    store.bind_run(&bound)?;
    let snapshot = store.snapshot();
    let direct_directory = tempfile::tempdir()?;
    let legacy_directory = tempfile::tempdir()?;
    let direct = capture_at(
        &store,
        &snapshot,
        "snapshot-seam:parity",
        direct_directory.path(),
    )?;
    let legacy = capture(&store, "snapshot-seam:parity", legacy_directory.path())?;
    check!(eq; direct.streams, legacy.streams);
    check!(eq; direct.source_fingerprints, legacy.source_fingerprints);
    check!(eq; direct.legacy_logical_digest, legacy.legacy_logical_digest);
    check!(eq; direct.lineage.input_digest, legacy.lineage.input_digest);
    check!(eq; direct.lineage.source_digest, legacy.lineage.source_digest);
    check!(eq; direct.lineage.snapshot_sequence, snapshot.sequence());
    check!(eq; legacy.lineage.snapshot_sequence, snapshot.sequence());
    check!(eq; direct.input_token.run_manifest_digest, Some(serialized_digest(&bound)?));
    check!(eq; legacy.input_token.run_manifest_digest, direct.input_token.run_manifest_digest);
    check!(eq; direct.codec, super::super::codec::MANIFEST_CODEC_ID);
    check!(eq; direct.codec_revision, super::super::codec::CODEC_REVISION);
    check!(eq; direct.input_token.protocol, super::super::codec::TOKEN_PROTOCOL);
    check!(eq; direct.streams.len(), super::super::stream::StreamKind::ALL.len());
    check!(eq; direct.source_fingerprints.len(), INPUT_TABLES.len());
    direct.validate()?;
    legacy.validate()?;
    check!(eq; store.snapshot().sequence(), snapshot.sequence());
    Ok(())
}

#[test]
fn snapshot_capture_refuses_foreign_origin_before_objects_or_row_decoding() -> TestResult {
    let first_directory = tempfile::tempdir()?;
    let second_directory = tempfile::tempdir()?;
    let first = Store::open(first_directory.path())?;
    let second = Store::open(second_directory.path())?;
    let identity = "a".repeat(64);
    first.journal_done("export-lineage-v1", "store-identity", &identity)?;
    second.journal_done("export-lineage-v1", "store-identity", &identity)?;
    first.append(Table::Schools, &school("Origin Fixture"))?;
    second.append(Table::Schools, &serde_json::json!({"id":"invalid-school"}))?;
    let snapshot = second.snapshot();
    let first_sequence = first.snapshot().sequence();
    check!(eq; first_sequence, snapshot.sequence());
    let directory = tempfile::tempdir()?;
    let result = capture_at(&first, &snapshot, "snapshot-seam:foreign", directory.path());
    check!(matches!(
        result,
        Err(ReportError::Store(
            StoreError::RevisionOriginMismatch { .. }
        ))
    ));
    check!(std::fs::read_dir(directory.path())?
        .next()
        .transpose()?
        .is_none());
    check!(eq; first.snapshot().sequence(), first_sequence);
    check!(eq; second.snapshot().sequence(), snapshot.sequence());
    Ok(())
}

#[test]
fn snapshot_capture_does_not_hydrate_identity_created_after_the_view() -> TestResult {
    let store_directory = tempfile::tempdir()?;
    let store = Store::open(store_directory.path())?;
    let snapshot = store.snapshot();
    store.store_identity()?;
    let sequence = store.snapshot().sequence();
    let directory = tempfile::tempdir()?;
    let result = capture_at(
        &store,
        &snapshot,
        "snapshot-seam:missing-identity",
        directory.path(),
    );
    check!(matches!(
        result,
        Err(ReportError::Store(StoreError::StoreLineage(
            StoreLineageError::Missing
        )))
    ));
    check!(std::fs::read_dir(directory.path())?
        .next()
        .transpose()?
        .is_none());
    check!(eq; store.snapshot().sequence(), sequence);
    check!(matches!(
        snapshot.store_identity(),
        Err(StoreError::StoreLineage(StoreLineageError::Missing))
    ));
    Ok(())
}

#[test]
fn snapshot_capture_propagates_existing_fold_budget_refusal() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let base = school("Oversized Snapshot School");
    for index in 0..12_u32 {
        let mut row = base.clone();
        row.aliases
            .push(format!("{index:02}{}", "x".repeat(100_000)));
        store.append(Table::Schools, &row)?;
    }
    store.store_identity()?;
    let snapshot = store.snapshot();
    let source = snapshot.tables_digest(&INPUT_TABLES)?;
    let directory = tempfile::tempdir()?;
    let result = capture_at(&store, &snapshot, "snapshot-seam:budget", directory.path());
    check!(matches!(
        result,
        Err(ReportError::Store(StoreError::BudgetExceeded {
            what: "canonical entity encoded fold",
            ..
        }))
    ));
    let current = store.snapshot();
    check!(eq; current.sequence(), snapshot.sequence());
    check!(eq; current.tables_digest(&INPUT_TABLES)?, source);
    Ok(())
}

#[test]
fn snapshot_capture_checks_job_bytes_before_cloning_or_writing_objects() -> TestResult {
    let store_directory = tempfile::tempdir()?;
    let store = Store::open(store_directory.path())?;
    store.store_identity()?;
    let snapshot = store.snapshot();
    let directory = tempfile::tempdir()?;
    let job = "x".repeat(4097);
    let result = capture_at(&store, &snapshot, &job, directory.path());
    check!(matches!(
        result,
        Err(ReportError::Resource(ExportResourceError::RowTooLarge {
            what: "manifest export job",
            bytes: 4097,
            max: 4096
        }))
    ));
    check!(std::fs::read_dir(directory.path())?
        .next()
        .transpose()?
        .is_none());
    check!(eq; store.snapshot().sequence(), snapshot.sequence());
    Ok(())
}
