use super::*;

fn publish(store: &Store, operation: &str, rows: &[DerivedRow]) -> TestResult {
    let mut stage = store.stage_derived()?;
    stage_required_tables(&mut stage, &[Table::ReviewCases])?;
    stage.replace_many(Table::ReviewCases, rows)?;
    stage.publish(operation, operation)?;
    Ok(())
}

#[test]
fn pinned_review_cases_and_revision_survive_publication_and_reclaim_together() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    publish(&store, "old", &[derived("old-case", 1)])?;
    let old = store.snapshot();
    let old_token = enabled_snapshot(&old)?;
    let old_root = old.tables_digest(&[Table::ReviewCases])?;
    publish(&store, "new", &[derived("new-case", 2)])?;
    let new = store.snapshot();
    let new_token = enabled_snapshot(&new)?;
    store.reclaim_derived_generations(100)?;
    check!(eq; crate::tests::generation_rows(&store, Table::ReviewCases, old.derived_generation())?, 0);
    check!(eq; serde_json::to_value(old.scan::<DerivedRow>(Table::ReviewCases)?)?, json!([{"id": "old-case", "note": 1}]));
    check!(eq; serde_json::to_value(new.scan::<DerivedRow>(Table::ReviewCases)?)?, json!([{"id": "new-case", "note": 2}]));
    check!(eq; old_token.table_revision(Table::ReviewCases)?, 1);
    check!(eq; new_token.table_revision(Table::ReviewCases)?, 2);
    check!(eq; enabled_snapshot(&old)?, old_token);
    check!(eq; enabled_snapshot(&new)?, new_token);
    check!(eq; enabled(&store)?, new_token);
    check!(eq; old.tables_digest(&[Table::ReviewCases])?, old_root);
    check!(ne; new.tables_digest(&[Table::ReviewCases])?, old_root);
    Ok(())
}

#[test]
fn empty_and_carry_forward_publications_each_advance_review_cases_once() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    publish(&store, "empty", &[])?;
    assert_counters(&store, [0, 0, 1, 0, 0, 0])?;
    let mut empty_carried = store.stage_derived()?;
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.generation_partitioned())
    {
        empty_carried.carry_forward(table)?;
    }
    empty_carried.publish("empty-carried", "empty-carried")?;
    assert_counters(&store, [0, 0, 2, 0, 0, 0])?;
    publish(&store, "populated", &[derived("case", 1)])?;
    assert_counters(&store, [0, 0, 3, 0, 0, 0])?;
    let mut stage = store.stage_derived()?;
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.generation_partitioned())
    {
        stage.carry_forward(table)?;
    }
    stage.publish("carried", "carried")?;
    assert_counters(&store, [0, 0, 4, 0, 0, 0])?;
    check!(eq; store.count(Table::ReviewCases)?, 1);
    Ok(())
}

#[test]
fn abandoned_staging_repeated_publication_and_reclaim_do_not_advance_inputs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let before = enabled(&store)?;
    let mut abandoned = store.stage_derived()?;
    stage_required_tables(&mut abandoned, &[Table::ReviewCases])?;
    abandoned.replace_many(Table::ReviewCases, &[derived("abandoned", 1)])?;
    check!(eq; enabled(&store)?, before);
    drop(abandoned);
    store.reclaim_derived_generations(100)?;
    check!(eq; enabled(&store)?, before);
    publish(&store, "first", &[derived("current", 1)])?;
    let before = enabled(&store)?;
    publish(&store, "first", &[derived("current", 1)])?;
    store.reclaim_derived_generations(100)?;
    check!(eq; enabled(&store)?, before);
    check!(eq; store.count(Table::ReviewCases)?, 1);
    Ok(())
}

#[test]
fn refused_stale_or_incomplete_publication_does_not_advance_review_revision() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let mut stale = store.stage_derived()?;
    stage_required_tables(&mut stale, &[])?;
    mutate(&store, Table::Athletes, "moved")?;
    let before = enabled(&store)?;
    check!(matches!(
        stale.publish("stale", "stale"),
        Err(StoreError::PublicationRefused { .. })
    ));
    check!(eq; enabled(&store)?, before);
    let mut incomplete = store.stage_derived()?;
    incomplete.replace_many(Table::ReviewCases, &[derived("case", 1)])?;
    check!(matches!(
        incomplete.publish("incomplete", "incomplete"),
        Err(StoreError::PublicationRefused { .. })
    ));
    check!(eq; enabled(&store)?, before);
    check!(eq; store.derived_generation(), 0);
    check!(store.receipt("stale")?.is_none());
    check!(store.receipt("incomplete")?.is_none());
    Ok(())
}
