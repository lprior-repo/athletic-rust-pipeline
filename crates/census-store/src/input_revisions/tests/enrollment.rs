use super::*;

#[test]
fn absent_capability_stays_disabled_across_writes_reads_and_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for table in TABLES {
        mutate(&store, table, table.file())?;
    }
    let before = retained_state(&store)?;
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    check!(eq; retained_state(&store)?, before);
    drop(store);
    let reopened = Store::open_existing(directory.path())?;
    check!(eq; reopened.snapshot().public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    check!(eq; retained_state(&reopened)?, before);
    Ok(())
}

#[test]
fn explicit_enrollment_preserves_old_rows_and_is_once_only() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    mutate(&store, Table::Athletes, "existing")?;
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    let revision = store.snapshot().revision();
    let token = enroll(&store)?;
    check!(eq; token.baseline_evidence_generation, revision.evidence_generation());
    check!(eq; token.baseline_derived_generation, revision.derived_generation());
    check!(eq; without_capability(retained_state(&store)?), before);
    check!(eq; store.snapshot().sequence(), sequence.checked_add(1).ok_or("sequence overflow")?);
    assert_counters(&store, [0; 6])?;
    let enrolled = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    check!(eq; enroll(&store)?, token);
    check!(eq; retained_state(&store)?, enrolled);
    check!(eq; store.snapshot().sequence(), sequence);
    drop(store);
    let reopened = Store::open_existing(directory.path())?;
    check!(eq; enabled(&reopened)?, token);
    check!(eq; retained_state(&reopened)?, enrolled);
    Ok(())
}

#[test]
fn enrollment_refuses_stale_and_foreign_revisions_without_writes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let foreign_directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let foreign = Store::open(foreign_directory.path())?;
    let old = store.snapshot().revision();
    mutate(&store, Table::Athletes, "changed")?;
    let before = retained_state(&store)?;
    check!(matches!(
        store.enroll_public_identity_input_revisions_v2(old),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(matches!(
        store.enroll_public_identity_input_revisions_v2(foreign.snapshot().revision()),
        Err(StoreError::RevisionOriginMismatch { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn qualification_requires_its_current_writer_and_keeps_generic_open_refused() -> TestResult {
    let (directory, store, writer) = crate::public_identity_tests::support::unenrolled_fixture()?;
    let before = retained_state(&store)?;
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    check!(store
        .enroll_public_identity_input_revisions_v2(store.snapshot().revision())
        .is_err());
    check!(eq; retained_state(&store)?, before);
    let token =
        writer.enroll_public_identity_input_revisions_v2(&store, store.snapshot().revision())?;
    check!(eq; token, enabled(&store)?);
    check!(eq; without_capability(retained_state(&store)?), before);
    drop(store);
    let root = directory.path().join("identity");
    check!(Store::open_existing(&root).is_err());
    check!(Store::migrate(&root).is_err());
    let (store, _) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; enabled(&store)?, token);
    Ok(())
}

#[test]
fn qualification_enrollment_refuses_foreign_and_historical_writers() -> TestResult {
    let (_directory, store, writer) = crate::public_identity_tests::support::unenrolled_fixture()?;
    let (_other_directory, other, _) = crate::public_identity_tests::support::unenrolled_fixture()?;
    let before = retained_state(&other)?;
    check!(writer
        .enroll_public_identity_input_revisions_v2(&other, other.snapshot().revision())
        .is_err());
    check!(eq; retained_state(&other)?, before);
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    check!(Store::create_public_identity_qualification_v1(&root).is_err());
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(!root.exists());
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    Ok(())
}

#[test]
fn checked_getter_rejects_every_non_dependency_table() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let token = enroll(&store)?;
    for table in Table::ALL {
        check!(eq; token.table_revision(table).is_ok(), TABLES.contains(&table));
    }
    Ok(())
}
