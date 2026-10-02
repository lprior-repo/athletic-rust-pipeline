use super::*;

#[tokio::test]
async fn missing_coach_page_retains_school_and_later_completes_once() {
    recovers_same_school_once(FirstPage::Missing).await;
}

#[tokio::test]
async fn malformed_coach_page_retains_school_and_later_completes_once() {
    recovers_same_school_once(FirstPage::Malformed).await;
}

#[tokio::test]
async fn short_coach_page_retains_facts_and_later_completes_once() {
    recovers_same_school_once(FirstPage::Short).await;
}

#[tokio::test]
async fn empty_short_coach_page_is_not_completed_as_no_coaches() {
    recovers_same_school_once(FirstPage::EmptyShort).await;
}

#[tokio::test]
async fn later_malformed_coach_page_retains_earlier_facts_and_can_recover() {
    recovers_same_school_once(FirstPage::LaterMalformed).await;
}

#[tokio::test]
async fn bounded_coach_walk_retains_facts_without_completing_and_can_recover() {
    recovers_same_school_once(FirstPage::Bounded).await;
}

#[tokio::test]
async fn recording_incomplete_coach_page_cannot_emit_completion_receipt() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let cache = dir.path().join("http");
    seed_failure(&cache, &FirstPage::Short);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&cache);
    let recording = Recording::new();
    let ctx = context(&fetcher, &store, Some(&recording));
    let options = options();
    let row = school();
    let mut failed = run(&ctx, &options);
    failed
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await
        .expect("record incomplete facts");
    let recorded = recording.drain();
    assert_eq!(recorded.journal.len(), 1);
    let marker = recorded.journal.first().expect("incomplete marker");
    assert_eq!(marker.phase, super::super::recovery::phase("NH:2132:450"));
    assert_eq!(marker.payload["owner_key"], json!("NH:2132:450"));
    assert_eq!(
        store
            .walk_table(Table::Schools)
            .expect("unwritten recording")
            .rows,
        0
    );
    let coaches: Vec<_> = recorded
        .rows
        .iter()
        .filter(|batch| batch.table == Table::Coaches)
        .flat_map(|batch| batch.rows.iter())
        .map(|row| row["name"].clone())
        .collect();
    assert_eq!(coaches, vec![json!("Casey Reed"), json!("Ada Lane")]);
    let schools: Vec<_> = recorded
        .rows
        .iter()
        .filter(|batch| batch.table == Table::Schools)
        .flat_map(|batch| batch.rows.iter())
        .map(|row| row["name"].clone())
        .collect();
    assert_eq!(schools, vec![json!("Recovery High School")]);
    apply_recorded(&store, &recorded);
    assert_eq!(
        pending(&store, &row)["recovery"]["responses"][0]["content_digest"],
        json!(crate::net::cache::content_digest(
            &std::fs::read(cache.join(format!(
                "{}.body",
                Fetcher::key_for("GET", &coach_url(1), "")
            )))
            .expect("cached body")
        ))
    );
    seed(
        &cache,
        1,
        &json!({"data": {"total": 1, "rows": [coach("Ada", "Lane")]}}).to_string(),
    );
    let mut recovered = run(&ctx, &options);
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await
        .expect("record complete facts");
    recovered
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await
        .expect("record duplicate");
    let recorded = recording.drain();
    assert_eq!(recorded.journal.len(), 1);
    let receipt = recorded.journal.first().expect("complete receipt");
    assert_eq!(receipt.phase, JOURNAL);
    assert_eq!(receipt.payload["coach_rows"], json!(2));
    assert_eq!(recovered.tally.skipped, 1);
}

#[tokio::test]
async fn school_without_coach_lookup_id_keeps_facts_and_owes_completion() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = fetcher(&dir.path().join("http"));
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let mut row = school();
    row.public_id = None;
    let mut pending = run(&ctx, &options);
    pending
        .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
        .await
        .expect("retain school facts");
    assert_eq!(
        (
            pending.tally.schools,
            pending.tally.coaches,
            pending.tally.errors
        ),
        (1, 1, 1)
    );
    assert_eq!(
        store.journal_keys(JOURNAL).expect("completion keys").len(),
        0
    );
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("school facts");
    assert_eq!(
        schools.first().map(|row| row.name.as_str()),
        Some("Recovery High School")
    );
}

#[tokio::test]
async fn malformed_same_page_rows_retain_both_earlier_and_later_coaches_without_completion() {
    recovers_same_school_once(FirstPage::SamePageMalformed).await;
}

#[tokio::test]
async fn malformed_published_coach_fields_are_not_silently_folded_into_absence() {
    recovers_same_school_once(FirstPage::MalformedField).await;
}
