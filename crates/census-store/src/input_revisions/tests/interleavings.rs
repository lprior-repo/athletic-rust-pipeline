use super::*;

fn pending_observation(store: &Store) -> TestResult<crate::StoreBatch<'_>> {
    let mut batch = store.write_batch();
    batch.append_many(Table::Athletes, &[derived("athlete", 1)])?;
    Ok(batch)
}

fn pending_publication(store: &Store) -> TestResult<crate::DerivedStage<'_>> {
    let mut stage = store.stage_derived()?;
    stage_required_tables(&mut stage, &[Table::ReviewCases])?;
    stage.replace_many(Table::ReviewCases, &[derived("case", 1)])?;
    Ok(stage)
}

#[test]
fn enrollment_before_a_prepared_write_counts_that_commit_from_the_new_baseline() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let disabled = store.snapshot();
    let revision = disabled.revision();
    let pending = pending_observation(&store)?;
    store.enroll_public_identity_input_revisions_v2(revision)?;
    pending.commit_once_at_revision("write", "digest", revision)?;
    assert_counters(&store, [1, 0, 0, 0, 0, 0])?;
    check!(eq; store.count(Table::Athletes)?, 1);
    check!(eq; disabled.public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    check!(disabled.scan::<DerivedRow>(Table::Athletes)?.is_empty());
    let before = retained_state(&store)?;
    check!(matches!(
        store.enroll_public_identity_input_revisions_v2(revision),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    check!(eq; enroll(&store)?, enabled(&store)?);
    Ok(())
}

#[test]
fn write_before_enrollment_refuses_the_old_fence_and_enrolls_only_a_new_baseline() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let revision = store.snapshot().revision();
    pending_observation(&store)?.commit_once_at_revision("write", "digest", revision)?;
    let before = retained_state(&store)?;
    check!(matches!(
        store.enroll_public_identity_input_revisions_v2(revision),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, PublicIdentityInputRevisionsV2::Disabled);
    let token = enroll(&store)?;
    check!(eq; token.baseline_evidence_generation, store.evidence_generation());
    assert_counters(&store, [0; 6])?;
    check!(eq; without_capability(retained_state(&store)?), before);
    check!(eq; store.count(Table::Athletes)?, 1);
    Ok(())
}

#[test]
fn publication_before_a_prepared_write_moves_the_write_fence_without_losing_input_counters(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let revision = store.snapshot().revision();
    let pending_write = pending_observation(&store)?;
    pending_publication(&store)?.publish("publication", "digest")?;
    let before = retained_state(&store)?;
    check!(matches!(
        pending_write.commit_once_at_revision("write", "digest", revision),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    assert_counters(&store, [0, 0, 1, 0, 0, 0])?;
    check!(store.receipt("write")?.is_none());
    pending_observation(&store)?.commit_once_at_revision(
        "write",
        "digest",
        store.snapshot().revision(),
    )?;
    assert_counters(&store, [1, 0, 1, 0, 0, 0])?;
    check!(eq; store.count(Table::Athletes)?, 1);
    check!(eq; store.count(Table::ReviewCases)?, 1);
    Ok(())
}

#[test]
fn write_before_publication_moves_the_publication_fence_and_leaves_review_inputs_unchanged(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    let stage = pending_publication(&store)?;
    let revision = store.snapshot().revision();
    pending_observation(&store)?.commit_once_at_revision("write", "digest", revision)?;
    let before = enabled(&store)?;
    check!(matches!(
        stage.publish("publication", "digest"),
        Err(StoreError::PublicationRefused { .. })
    ));
    check!(eq; enabled(&store)?, before);
    assert_counters(&store, [1, 0, 0, 0, 0, 0])?;
    check!(eq; store.derived_generation(), 0);
    check!(eq; store.count(Table::ReviewCases)?, 0);
    check!(store.receipt("publication")?.is_none());
    pending_publication(&store)?.publish("publication", "digest")?;
    assert_counters(&store, [1, 0, 1, 0, 0, 0])?;
    check!(eq; store.count(Table::Athletes)?, 1);
    check!(eq; store.count(Table::ReviewCases)?, 1);
    Ok(())
}
