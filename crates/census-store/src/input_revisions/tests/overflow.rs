use super::*;

#[test]
fn every_exhausted_dependency_refuses_its_entire_direct_transaction() -> TestResult {
    for table in TABLES {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        enroll(&store)?;
        stage_counters(&store, [u64::MAX; 6])?;
        let before = retained_state(&store)?;
        let sequence = store.snapshot().sequence();
        let evidence = store.evidence_generation();
        let row = derived("refused", 1);
        let result = match table.storage_mode() {
            StorageMode::ObservationLog => store.append(table, &row),
            StorageMode::DerivedMap | StorageMode::DerivedGeneration => store.replace(table, &row),
        };
        check!(matches!(result, Err(StoreError::CounterOverflow)));
        check!(eq; retained_state(&store)?, before);
        check!(eq; store.snapshot().sequence(), sequence);
        check!(eq; store.evidence_generation(), evidence);
    }
    Ok(())
}

#[test]
fn one_exhausted_counter_refuses_all_rows_journals_effects_and_receipts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    stage_counters(&store, [0, 0, 0, 0, u64::MAX, 0])?;
    let before = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    let mut base = store.write_batch();
    base.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    base.replace_many(Table::IdentityVerdicts, &[derived("verdict", 1)])?;
    base.journal_done("overflow", "journal", &json!({"staged": true}))?;
    let mut effect = store.write_batch();
    effect.append_many(Table::Performances, &[derived("performance", 1)])?;
    let outcome = base.commit_once_with_effects(
        "outer",
        "digest",
        vec![effect.conditional("effect", "digest")?],
    );
    check!(matches!(outcome, Err(StoreError::CounterOverflow)));
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.snapshot().sequence(), sequence);
    mutate(&store, Table::Athletes, "allowed")?;
    assert_counters(&store, [1, 0, 0, 0, u64::MAX, 0])?;
    Ok(())
}

#[test]
fn exhausted_review_counter_refuses_pointer_receipt_and_publication_journal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    stage_counters(&store, [0, 0, u64::MAX, 0, 0, 0])?;
    let evidence = store.evidence_generation();
    let derived = store.derived_generation();
    let mut stage = store.stage_derived()?;
    stage_required_tables(&mut stage, &[Table::ReviewCases])?;
    stage.replace_many(Table::ReviewCases, &[crate::tests::derived("staged", 1)])?;
    stage.journal_done_once("overflow-publication", "journal", &json!({"staged": true}))?;
    check!(matches!(
        stage.publish("publication", "digest"),
        Err(StoreError::CounterOverflow)
    ));
    assert_counters(&store, [0, 0, u64::MAX, 0, 0, 0])?;
    check!(eq; store.evidence_generation(), evidence);
    check!(eq; store.derived_generation(), derived);
    check!(eq; store.count(Table::ReviewCases)?, 0);
    check!(store.receipt("publication")?.is_none());
    check!(store
        .journal_payload("overflow-publication", "journal")?
        .is_none());
    Ok(())
}

#[test]
fn repeated_exhausted_effect_and_excluded_writes_do_not_consume_counters() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let mut first = store.write_batch();
    first.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    first.commit_once("effect", "digest")?;
    stage_counters(&store, [u64::MAX; 6])?;
    let mut effect = store.write_batch();
    effect.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    store.write_batch().commit_once_with_effects(
        "outer",
        "digest",
        vec![effect.conditional("effect", "digest")?],
    )?;
    mutate(&store, Table::Schools, "school")?;
    store.journal_done("overflow", "excluded", &json!({"ok": true}))?;
    assert_counters(&store, [u64::MAX; 6])
}
