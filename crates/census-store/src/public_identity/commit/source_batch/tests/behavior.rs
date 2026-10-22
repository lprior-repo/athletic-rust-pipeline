use super::*;

#[test]
fn cohort_commits_two_ordinary_facts_with_one_capture_and_exact_replay() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    append_pair(&store)?;
    let outside = in_year(&subject("cohort-outside", "90103")?, year_2028()?)?;
    store.append(Table::Athletes, &outside)?;
    let original = store.snapshot().revision();
    let initial = writer.head(&store)?;
    let generation = store.evidence_generation();
    let outcomes = writer.establish_source_bound_cohort(
        &store,
        "cohort-two",
        GradYear::CO2027,
        original,
        &initial,
    )?;
    check!(eq; outcomes.len(), 2);
    check!(outcomes.iter().all(|outcome| !outcome.replayed()));
    check!(eq; writer.head(&store)?.sequence, 2);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 2);
    check!(eq; store.evidence_generation(), generation.checked_add(2).ok_or("generation")?);
    check!(eq; store.receipt_count()?, 3);
    let raw = store
        .journal
        .get("manual-context-admission-v1\0header")?
        .ok_or("admission header")?;
    let header: serde_json::Value = serde_json::from_slice(&raw)?;
    check!(eq; header.get("reservations").and_then(serde_json::Value::as_u64), Some(1));
    let request = Request::new(writer.store_identity(), "cohort-two", GradYear::CO2027)?;
    check!(eq; completion(&store, &request)?.appended, 2);
    let ledger = writer.verified(&store)?;
    check!(matches!(
        ledger
            .history
            .projection()
            .resolve_candidate(&outside.id.cast())?,
        PublicResolution::Unknown
    ));
    audit_records(&store, &writer, ledger.history.records())?;
    let before = retained_state(&store)?;
    let replayed = writer.establish_source_bound_cohort(
        &store,
        "cohort-two",
        GradYear::CO2027,
        original,
        &initial,
    )?;
    check!(replayed.iter().all(IdentityCommitOutcomeV1::replayed));
    check!(eq; public_ids(&replayed), public_ids(&outcomes));
    check!(eq; replayed.iter().map(IdentityCommitOutcomeV1::head).collect::<Vec<_>>(), outcomes.iter().map(IdentityCommitOutcomeV1::head).collect::<Vec<_>>());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn cohort_replays_after_cold_reopen_and_refuses_changed_cohort() -> TestResult {
    let (directory, store, writer) = fixture()?;
    append_pair(&store)?;
    let initial = writer.head(&store)?;
    let outcomes = commit(&writer, &store, "cohort-cold")?;
    let ids = public_ids(&outcomes);
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let before = retained_state(&store)?;
    let repeated = writer.establish_source_bound_cohort(
        &store,
        "cohort-cold",
        GradYear::CO2027,
        store.snapshot().revision(),
        &initial,
    )?;
    check!(repeated.iter().all(IdentityCommitOutcomeV1::replayed));
    check!(eq; public_ids(&repeated), ids);
    let error = refused(writer.establish_source_bound_cohort(
        &store,
        "cohort-cold",
        year_2028()?,
        store.snapshot().revision(),
        &initial,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::OperationConflict)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn provider_competition_outside_cohort_preserves_full_graph_ambiguity() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("cohort-competing", "90201")?;
    let competitor = in_year(&first, year_2028()?)?;
    let independent = subject("cohort-independent", "90202")?;
    store.append_many(
        Table::Athletes,
        &[first.clone(), competitor.clone(), independent.clone()],
    )?;
    let outcomes = commit(&writer, &store, "cohort-competing")?;
    check!(eq; outcomes.len(), 1);
    let ledger = writer.verified(&store)?;
    for athlete in [&first, &competitor] {
        check!(matches!(
            ledger
                .history
                .projection()
                .resolve_candidate(&athlete.id.cast())?,
            PublicResolution::Unknown
        ));
    }
    check!(!matches!(
        ledger
            .history
            .projection()
            .resolve_candidate(&independent.id.cast())?,
        PublicResolution::Unknown
    ));
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 1);
    Ok(())
}

#[test]
fn cohort_skips_existing_bindings_through_existing_projection_rules() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("cohort-existing", "90301")?;
    let second = subject("cohort-new", "90302")?;
    store.append_many(Table::Athletes, &[first.clone(), second])?;
    let prior = writer.establish_source_bound(
        &store,
        "single-existing",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let generation = store.evidence_generation();
    let outcomes = commit(&writer, &store, "cohort-skip")?;
    check!(eq; outcomes.len(), 1);
    let outcome = outcomes.first().ok_or("cohort outcome")?;
    check!(ne; outcome.public_id(), prior.public_id());
    check!(eq; outcome.head().sequence, 2);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 2);
    check!(eq; store.evidence_generation(), generation.checked_add(1).ok_or("generation")?);
    Ok(())
}
