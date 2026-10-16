use super::*;

fn write_all_tables(store: &Store, note: u32) -> TestResult {
    let mut batch = store.write_batch();
    for table in TABLES {
        batch.record_many(table, &[derived(table.file(), note)])?;
    }
    batch.commit()?;
    Ok(())
}

#[test]
fn pinned_six_table_rows_and_revisions_stay_paired_across_one_atomic_batch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    write_all_tables(&store, 1)?;
    let old = store.snapshot();
    let old_token = enabled_snapshot(&old)?;
    let old_root = old.tables_digest(&TABLES)?;
    write_all_tables(&store, 2)?;
    let new = store.snapshot();
    let new_token = enabled_snapshot(&new)?;
    for table in TABLES {
        check!(eq; old_token.table_revision(table)?, 1);
        check!(eq; new_token.table_revision(table)?, 2);
        check!(eq; serde_json::to_value(old.scan::<DerivedRow>(table)?)?, json!([{"id": table.file(), "note": 1}]));
        check!(eq; serde_json::to_value(new.scan::<DerivedRow>(table)?)?, json!([{"id": table.file(), "note": 2}]));
    }
    check!(eq; enabled_snapshot(&old)?, old_token);
    check!(eq; old.tables_digest(&TABLES)?, old_root);
    check!(ne; new.tables_digest(&TABLES)?, old_root);
    Ok(())
}

#[test]
fn pinned_snapshot_keeps_its_disabled_or_enabled_capability_and_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let disabled = store.snapshot();
    let baseline = enroll(&store)?;
    check!(eq; disabled.public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    let before = store.snapshot();
    mutate(&store, Table::Athletes, "later")?;
    check!(eq; before.public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Enabled(baseline));
    check!(eq; before.scan::<DerivedRow>(Table::Athletes)?.len(), 0);
    check!(eq; store.snapshot().scan::<DerivedRow>(Table::Athletes)?.len(), 1);
    assert_counters(&store, [1, 0, 0, 0, 0, 0])
}

#[test]
fn cloned_store_writers_share_one_revision_owner() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let first_store = store.clone();
    let second_store = store.clone();
    std::thread::scope(|scope| -> TestResult {
        let first = scope.spawn(move || mutate(&first_store, Table::Athletes, "first"));
        let second = scope.spawn(move || mutate(&second_store, Table::Athletes, "second"));
        let first_result = first.join().map_err(|_| "first writer panicked");
        let second_result = second.join().map_err(|_| "second writer panicked");
        first_result??;
        second_result??;
        Ok(())
    })?;
    assert_counters(&store, [2, 0, 0, 0, 0, 0])?;
    check!(eq; store.count(Table::Athletes)?, 2);
    drop(store);
    let reopened = Store::open_existing(directory.path())?;
    assert_counters(&reopened, [2, 0, 0, 0, 0, 0])
}

#[test]
fn revision_fenced_batch_updates_counters_and_stale_retry_writes_nothing() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let revision = store.snapshot().revision();
    let mut batch = store.write_batch();
    batch.append_many(Table::SourceObservations, &[derived("source", 1)])?;
    let (application, next) = batch
        .commit_once_at_revision("revision", "digest", revision)?
        .into_parts();
    check!(application.written());
    assert_counters(&store, [0, 1, 0, 0, 0, 0])?;
    check!(eq; next.evidence_generation(), store.evidence_generation());
    let before = retained_state(&store)?;
    let mut stale = store.write_batch();
    stale.append_many(Table::RelayResults, &[derived("relay", 1)])?;
    check!(matches!(
        stale.commit_once_at_revision("stale", "digest", revision),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
