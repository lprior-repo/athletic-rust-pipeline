use super::*;
use crate::athleticnet::{meet_requests, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{Fetcher, RepresentationHeaders};
use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalPerformance};
use census_store::{Store, Table};
use std::path::Path;

const MEET_CAPTURED_AT: &str = "2026-09-22T11:59:58Z";
const RESULTS_CAPTURED_AT: &str = "2026-09-22T12:00:00Z";

fn seed_cache(
    cache: &Path,
    url: &str,
    body: &str,
    token: Option<&str>,
    fetched_at: &str,
) -> TestResult {
    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
    if let Some(token) = token {
        headers.push(("anettokens".to_string(), token.to_string()));
    }
    let representation = RepresentationHeaders::canonical(&headers)?;
    let key = Fetcher::key_for("GET", url, &representation.identity());
    let meta = CacheMeta {
        redirects: Vec::new(),
        url: url.to_string(),
        method: "GET".to_string(),
        representation,
        status: 200,
        content_digest: content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        ..CacheMeta::default()
    };
    std::fs::create_dir_all(cache)?;
    write_cache(
        &cache.join(format!("{key}.body")),
        &cache.join(format!("{key}.meta.json")),
        body.as_bytes(),
        &meta,
    )?;
    Ok(())
}

fn retained_evidence(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let meets: Vec<CanonicalMeet> = snapshot.scan(Table::Meets)?;
    check!(eq; meets.len(), 1);
    let meet = meets.first().ok_or("captured meet")?;
    check!(eq; meet.evidence.first().ok_or("meet evidence")?.observed_on, MEET_CAPTURED_AT);
    let performances: Vec<CanonicalPerformance> = snapshot.scan(Table::Performances)?;
    check!(eq; performances.len(), 974);
    for performance in &performances {
        let evidence = performance.evidence.first().ok_or("performance evidence")?;
        check!(eq; evidence.source.id, "athleticnet");
        check!(eq; evidence.observed_on, RESULTS_CAPTURED_AT);
    }
    let athletes: Vec<CanonicalAthlete> = snapshot.scan(Table::Athletes)?;
    let school_year = SchoolYear::new(2025).ok_or("published school year")?;
    for athlete in &athletes {
        check!(eq; athlete.evidence.first().ok_or("athlete evidence")?.observed_on,
            RESULTS_CAPTURED_AT);
        let grade = athlete.observed_grades.first().ok_or("published grade")?;
        check!(eq; grade.school_year, school_year);
    }
    Ok(())
}

#[test]
fn cached_meet_keeps_capture_instants_and_published_school_year_on_later_replay() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            let [meet_url, results_url] = meet_requests(634313);
            let meet: MeetData = serde_json::from_str(MEET_DATA)?;
            seed_cache(&cache, &meet_url, MEET_DATA, None, MEET_CAPTURED_AT)?;
            seed_cache(
                &cache,
                &results_url,
                ALL_RESULTS,
                meet.token.as_deref(),
                RESULTS_CAPTURED_AT,
            )?;
            let fetcher = Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let mut ctx = crate::AdapterContext {
                fetcher: &fetcher,
                store: &store,
                refresh: false,
                school_year: SchoolYear::new(2030).ok_or("decision school year")?,
                observed_on: "2030-09-30".into(),
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2030, 9, 30)
                    .ok_or("decision date")?,
                recording: None,
            };
            let mut options = Options {
                meets: vec![634313],
                observed_on: "2030-09-30".into(),
                ..Options::default()
            };
            let first = crate::athleticnet::collect(&ctx, &options).await?;
            check!(eq; first.rows, 974);
            check!(eq; first.requests, 0);
            check!(eq; first.from_cache, 2);
            retained_evidence(&store)?;
            ctx.observed_on = "2031-09-30".into();
            options.observed_on = ctx.observed_on.clone();
            let replay = crate::athleticnet::collect(&ctx, &options).await?;
            check!(eq; replay.rows, 0);
            check!(eq; replay.requests, 0);
            check!(eq; replay.from_cache, 0);
            retained_evidence(&store)?;
            Ok(())
        })
}
