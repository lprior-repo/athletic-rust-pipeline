use super::*;

#[test]
fn new_cohort_checks_stale_revision_and_head_before_capture() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    append_pair(&store)?;
    let original = store.snapshot().revision();
    let initial = writer.head(&store)?;
    commit(&writer, &store, "cohort-initial")?;
    let current = writer.head(&store)?;
    let before = retained_state(&store)?;
    let stale_revision = refused(writer.establish_source_bound_cohort(
        &store,
        "cohort-stale-revision",
        GradYear::CO2027,
        original,
        &current,
    ))?;
    check!(matches!(stale_revision, StoreError::RevisionMoved { .. }));
    let stale_head = refused(writer.establish_source_bound_cohort(
        &store,
        "cohort-stale-head",
        GradYear::CO2027,
        store.snapshot().revision(),
        &initial,
    ))?;
    check!(matches!(
        stale_head,
        StoreError::PublicIdentity(PublicIdentityError::HeadMoved)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn zero_accepted_sources_refuses_without_fact_or_completion_receipt() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("cohort-zero-first", "90401")?;
    let second = in_year(&first, year_2028()?)?;
    store.append_many(Table::Athletes, &[first, second])?;
    let initial = writer.head(&store)?;
    let generation = store.evidence_generation();
    let error = refused(commit(&writer, &store, "cohort-zero"))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
    ));
    check!(eq; writer.head(&store)?, initial);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 0);
    check!(eq; store.evidence_generation(), generation);
    let request = Request::new(writer.store_identity(), "cohort-zero", GradYear::CO2027)?;
    check!(store.receipt(&request.operation)?.is_none());
    let staged = retained_state(&store)?;
    check!(matches!(
        refused(commit(&writer, &store, "cohort-zero"))?,
        StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
    ));
    check!(eq; retained_state(&store)?, staged);
    Ok(())
}

#[test]
fn cohort_namespace_rejects_single_source_and_generic_receipt_mutation() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    append_pair(&store)?;
    commit(&writer, &store, "cohort-owned")?;
    let request = Request::new(writer.store_identity(), "cohort-owned", GradYear::CO2027)?;
    let candidate = subject("cohort-first", "90101")?.id.cast();
    let operation = request.child(&candidate)?;
    let before = retained_state(&store)?;
    check!(writer
        .establish_source_bound(
            &store,
            &operation,
            &candidate,
            store.snapshot().revision(),
            &writer.head(&store)?
        )
        .is_err());
    check!(store
        .credit_observations("cohort-credit", &request.operation)
        .is_err());
    check!(crate::receipt::refuse_over_operation(&request.operation, "changed").is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
