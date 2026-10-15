use super::*;

#[test]
fn direct_mutations_advance_only_their_dependency_once() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    for table in TABLES {
        let before = enabled(&store)?;
        let rows = [derived("first", 1), derived("second", 2)];
        match table.storage_mode() {
            StorageMode::ObservationLog => store.append_many(table, &rows)?,
            StorageMode::DerivedMap | StorageMode::DerivedGeneration => {
                store.replace_many(table, &rows)?
            }
        }
        let after = enabled(&store)?;
        for dependency in TABLES {
            let expected = if dependency == table {
                before
                    .table_revision(dependency)?
                    .checked_add(1)
                    .ok_or("counter overflow")?
            } else {
                before.table_revision(dependency)?
            };
            check!(eq; after.table_revision(dependency)?, expected);
        }
    }
    assert_counters(&store, [1; 6])
}

#[test]
fn multi_table_batch_and_same_table_pages_each_advance_once() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let mut batch = store.write_batch();
    for table in TABLES {
        batch.record_many(table, &[derived(table.file(), 1)])?;
    }
    batch.append_many(Table::Athletes, &[derived("second-athlete", 2)])?;
    batch.commit()?;
    assert_counters(&store, [1; 6])?;
    check!(eq; store.count(Table::Athletes)?, 2);
    Ok(())
}

#[test]
fn empty_journal_identity_and_unrelated_transactions_do_not_advance_inputs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let before = enroll(&store)?;
    store.write_batch().commit()?;
    store.journal_done("input-revision-test", "journal-only", &json!({"ok": true}))?;
    for table in Table::ALL
        .into_iter()
        .filter(|table| !TABLES.contains(table))
    {
        mutate(&store, table, table.file())?;
    }
    for table in TABLES {
        let mut batch = store.write_batch();
        batch.record_many::<DerivedRow>(table, &[])?;
        batch.commit()?;
    }
    check!(eq; enabled(&store)?, before);
    Ok(())
}

#[test]
fn repeated_outer_and_filtered_conditional_effects_do_not_advance_inputs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let mut first = store.write_batch();
    first.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    first.commit_once("effect", "digest")?;
    let before = enabled(&store)?;
    let mut repeated = store.write_batch();
    repeated.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    check!(matches!(
        repeated.commit_once("effect", "digest")?,
        Application::Repeated(_)
    ));
    let mut effect = store.write_batch();
    effect.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    let effect = effect.conditional("effect", "digest")?;
    store
        .write_batch()
        .commit_once_with_effects("outer", "outer-digest", vec![effect])?;
    check!(eq; enabled(&store)?, before);
    check!(eq; store.count(Table::Athletes)?, 1);
    Ok(())
}

#[test]
fn fresh_conditional_effects_on_one_table_advance_once_after_merging() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let mut base = store.write_batch();
    base.append_many(Table::Athletes, &[derived("base", 1)])?;
    let mut first = store.write_batch();
    first.append_many(Table::Athletes, &[derived("first", 2)])?;
    let mut second = store.write_batch();
    second.append_many(Table::Athletes, &[derived("second", 3)])?;
    second.append_many(Table::Performances, &[derived("performance", 4)])?;
    base.commit_once_with_effects(
        "outer",
        "digest",
        vec![
            first.conditional("first-effect", "first-digest")?,
            second.conditional("second-effect", "second-digest")?,
        ],
    )?;
    assert_counters(&store, [1, 0, 0, 0, 1, 0])?;
    check!(eq; store.count(Table::Athletes)?, 3);
    Ok(())
}

#[test]
fn public_identity_commit_and_replay_keep_all_input_revisions() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_public_identity_input_revisions_v2(&store, store.snapshot().revision())?;
    let athlete = subject("input-revision-identity", "501")?;
    store.append(Table::Athletes, &athlete)?;
    let before = enabled(&store)?;
    let revision = store.snapshot().revision();
    let head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "identity-only",
        &athlete.id.cast(),
        revision,
        &head,
    )?;
    check!(!committed.replayed());
    check!(eq; enabled(&store)?, before);
    let replay = writer.establish_source_bound(
        &store,
        "identity-only",
        &athlete.id.cast(),
        revision,
        &head,
    )?;
    check!(replay.replayed());
    check!(eq; enabled(&store)?, before);
    Ok(())
}
