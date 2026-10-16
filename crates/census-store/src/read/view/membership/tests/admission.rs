use super::*;

#[test]
fn parent_admission_precedes_malformed_root_validation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    raw(&store, "row/a", b"not json")?;
    let mut state = (0, 0);
    let result = store.snapshot().for_each_journal_root_with_admission(
        namespace()?,
        budget(1),
        &mut state,
        |state| {
            state.0 += 1;
            Err(StoreError::CounterOverflow)
        },
        |state, _| {
            state.1 += 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; state, (1, 0));
    Ok(())
}

#[test]
fn parent_admission_counts_chunks_and_surplus_before_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 1, 4)?)?;
    store.journal.insert(
        chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
        chunk_value("row/a", 0, 1, b"null")?,
    )?;
    store
        .journal
        .insert(chunk_key(&Store::journal_key(PHASE, "row/a"), 1)?, b"extra")?;
    let mut state = (0, Vec::new());
    let result = store.snapshot().for_each_journal_root_with_admission(
        namespace()?,
        budget(3),
        &mut state,
        |state| {
            state.0 += 1;
            Ok(())
        },
        |state, key| {
            state.1.push((state.0, key.to_owned()));
            Ok(())
        },
    );
    descriptor(result, "row/", "chunk is orphaned, missing, or surplus")?;
    check!(eq; state.0, 3);
    check!(eq; state.1, vec![(1, "row/a".to_owned())]);
    Ok(())
}

#[test]
fn parent_admission_counts_row_sentinel_and_preserves_complete_fingerprint() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    put(&store, PHASE, "row/a", 1)?;
    put(&store, PHASE, "row/b", 2)?;
    let view = store.snapshot();
    let mut state = (0, 0);
    let result = view.for_each_journal_root_with_admission(
        namespace()?,
        budget(1),
        &mut state,
        |state| {
            state.0 += 1;
            Ok(())
        },
        |state, _| {
            state.1 += 1;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 2,
            available: 1
        })
    ));
    check!(eq; state, (2, 1));
    let expected = roots(&view)?;
    let actual = view.for_each_journal_root_with_admission(
        namespace()?,
        budget(2),
        &mut (),
        |_| Ok(()),
        |_, _| Ok(()),
    )?;
    check!(expected
        .fingerprint()
        .same_contents(&store, actual.fingerprint())?);
    check!(eq; expected.logical_roots(), actual.logical_roots());
    Ok(())
}
