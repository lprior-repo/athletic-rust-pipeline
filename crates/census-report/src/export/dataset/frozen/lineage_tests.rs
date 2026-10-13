use census_store::{Store, StoreError, StoreLineageError, Table};

use super::super::ExportDataset;
use crate::report::ReportError;

#[path = "../../../../../../tools/lineage_test_process.rs"]
mod process;
mod support;

use process::bounded_case;
use support::{fixture_dir, TestResult};

#[test]
fn initialized_fenced_snapshot_derives_without_reacquiring_append_lock() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::initialized_fenced_snapshot_derives_without_reacquiring_append_lock"
        ),
        || {
            let directory = fixture_dir()?;
            let store = Store::open_with(directory.path(), 8_388_608)?;
            let identity = store.store_identity()?;
            let fence = store.fenced_snapshot();
            let dataset = ExportDataset::from_snapshot(&store, fence.view())?;
            check!(eq; dataset.lineage.store_identity, identity.as_str());
            check!(eq; dataset.lineage.snapshot_sequence, fence.view().sequence());
            check!(eq; dataset.schools.len(), 0);
            Ok(())
        },
    )
}

#[test]
fn uninitialized_fenced_snapshot_refuses_without_creating_live_identity() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::uninitialized_fenced_snapshot_refuses_without_creating_live_identity"
        ),
        || {
            let directory = fixture_dir()?;
            let store = Store::open_with(directory.path(), 8_388_608)?;
            let fence = store.fenced_snapshot();
            let sequence = fence.view().sequence();
            let result = ExportDataset::from_snapshot(&store, fence.view());
            check!(matches!(
                result,
                Err(ReportError::Store(StoreError::StoreLineage(
                    StoreLineageError::Missing
                )))
            ));
            check!(eq; fence.view().sequence(), sequence);
            drop(fence);
            check!(eq; store.snapshot().sequence(), sequence);
            check!(!store.journal_contains("export-lineage-v1", "store-identity")?);
            Ok(())
        },
    )
}

#[test]
fn foreign_snapshot_refuses_before_decoding_rows_even_with_identical_identity() -> TestResult {
    let first_directory = fixture_dir()?;
    let second_directory = fixture_dir()?;
    let first = Store::open_with(first_directory.path(), 8_388_608)?;
    let second = Store::open_with(second_directory.path(), 8_388_608)?;
    let identity = "a".repeat(64);
    first.journal_done("export-lineage-v1", "store-identity", &identity)?;
    second.journal_done("export-lineage-v1", "store-identity", &identity)?;
    second.append(Table::Schools, &serde_json::json!({"id":"invalid-school"}))?;
    let view = second.snapshot();
    let result = ExportDataset::from_snapshot(&first, &view);
    check!(matches!(
        result,
        Err(ReportError::Store(
            StoreError::RevisionOriginMismatch { .. }
        ))
    ));
    let same_origin = ExportDataset::from_snapshot(&second, &view);
    check!(matches!(
        same_origin,
        Err(ReportError::Store(StoreError::Decode { .. }))
    ));
    Ok(())
}

#[test]
fn old_same_origin_snapshot_keeps_old_rows_after_new_evidence() -> TestResult {
    let directory = fixture_dir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    let identity = store.store_identity()?;
    let view = store.snapshot();
    let school = super::fixtures::school("Newly Observed School");
    store.append(Table::Schools, &school)?;
    let old = ExportDataset::from_snapshot(&store.clone(), &view)?;
    check!(eq; old.schools.len(), 0);
    check!(eq; old.lineage.store_identity, identity.as_str());
    check!(eq; old.lineage.snapshot_sequence, view.sequence());
    let current = ExportDataset::load(&store)?;
    check!(eq; current.schools.len(), 1);
    check!(current.schools.contains_key(&school.id));
    check!(ne; current.lineage.source_digest, old.lineage.source_digest);
    Ok(())
}

#[test]
fn moved_store_adopts_identity_and_retains_frozen_archive_bytes() -> TestResult {
    let directory = fixture_dir()?;
    let original = directory.path().join("original");
    let moved = directory.path().join("moved");
    let retained;
    let bytes;
    {
        let store = Store::open_with(&original, 8_388_608)?;
        store.append(Table::Schools, &super::fixtures::school("Retained School"))?;
        let dataset = ExportDataset::load(&store)?;
        retained = dataset.lineage.clone();
        let archive = store.out_dir().join("retained.json");
        dataset.save_frozen(&archive)?;
        bytes = std::fs::read(&archive)?;
    }
    std::fs::rename(&original, &moved)?;
    let store = Store::open_with(&moved, 8_388_608)?;
    check!(eq; store.store_identity()?.as_str(), retained.store_identity.as_str());
    let archive = store.out_dir().join("retained.json");
    let reopened = ExportDataset::reopen_frozen(&archive)?;
    check!(eq; reopened.lineage, retained);
    check!(eq; reopened.schools.len(), 1);
    reopened.ensure_store(&store)?;
    reopened.ensure_current(&store)?;
    check!(eq; std::fs::read(&archive)?, bytes);
    let foreign = Store::open_with(directory.path().join("foreign"), 8_388_608)?;
    foreign.append(Table::Schools, &super::fixtures::school("Retained School"))?;
    let foreign_dataset = ExportDataset::load(&foreign)?;
    check!(eq; foreign_dataset.lineage.source_digest, retained.source_digest);
    check!(reopened.ensure_store(&foreign).is_err());
    Ok(())
}

#[test]
fn uppercase_legacy_archive_stays_exact_without_lowercase_v2_substitution() -> TestResult {
    let directory = fixture_dir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    let uppercase = "ABCDEF0123456789".repeat(4);
    store.journal_done("export-lineage-v1", "store-identity", &uppercase)?;
    let before = store.snapshot().sequence();
    let dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.lineage.store_identity, uppercase);
    let archive = store.out_dir().join("uppercase.json");
    dataset.save_frozen(&archive)?;
    let bytes = std::fs::read(&archive)?;
    let reopened = ExportDataset::reopen_frozen(&archive)?;
    check!(eq; reopened.lineage.store_identity, uppercase);
    check!(eq; store.snapshot().sequence(), before);
    let refusal = super::freeze_v2(&store, "uppercase-refusal");
    check!(matches!(refusal, Err(ReportError::Invariant { detail })
        if detail.contains("input token store identity")
            && detail.contains("not a lowercase sha256 digest")));
    check!(eq; store.store_identity()?.as_str(), uppercase.as_str());
    check!(eq; std::fs::read(&archive)?, bytes);
    check!(census_store::publish::Digest::parse(&uppercase).is_err());
    Ok(())
}
