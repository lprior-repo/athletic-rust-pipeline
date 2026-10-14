use super::*;

#[test]
fn new_keys_below_and_above_old_members_do_not_change_the_retained_view() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/middle", 1)?;
    let captured = store.snapshot();
    let before = roots(&captured)?;
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, "row/after", &json!({"revision": 2}))?;
    batch.journal_done(PHASE, "row/z-last", &json!({"revision": 2}))?;
    batch.commit()?;
    let after = roots(&captured)?;
    check!(before
        .fingerprint()
        .same_contents(&store, after.fingerprint())?);
    check!(eq; after.logical_roots(), 1);
    let current = store.snapshot();
    check!(eq; current.evidence_generation(), captured.evidence_generation());
    let changed = roots(&current)?;
    check!(eq; changed.logical_roots(), 3);
    check!(!before
        .fingerprint()
        .same_contents(&store, changed.fingerprint())?);
    Ok(())
}

#[test]
fn callback_payload_reads_and_membership_share_one_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    put(&store, PHASE, "row/b", 1)?;
    let captured = store.snapshot();
    let mut seen = Vec::new();
    let inventory = captured.for_each_journal_root_bounded(namespace()?, budget(2), |key| {
        if key == "row/a" {
            let mut batch = store.write_batch();
            batch.journal_done(PHASE, "row/b", &json!({"revision": 2}))?;
            batch.journal_done(PHASE, "row/c", &json!({"revision": 2}))?;
            batch.commit()?;
        }
        seen.push((key.to_string(), captured.journal_payload(PHASE, key)?));
        Ok(())
    })?;
    check!(eq; inventory.logical_roots(), 2);
    check!(eq; seen, vec![
        ("row/a".to_string(), Some(json!({"revision": 1}))),
        ("row/b".to_string(), Some(json!({"revision": 1}))),
    ]);
    check!(eq; roots(&store.snapshot())?.logical_roots(), 3);
    Ok(())
}

#[test]
fn exact_phase_and_prefix_exclude_neighbors_and_operational_journals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    let before = roots(&store.snapshot())?;
    put(&store, "membership_fixture_v10", "row/a", 2)?;
    put(&store, PHASE, "rows/a", 2)?;
    put(&store, PHASE, "operational/retry", 2)?;
    let after = roots(&store.snapshot())?;
    check!(before
        .fingerprint()
        .same_contents(&store, after.fingerprint())?);
    check!(eq; after.logical_roots(), 1);
    Ok(())
}

#[test]
fn replacements_and_removals_change_the_physical_fingerprint() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    let first = roots(&store.snapshot())?;
    put(&store, PHASE, "row/a", 2)?;
    let second = roots(&store.snapshot())?;
    check!(eq; first.fingerprint().physical_rows(), second.fingerprint().physical_rows());
    check!(!first
        .fingerprint()
        .same_contents(&store, second.fingerprint())?);
    store.journal.remove(Store::journal_key(PHASE, "row/a"))?;
    let removed = roots(&store.snapshot())?;
    check!(eq; removed.logical_roots(), 0);
    check!(!second
        .fingerprint()
        .same_contents(&store, removed.fingerprint())?);
    Ok(())
}

#[test]
fn raw_final_comparison_needs_no_logical_decoder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    let before = roots(&store.snapshot())?;
    let current = store.snapshot();
    let raw_fingerprint = current.journal_namespace_fingerprint(namespace()?, budget(1))?;
    check!(before
        .fingerprint()
        .same_contents(&store, &raw_fingerprint)?);
    raw(&store, "row/a", br#"{"type":"unknown-future-kind"}"#)?;
    let changed = store
        .snapshot()
        .journal_namespace_fingerprint(namespace()?, budget(1))?;
    check!(!before.fingerprint().same_contents(&store, &changed)?);
    descriptor(
        roots(&store.snapshot()),
        "row/a",
        "root is neither an inline entry nor a chunked-manifest",
    )?;
    Ok(())
}

#[test]
fn foreign_origin_refuses_even_identical_empty_contents() -> TestResult {
    let left_dir = tempfile::tempdir()?;
    let right_dir = tempfile::tempdir()?;
    let left = Store::open(left_dir.path())?;
    let right = Store::open(right_dir.path())?;
    let left_fingerprint = left
        .snapshot()
        .journal_namespace_fingerprint(namespace()?, budget(0))?;
    let right_fingerprint = right
        .snapshot()
        .journal_namespace_fingerprint(namespace()?, budget(0))?;
    check!(matches!(
        left_fingerprint.same_contents(&left, &right_fingerprint),
        Err(StoreError::RevisionOriginMismatch { operation })
            if operation == "compare journal namespace membership"
    ));
    check!(matches!(
        left_fingerprint.same_contents(&right, &right_fingerprint),
        Err(StoreError::RevisionOriginMismatch { operation })
            if operation == "compare journal namespace membership"
    ));
    let clone = left.clone();
    left_fingerprint.ensure_origin(&clone)?;
    Ok(())
}
