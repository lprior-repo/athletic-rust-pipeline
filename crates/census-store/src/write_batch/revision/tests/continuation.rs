use super::fixtures::WITNESS_PHASE;
use super::*;

#[test]
fn own_written_revision_continues_independent_batches_and_reopens_exactly() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot().revision();
    let initial_pair = pair(initial);
    let (first, first_revision) = effects(&store, "own-first", "First High", 1)?
        .commit_once_at_revision("own-first", "first-digest", initial)?
        .into_parts();
    check!(first.written());
    check!(eq; first.appended(), 1);
    check!(eq; pair(first_revision), pair(store.snapshot().revision()));
    check!(eq; first_revision.evidence_generation(), initial_pair.0.checked_add(1).ok_or("test evidence overflow")?);
    check!(eq; first_revision.derived_generation(), initial_pair.1);
    let (second, second_revision) = effects(&store, "own-second", "Second High", 2)?
        .commit_once_at_revision("own-second", "second-digest", first_revision)?
        .into_parts();
    check!(second.written());
    check!(eq; pair(second_revision), pair(store.snapshot().revision()));
    check!(eq; second_revision.evidence_generation(), first_revision.evidence_generation().checked_add(1).ok_or("test evidence overflow")?);
    check!(eq; second_revision.derived_generation(), first_revision.derived_generation());
    check!(eq; pair(initial), initial_pair);
    check!(eq; store.receipt_count()?, 2);
    check!(eq; store.scan::<census_domain::model::CanonicalSchool>(Table::Schools)?.len(), 2);
    check!(eq; store.scan::<DerivedRow>(Table::ReviewCases)?.len(), 2);
    check!(eq; store.journal_payload(WITNESS_PHASE, "own-first")?, Some(serde_json::json!({"school":"First High","note":1})));
    check!(eq; store.journal_payload(WITNESS_PHASE, "own-second")?, Some(serde_json::json!({"school":"Second High","note":2})));
    let retained = RetainedState::capture(&store)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    check!(eq; reopened.receipt("own-first")?, Some(first.receipt().clone()));
    check!(eq; reopened.receipt("own-second")?, Some(second.receipt().clone()));
    Ok(())
}

#[test]
fn journal_only_and_receipt_only_writes_return_the_unchanged_pair() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot().revision();
    let mut batch = store.write_batch();
    batch.journal_done(WITNESS_PHASE, "journal-only", &"retained")?;
    let (journal, unchanged) = batch
        .commit_once_at_revision("journal-only", "journal-digest", initial)?
        .into_parts();
    check!(journal.written());
    check!(eq; journal.appended(), 0);
    check!(eq; pair(unchanged), pair(initial));
    check!(eq; pair(unchanged), pair(store.snapshot().revision()));
    let (empty, unchanged_again) = store
        .write_batch()
        .commit_once_at_revision("receipt-only", "receipt-digest", unchanged)?
        .into_parts();
    check!(empty.written());
    check!(eq; empty.appended(), 0);
    check!(eq; pair(unchanged_again), pair(initial));
    let (entity, next) = effects(&store, "after-journal", "After Journal High", 1)?
        .commit_once_at_revision("after-journal", "entity-digest", unchanged_again)?
        .into_parts();
    check!(entity.written());
    check!(eq; pair(next), pair(store.snapshot().revision()));
    check!(eq; store.receipt_count()?, 3);
    let retained = RetainedState::capture(&store)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    check!(eq; reopened.journal_payload(WITNESS_PHASE, "journal-only")?, Some(serde_json::json!("retained")));
    Ok(())
}
