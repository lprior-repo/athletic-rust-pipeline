use super::fixtures::assert_moved;
use super::*;

#[test]
fn external_entity_mutation_refuses_an_own_write_continuation_without_effects() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (_, own_revision) = effects(&store, "entity-first", "Completed High", 1)?
        .commit_once_at_revision("entity-first", "first-digest", store.snapshot().revision())?
        .into_parts();
    let expected = pair(own_revision);
    let external = store.clone();
    external.append(Table::Schools, &school("Other Writer High"))?;
    let actual = pair(store.snapshot().revision());
    check!(ne; expected.0, actual.0);
    check!(eq; expected.1, actual.1);
    let retained = RetainedState::capture(&store)?;
    let error = refused(
        effects(&store, "entity-next", "Refused High", 2)?.commit_once_at_revision(
            "entity-next",
            "next-digest",
            own_revision,
        ),
    )?;
    assert_moved(error, "entity-next", expected, actual)?;
    check!(eq; pair(own_revision), expected);
    retained.assert_unchanged(&store)?;
    assert_absent(&store, "entity-next", "Refused High")?;
    drop(external);
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    assert_absent(&reopened, "entity-next", "Refused High")
}

#[test]
fn external_review_decision_refuses_a_stale_absent_case_replacement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = store.snapshot();
    check!(initial.scan::<DerivedRow>(Table::ReviewCases)?.is_empty());
    let stale = initial.revision();
    drop(initial);
    store.replace_many(Table::ReviewCases, &[derived("Decided High", 7)])?;
    let retained = RetainedState::capture(&store)?;
    let error = refused(
        effects(&store, "stale-review", "Decided High", 0)?.commit_once_at_revision(
            "stale-review",
            "review-digest",
            stale,
        ),
    )?;
    assert_moved(
        error,
        "stale-review",
        pair(stale),
        pair(store.snapshot().revision()),
    )?;
    retained.assert_unchanged(&store)?;
    assert_marker_absent(&store, "stale-review")?;
    check!(store
        .scan::<census_domain::model::CanonicalSchool>(Table::Schools)?
        .is_empty());
    check!(eq; serde_json::to_value(store.scan::<DerivedRow>(Table::ReviewCases)?)?, serde_json::to_value([derived("Decided High", 7)])?);
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    assert_marker_absent(&reopened, "stale-review")
}

#[test]
fn derived_publication_refuses_old_partition_writes_and_preserves_decided_cases() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (_, own_revision) = effects(&store, "publication-first", "Completed High", 1)?
        .commit_once_at_revision(
            "publication-first",
            "first-digest",
            store.snapshot().revision(),
        )?
        .into_parts();
    let expected = pair(own_revision);
    let mut publication = store.stage_derived()?;
    stage_required_tables(&mut publication, &[Table::ReviewCases])?;
    publication.replace_many(Table::ReviewCases, &[derived("Published Decision High", 7)])?;
    let published = publication.publish("derived-publication", "publication-digest")?;
    check!(published.application.written());
    let actual = pair(store.snapshot().revision());
    check!(ne; actual.0, expected.0);
    check!(ne; actual.1, expected.1);
    check!(eq; actual.1, published.generation);
    let retained = RetainedState::capture(&store)?;
    let error =
        refused(
            effects(&store, "publication-next", "Published Decision High", 0)?
                .commit_once_at_revision("publication-next", "next-digest", own_revision),
        )?;
    assert_moved(error, "publication-next", expected, actual)?;
    retained.assert_unchanged(&store)?;
    assert_marker_absent(&store, "publication-next")?;
    check!(eq; serde_json::to_value(store.scan::<DerivedRow>(Table::ReviewCases)?)?, serde_json::to_value([derived("Published Decision High", 7)])?);
    check!(!store
        .scan::<census_domain::model::CanonicalSchool>(Table::Schools)?
        .iter()
        .any(|row| row.id == school("Published Decision High").id));
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    assert_marker_absent(&reopened, "publication-next")
}
