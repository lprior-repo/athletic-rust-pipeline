use super::*;

pub(super) fn assert_retained_facts(store: &Store, expected_coaches: &BTreeSet<String>) {
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("school facts");
    assert_eq!(schools.len(), 1);
    let retained = schools.first().expect("retained school");
    assert_eq!(retained.name, "Recovery High School");
    assert_eq!(retained.state, Some(UsJurisdiction::NewHampshire));
    assert_eq!(retained.enrollment, Some(955));
    assert_eq!(
        retained
            .source_identities
            .first()
            .map(|identity| identity.id.as_str()),
        Some("450")
    );
    assert_eq!(
        retained
            .evidence
            .first()
            .and_then(|e| e.source.url.as_deref()),
        Some(BASE)
    );
    let observations: Vec<SourceObservation> = store
        .scan(Table::SourceObservations)
        .expect("source observations");
    assert_eq!(observations.len(), 1);
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches).expect("coach facts");
    assert_eq!(coaches.len(), expected_coaches.len());
    assert_eq!(
        coaches
            .into_iter()
            .map(|row| row.name)
            .collect::<BTreeSet<_>>(),
        *expected_coaches
    );
    assert_eq!(
        store.journal_keys(JOURNAL).expect("journal keys"),
        std::collections::HashSet::new()
    );
}

pub(super) async fn recovers_same_school_once(first: FirstPage) {
    let dir = tempfile::tempdir().expect("temporary directory");
    let cache = dir.path().join("http");
    let expected = seed_failure(&cache, &first);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&cache);
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let row = school();
    let mut failed = run(&ctx, &options);
    failed
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await
        .expect("retained incomplete facts");
    assert_eq!((failed.tally.schools, failed.tally.errors), (1, 1));
    assert_eq!(failed.done.len(), 0);
    assert_retained_facts(&store, &expected);
    let marker = pending(&store, &row);
    assert_eq!(marker["version"], json!(2));
    assert_eq!(marker["owner_key"], json!("NH:2132:450"));
    assert_eq!(marker["kind"], json!("partial"));
    let responses = marker["recovery"]["responses"]
        .as_array()
        .expect("incomplete responses");
    assert_eq!(
        responses.len(),
        match first {
            FirstPage::Missing => 0,
            FirstPage::LaterMalformed | FirstPage::Bounded => 2,
            _ => 1,
        }
    );
    seed(
        &cache,
        1,
        &json!({"data": {"total": 2, "rows": [coach("Ada", "Lane"), coach("Beau", "Pine")]}})
            .to_string(),
    );
    assert_recovered_once(&ctx, &options, &row).await;
}

pub(super) async fn assert_recovered_once(
    ctx: &AdapterContext<'_>,
    options: &Options,
    row: &OrgSchool,
) {
    let mut recovered = run(ctx, options);
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await
        .expect("recovered school");
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await
        .expect("duplicate school");
    assert_eq!(
        (
            recovered.tally.schools,
            recovered.tally.coaches,
            recovered.tally.errors,
            recovered.tally.skipped
        ),
        (1, 3, 0, 1)
    );
    assert_eq!(
        ctx.store.journal_keys(JOURNAL).expect("completion keys"),
        std::collections::HashSet::from(["NH:2132:450".to_string()])
    );
    let payloads = ctx
        .store
        .journal_payloads(JOURNAL)
        .expect("completion payloads");
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads
            .first()
            .map(|payload| payload["coach_rows"].clone()),
        Some(json!(3))
    );
    let coaches: Vec<CanonicalCoach> = ctx.store.scan(Table::Coaches).expect("recovered facts");
    assert_eq!(
        coaches
            .iter()
            .map(|row| row.name.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
    );
    let before = ctx.store.walk_table(Table::Coaches).expect("coach count");
    let mut replayed = run(ctx, options);
    replayed
        .process_school(UsJurisdiction::NewHampshire, "2132", row, BASE)
        .await
        .expect("completed retry");
    assert_eq!((replayed.tally.schools, replayed.tally.skipped), (0, 1));
    assert_eq!(
        ctx.store
            .walk_table(Table::Coaches)
            .expect("unchanged facts"),
        before
    );
}
