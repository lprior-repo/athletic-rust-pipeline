use super::*;
use crate::athleticnet::{meet_requests, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{Fetcher, RepresentationHeaders};
use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalPerformance};
use census_store::{Store, Table};
use std::collections::HashSet;
use std::path::Path;

mod capture_time;
mod refresh;

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
    capture_time::retained_observations(store, RESULTS_CAPTURED_AT)?;
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
                capture: crate::CaptureContext::LegacyMetadata,
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

#[test]
fn old_meet_completion_is_retained_while_the_current_parser_interprets_cached_xc_labels(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            let [meet_url, results_url] = meet_requests(634313);
            let meet: MeetData = serde_json::from_str(MEET_DATA)?;
            let results = serde_json::json!({
                "teams":[{"IDSchool":3204,"SchoolName":"Middleton"}],
                "relayLegs":[],
                "flatEvents":[{"EventId":1,"Event":"3000m XC","EventShort":"3000m XC",
                    "Gender":"M","Division":"Varsity","results":[{
                        "IDResult":1,"AthleteID":28872883,"FirstName":"Captured",
                        "LastName":"Runner","Grade":"11","TeamID":3204,
                        "SchoolName":"Middleton","Result":"10:00.00","Place":"1"
                    }]}]
            }).to_string();
            let metadata_url = crate::athleticnet::metadata_request(634313);
            let metadata = serde_json::json!({"events":[{"ID":1,"Type":"T","isHurdle":false}]}).to_string();
            seed_cache(&cache, &meet_url, MEET_DATA, None, MEET_CAPTURED_AT)?;
            seed_cache(&cache, &results_url, &results, meet.token.as_deref(), RESULTS_CAPTURED_AT)?;
            seed_cache(&cache, &metadata_url, &metadata, meet.token.as_deref(), RESULTS_CAPTURED_AT)?;
            let old_meet = serde_json::json!({"url":meet_url,"parser":2,"parsed":true,"rows":0});
            let old_results = serde_json::json!({"url":results_url,"parser":2,"parsed":true,"rows":1});
            let old_metadata = serde_json::json!({"url":metadata_url,"parser":2,"parsed":true,"rows":0});
            store.journal_done("athleticnet", &meet_url, &old_meet)?;
            store.journal_done("athleticnet", &results_url, &old_results)?;
            store.journal_done("athleticnet", &metadata_url, &old_metadata)?;
            let fetcher = Fetcher::new(&cache, None, std::time::Duration::from_millis(1),
                HashMap::new(), Vec::new())?.with_offline(true);
            let ctx = crate::AdapterContext {
                capture: crate::CaptureContext::LegacyMetadata,
                fetcher:&fetcher, store:&store, refresh:false,
                school_year:SchoolYear::new(2026).ok_or("controlled school year")?,
                observed_on:"2026-09-30".into(),
                performance_as_of:chrono::NaiveDate::from_ymd_opt(2026,9,30).ok_or("controlled cutoff")?,
                recording:None,
            };
            let options = Options { meets:vec![634313], event_metadata:true, ..Options::default() };
            check!(eq; crate::athleticnet::PARSE_VERSION, 3);
            check!(eq; crate::athleticnet::MEET_COMPLETION_PHASE, "athleticnet_meet_completions_v3");
            check!(eq; crate::athleticnet::collect::journaled_urls(&ctx)?, HashSet::new());
            let first = crate::athleticnet::collect(&ctx, &options).await?;
            check!(eq; (first.rows,first.errors,first.requests,first.from_cache), (1,0,0,3));
            let events:Vec<census_domain::model::CanonicalEvent> = store.snapshot().scan(Table::Events)?;
            check!(eq; events.len(), 1);
            let event = events.first().ok_or("XC event")?;
            check!(eq; event.kind, census_domain::model::EventKind::CrossCountry);
            let performances:Vec<CanonicalPerformance> = store.snapshot().scan(Table::Performances)?;
            check!(eq; performances.len(), 1);
            let performance = performances.first().ok_or("captured performance")?;
            check!(eq; performance.mark, Mark::TimeSeconds(census_domain::model::ExactSeconds::parse("600")?));
            check!(eq; performance.evidence.first().ok_or("physical evidence")?.observed_on, RESULTS_CAPTURED_AT);
            for url in [&meet_url,&results_url,&metadata_url] {
                let current = store.journal_payload(crate::athleticnet::MEET_COMPLETION_PHASE, url)?
                    .ok_or("current completion")?;
                check!(eq; current.get("parser"), Some(&serde_json::json!(3)));
                check!(eq; current.get("parsed"), Some(&serde_json::json!(true)));
            }
            check!(eq; store.journal_payload("athleticnet", &meet_url)?, Some(old_meet));
            check!(eq; store.journal_payload("athleticnet", &results_url)?, Some(old_results));
            check!(eq; store.journal_payload("athleticnet", &metadata_url)?, Some(old_metadata));
            let replay = crate::athleticnet::collect(&ctx, &options).await?;
            check!(eq; (replay.rows,replay.requests,replay.from_cache), (0,0,0));
            check!(eq; store.walk_table(Table::Events)?.rows, 1);
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            Ok(())
        })
}
