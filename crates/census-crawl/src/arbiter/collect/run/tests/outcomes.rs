use super::*;

#[test]
fn missing_coach_page_retains_school_and_later_completes_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::Missing).await })
}

#[test]
fn malformed_coach_page_retains_school_and_later_completes_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::Malformed).await })
}

#[test]
fn short_coach_page_retains_facts_and_later_completes_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::Short).await })
}

#[test]
fn empty_short_coach_page_is_not_completed_as_no_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::EmptyShort).await })
}

#[test]
fn later_malformed_coach_page_retains_earlier_facts_and_can_recover() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::LaterMalformed).await })
}

#[test]
fn bounded_coach_walk_retains_facts_without_completing_and_can_recover() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::Bounded).await })
}

#[test]
fn recording_incomplete_coach_page_cannot_emit_completion_receipt() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    seed_failure(&cache, &FirstPage::Short)?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = fetcher(&cache)?;
    let recording = Recording::new();
    let ctx = context(&fetcher, &store, Some(&recording))?;
    let options = options();
    let row = school();
    let mut failed = run(&ctx, &options)?;
    failed.process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE).await?;
    let recorded = recording.drain();
    check!(!recorded.journal.iter().any(|entry| entry.phase == JOURNAL));
    let phase = super::super::recovery::phase("NH:2132:450");
    let marker = recorded.journal.iter().find(|entry| entry.phase == phase).ok_or("incomplete marker")?;
    check!(eq; marker.payload["owner_key"], json!("NH:2132:450"));
    check!(eq; store.walk_table(Table::Schools)?.rows, 0);
    let coaches: Vec<_> = recorded.rows.iter()
        .filter(|batch| batch.table == Table::Coaches)
        .flat_map(|batch| batch.rows.iter())
        .map(|row| row["name"].clone())
        .collect();
    check!(eq; coaches, vec![json!("Casey Reed"), json!("Ada Lane")]);
    let schools: Vec<_> = recorded.rows.iter()
        .filter(|batch| batch.table == Table::Schools)
        .flat_map(|batch| batch.rows.iter())
        .map(|row| row["name"].clone())
        .collect();
    check!(eq; schools, vec![json!("Recovery High School")]);
    apply_recorded(&store, &recorded)?;
    check!(eq;
        pending(&store, &row)?["recovery"]["responses"][0]["content_digest"],
        json!(crate::net::cache::content_digest(
            &std::fs::read(cache.join(format!("{}.body", Fetcher::key_for("GET", &coach_url(1), ""))))?
        ))
    );
    seed(&cache, 1, &json!({"data": {"total": 1, "rows": [coach("Ada", "Lane")]}}).to_string())?;
    let mut recovered = run(&ctx, &options)?;
    recovered.process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE).await?;
    recovered.process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE).await?;
    let recorded = recording.drain();
    let receipt = recorded.journal.iter().find(|entry| entry.phase == JOURNAL).ok_or("complete receipt")?;
    check!(eq; receipt.payload["coach_rows"], json!(2));
    Ok(())
    })
}

#[test]
fn school_without_coach_lookup_id_keeps_facts_and_owes_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = fetcher(&dir.path().join("http"))?;
    let ctx = context(&fetcher, &store, None)?;
    let options = options();
    let mut row = school();
    row.public_id = None;
    let mut pending = run(&ctx, &options)?;
    pending.process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE).await?;
    check!(eq; (pending.tally.schools, pending.tally.coaches, pending.tally.errors), (1, 1, 1));
    check!(eq; store.journal_keys(JOURNAL)?.len(), 0);
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.first().map(|row| row.name.as_str()), Some("Recovery High School"));
    Ok(())
    })
}

#[test]
fn malformed_same_page_rows_retain_both_earlier_and_later_coaches_without_completion() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::SamePageMalformed).await })
}

#[test]
fn malformed_published_coach_fields_are_not_silently_folded_into_absence() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { recovers_same_school_once(FirstPage::MalformedField).await })
}
