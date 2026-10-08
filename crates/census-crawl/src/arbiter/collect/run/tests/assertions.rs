use super::*;

pub(super) fn assert_retained_facts(
    store: &Store,
    expected_coaches: &BTreeSet<String>,
) -> TestResult {
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.len(), 1);
    let retained = schools.first().ok_or("retained school")?;
    check!(eq; retained.name, "Recovery High School");
    check!(eq; retained.state, Some(UsJurisdiction::NewHampshire));
    check!(eq; retained.enrollment, Some(955));
    check!(eq;
        retained.source_identities.first().map(|identity| identity.id.as_str()),
        Some("450")
    );
    check!(eq;
        retained.evidence.first().and_then(|e| e.source.url.as_deref()),
        Some(BASE)
    );
    let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations)?;
    check!(eq; observations.len(), 1);
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    check!(eq; coaches.len(), expected_coaches.len());
    check!(eq;
        coaches.into_iter().map(|row| row.name).collect::<BTreeSet<_>>(),
        *expected_coaches
    );
    check!(eq; store.journal_keys(JOURNAL)?, std::collections::HashSet::new());
    Ok(())
}

pub(super) async fn recovers_same_school_once(first: FirstPage) -> TestResult {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let expected = seed_failure(&cache, &first)?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = fetcher(&cache)?;
    let ctx = context(&fetcher, &store, None)?;
    let options = options();
    let row = school();
    let mut failed = run(&ctx, &options)?;
    failed
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await?;
    check!(eq; store.journal_keys(JOURNAL)?.len(), 0);
    assert_retained_facts(&store, &expected)?;
    let marker = pending(&store, &row)?;
    check!(eq; marker["owner_key"], json!("NH:2132:450"));
    check!(eq; marker["kind"], json!("partial"));
    seed(
        &cache,
        1,
        &json!({"data": {"total": 2, "rows": [coach("Ada", "Lane"), coach("Beau", "Pine")]}})
            .to_string(),
    )?;
    assert_recovered_once(&ctx, &options, &row).await
}

pub(super) async fn assert_recovered_once(
    ctx: &AdapterContext<'_>,
    options: &Options,
    row: &OrgSchool,
) -> TestResult {
    let mut recovered = run(ctx, options)?;
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await?;
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await?;
    check!(eq;
        ctx.store.journal_keys(JOURNAL)?,
        std::collections::HashSet::from(["NH:2132:450".to_string()])
    );
    let payloads = ctx.store.journal_payloads(JOURNAL)?;
    check!(eq; payloads.len(), 1);
    check!(eq; payloads.first().map(|payload| payload["coach_rows"].clone()), Some(json!(3)));
    let coaches: Vec<CanonicalCoach> = ctx.store.scan(Table::Coaches)?;
    check!(eq;
        coaches.iter().map(|row| row.name.as_str()).collect::<BTreeSet<_>>(),
        BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
    );
    let before = ctx.store.walk_table(Table::Coaches)?;
    let mut replayed = run(ctx, options)?;
    replayed
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await?;
    check!(eq; (replayed.tally.schools, replayed.tally.skipped), (0, 1));
    check!(eq; ctx.store.walk_table(Table::Coaches)?, before);
    Ok(())
}

pub(super) async fn assert_live_parity(
    recorded_ctx: &AdapterContext<'_>,
    live: &Store,
    options: &Options,
    row: &OrgSchool,
    host: &str,
) -> TestResult {
    let ctx = context(recorded_ctx.fetcher, live, None)?;
    run(&ctx, options)?
        .process_school_at(UsJurisdiction::NewHampshire, "2132", row, (BASE, host))
        .await?;
    for table in [Table::Schools, Table::SourceObservations, Table::Coaches] {
        check!(eq; recorded_ctx.store.walk_table(table)?, live.walk_table(table)?);
    }
    let key = super::super::completion_key(UsJurisdiction::NewHampshire, "2132", row.public_id)
        .ok_or("public owner")?;
    for phase in [JOURNAL.to_string(), super::super::recovery::phase(&key)] {
        check!(eq; recorded_ctx.store.journal_payloads(&phase)?, live.journal_payloads(&phase)?);
    }
    Ok(())
}
