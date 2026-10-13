use super::fixtures::{assert_moved, WITNESS_PHASE};
use super::*;

#[test]
fn strict_current_repeat_returns_the_accepted_unchanged_token() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot().revision();
    let (first, committed) = effects(&store, "strict-repeat", "Repeated High", 1)?
        .commit_once_at_revision("strict-repeat", "repeat-digest", initial)?
        .into_parts();
    let retained = RetainedState::capture(&store)?;
    let (repeated, accepted) = effects(&store, "strict-repeat", "Repeated High", 1)?
        .commit_once_at_revision("strict-repeat", "repeat-digest", committed)?
        .into_parts();
    check!(matches!(repeated, Application::Repeated(_)));
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; repeated.appended(), 0);
    check!(eq; pair(accepted), pair(committed));
    retained.assert_unchanged(&store)?;
    let (next, next_revision) = effects(&store, "repeat-next", "Next Independent High", 2)?
        .commit_once_at_revision("repeat-next", "next-digest", accepted)?
        .into_parts();
    check!(next.written());
    check!(eq; pair(next_revision), pair(store.snapshot().revision()));
    let retained = RetainedState::capture(&store)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    let current = reopened.snapshot().revision();
    let (repeated, accepted) = effects(&reopened, "strict-repeat", "Repeated High", 1)?
        .commit_once_at_revision("strict-repeat", "repeat-digest", current)?
        .into_parts();
    check!(eq; repeated.receipt(), first.receipt());
    check!(repeated.repeated());
    check!(eq; pair(accepted), pair(current));
    retained.assert_unchanged(&reopened)
}

#[test]
fn stale_matching_historical_receipt_is_refused_before_new_effects() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot().revision();
    let (first, historical) = effects(&store, "historical-repeat", "Historical High", 1)?
        .commit_once_at_revision("historical-repeat", "historical-digest", initial)?
        .into_parts();
    let external_store = store.clone();
    let mut external = external_store.write_batch();
    external.append_many(Table::Schools, &[school("External Writer High")])?;
    external.commit()?;
    drop(external_store);
    let actual = pair(store.snapshot().revision());
    let retained = RetainedState::capture(&store)?;
    let error =
        refused(
            effects(&store, "historical-repeat", "Never Replayed High", 9)?
                .commit_once_at_revision("historical-repeat", "historical-digest", historical),
        )?;
    assert_moved(error, "historical-repeat", pair(historical), actual)?;
    retained.assert_unchanged(&store)?;
    check!(eq; store.receipt("historical-repeat")?, Some(first.receipt().clone()));
    check!(eq; store.journal_payload(WITNESS_PHASE, "historical-repeat")?, Some(serde_json::json!({"school":"Historical High","note":1})));
    check!(!store
        .scan::<census_domain::model::CanonicalSchool>(Table::Schools)?
        .iter()
        .any(|row| row.id == school("Never Replayed High").id));
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    check!(eq; reopened.receipt("historical-repeat")?, Some(first.receipt().clone()));
    Ok(())
}

#[test]
fn legacy_repeats_preserve_their_repeat_before_fence_semantics() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot().revision();
    let stale_evidence = initial.evidence_generation();
    let first = effects(&store, "legacy-repeat", "Legacy High", 1)?
        .commit_once("legacy-repeat", "legacy-digest")?;
    store.append(Table::Schools, &school("Legacy External High"))?;
    check!(ne; stale_evidence, store.evidence_generation());
    let retained = RetainedState::capture(&store)?;
    let unfenced = effects(&store, "legacy-repeat", "Never Legacy High", 2)?
        .commit_once("legacy-repeat", "legacy-digest")?;
    check!(unfenced.repeated());
    check!(eq; unfenced.receipt(), first.receipt());
    let fenced = effects(&store, "legacy-repeat", "Never Legacy High", 2)?
        .commit_once_at_evidence_generation("legacy-repeat", "legacy-digest", stale_evidence)?;
    check!(fenced.repeated());
    check!(eq; fenced.receipt(), first.receipt());
    retained.assert_unchanged(&store)?;
    let error = refused(
        effects(&store, "legacy-repeat", "Never Legacy High", 2)?.commit_once_at_revision(
            "legacy-repeat",
            "legacy-digest",
            initial,
        ),
    )?;
    assert_moved(
        error,
        "legacy-repeat",
        pair(initial),
        pair(store.snapshot().revision()),
    )?;
    retained.assert_unchanged(&store)?;
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    check!(eq; reopened.receipt("legacy-repeat")?, Some(first.receipt().clone()));
    Ok(())
}
