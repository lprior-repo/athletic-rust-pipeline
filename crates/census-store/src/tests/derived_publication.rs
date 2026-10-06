use super::*;

fn publish(
    store: &Store,
    rows: &[DerivedRow],
    operation: &str,
    digest: &str,
) -> TestResult<Publication> {
    let mut stage = store.stage_derived()?;
    stage.replace_many(Table::ReviewCases, rows)?;
    Ok(stage.publish(operation, digest)?)
}

fn cases(count: u32) -> Vec<DerivedRow> {
    (0..count)
        .map(|index| derived(&format!("case-{index}"), 1))
        .collect()
}

#[test]
fn a_staged_generation_is_invisible_until_the_pointer_moves() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    publish(&store, &cases(5), "publish:index", "digest-one")?;
    check!(eq; store.derived_generation(), 1);
    check!(eq; store.scan::<DerivedRow>(Table::ReviewCases)?.len(), 5);
    let mut pending = store.stage_derived()?;
    pending.replace_many(Table::ReviewCases, &[derived("case-0", 2)])?;
    check!(eq; store.derived_generation(), 1, "staging must not move the pointer");
    check!(
        eq;
        store.scan::<DerivedRow>(Table::ReviewCases)?.len(),
        5,
        "a staged generation must stay out of reads until it is published"
    );
    pending.publish("publish:next", "digest-two")?;
    check!(eq; store.derived_generation(), 2);
    check!(eq; generation_rows(&store, Table::ReviewCases, 2)?, 1);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        0,
        "publication reclaims the generation it supersedes"
    );
    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases)?;
    check!(eq; scanned.len(), 1);
    check!(eq; scanned[0].note, 2);
    Ok(())
}

#[test]
fn a_repeated_pass_publishes_nothing_and_leaves_its_generation_for_reclaim() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows = cases(3);
    publish(&store, &rows, "publish:index", "digest-one")?;
    let repeated = publish(&store, &rows, "publish:index", "digest-one")?;
    check!(
        matches!(repeated.application, Application::Repeated(_)),
        "the same operation and digest must replay its receipt rather than stage anything"
    );
    check!(eq; store.derived_generation(), 1, "a repeat must not move the pointer");
    check!(eq; store.receipt_count()?, 1, "a repeat must not record a second receipt");
    let reclaimed = store.reclaim_derived_generations(u64::MAX)?;
    check!(reclaimed.complete);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 2)?,
        0,
        "the abandoned staged generation must be reclaimed"
    );
    check!(eq; generation_rows(&store, Table::ReviewCases, 1)?, 3);
    Ok(())
}

#[test]
fn a_publication_refuses_when_evidence_moved_while_the_generation_was_staged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut pending = store.stage_derived()?;
    pending.replace_many(Table::ReviewCases, &cases(2))?;
    store.append_many(Table::Schools, &[school("Abbotsford")])?;
    let outcome = pending.publish("publish:index", "digest-one");
    check!(
        matches!(outcome, Err(StoreError::PublicationRefused { .. })),
        "a generation staged against moved inputs must be refused: {outcome:?}"
    );
    check!(eq; store.derived_generation(), 0, "a refused publication must not move the pointer");
    check!(
        eq;
        store.scan::<CanonicalSchool>(Table::Schools)?.len(),
        1,
        "the appearance that moved the inputs must survive the refusal"
    );
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        2,
        "the refused generation stays staged and unpublished"
    );
    let reclaimed = store.reclaim_derived_generations(u64::MAX)?;
    check!(reclaimed.complete);
    check!(eq; generation_rows(&store, Table::ReviewCases, 1)?, 0);
    Ok(())
}
