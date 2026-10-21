use super::*;
use crate::ResultsCaptureContext;
use census_domain::model::{
    CanonicalEvent, CanonicalSchool, CanonicalTeam, ReviewCase, SourceObservation,
};
use census_store::{Entity, TableWalk};
use serde_json::Value;
use std::path::PathBuf;

const MEET_ID: i64 = 73003;

#[test]
fn options_refresh_bypasses_completed_meet_and_cached_documents() -> TestResult {
    check_refresh(false, true)
}

#[test]
fn context_refresh_bypasses_completed_meet_and_cached_documents() -> TestResult {
    check_refresh(true, false)
}

#[test]
fn both_refresh_flags_bypass_completed_meet_and_cached_documents() -> TestResult {
    check_refresh(true, true)
}

fn check_refresh(context_refresh: bool, options_refresh: bool) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for event_metadata in [false, true] {
                let directory = tempfile::tempdir()?;
                let capture =
                    crate::net::durable_capture_context(&directory.path().join("capture"))?;
                let store_path = directory.path().join("store");
                let cache = directory.path().join("http");
                seed_documents(&cache)?;
                let fetcher = Fetcher::new(
                    &cache,
                    None,
                    std::time::Duration::from_millis(1),
                    HashMap::new(),
                    Vec::new(),
                )?
                .with_offline(true);
                let store = Store::open(&store_path)?;
                seed_legacy_completions(&store)?;
                let options = Options {
                    meets: vec![MEET_ID],
                    event_metadata,
                    ..Options::default()
                };
                let first =
                    crate::athleticnet::collect(&context(&store, &fetcher, &capture)?, &options)
                        .await?;
                check!(eq; (first.rows, first.errors, first.requests, first.from_cache),
                    (1, 0, 0, 2 + u64::from(event_metadata)));
                let before = retained(&store)?;
                check!(eq; before.completion.len(), 5 + usize::from(event_metadata));
                let cache_before = cache_files(&cache)?;
                assert_replay_and_refresh(
                    &store,
                    &fetcher,
                    &options,
                    context_refresh,
                    options_refresh,
                    &capture,
                )
                .await?;
                check!(eq; retained(&store)?, before);
                check!(eq; cache_files(&cache)?, cache_before);
                drop(store);
                let reopened = Store::open(&store_path)?;
                check!(eq; retained(&reopened)?, before);
                assert_replay_and_refresh(
                    &reopened,
                    &fetcher,
                    &options,
                    context_refresh,
                    options_refresh,
                    &capture,
                )
                .await?;
                check!(eq; retained(&reopened)?, before);
                check!(eq; cache_files(&cache)?, cache_before);
            }
            Ok(())
        })
}

async fn assert_replay_and_refresh(
    store: &Store,
    fetcher: &Fetcher,
    options: &Options,
    context_refresh: bool,
    options_refresh: bool,
    capture: &ResultsCaptureContext,
) -> TestResult {
    let mut ctx = context(store, fetcher, capture)?;
    let replay = crate::athleticnet::collect(&ctx, options).await?;
    check!(eq; (replay.rows, replay.errors, replay.requests, replay.from_cache), (0, 0, 0, 0));
    ctx.refresh = context_refresh;
    let refreshed = Options {
        refresh: options_refresh,
        ..options.clone()
    };
    let report = crate::athleticnet::collect(&ctx, &refreshed).await?;
    check!(eq; (report.rows, report.errors, report.requests, report.from_cache), (0, 1, 0, 0));
    let [meet_url, _] = meet_requests(MEET_ID);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains(&meet_url) && note.contains("offline and not cached")));
    Ok(())
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<crate::AdapterContext<'a>> {
    Ok(crate::AdapterContext {
        capture: crate::CaptureContext::DurableV1(capture),
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2030).ok_or("decision school year")?,
        observed_on: "2030-09-30".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2030, 9, 30).ok_or("decision cutoff")?,
        recording: None,
    })
}

fn seed_documents(cache: &Path) -> TestResult {
    let meet = serde_json::json!({
        "sport2":"tfo", "meet":{"ID":MEET_ID,"Name":"Synthetic refresh meet",
        "MeetDate":"2026-05-15", "Location":{"State":"WI","Name":"Synthetic field"}}
    });
    let results = serde_json::json!({
        "teams":[{"IDSchool":73001,"SchoolName":"Synthetic Capture High"}],
        "relayLegs":[],"flatEvents":[{"EventId":1,"Event":"100 Meters",
        "EventShort":"100m","Gender":"M","Division":"Varsity","results":[{
            "IDResult":MEET_ID,"AthleteID":73002,"FirstName":"Synthetic",
            "LastName":"Runner","Grade":"11","TeamID":73001,
            "SchoolName":"Synthetic Capture High","Result":"11.25","Place":"1"
        }]}]
    });
    let metadata = serde_json::json!({"events":[{"ID":1,"Type":"T","isHurdle":false}]});
    let [meet_url, results_url] = meet_requests(MEET_ID);
    seed_cache(cache, &meet_url, &meet.to_string(), None, MEET_CAPTURED_AT)?;
    seed_cache(
        cache,
        &results_url,
        &results.to_string(),
        None,
        RESULTS_CAPTURED_AT,
    )?;
    seed_cache(
        cache,
        &crate::athleticnet::metadata_request(MEET_ID),
        &metadata.to_string(),
        None,
        RESULTS_CAPTURED_AT,
    )
}

fn seed_legacy_completions(store: &Store) -> TestResult {
    let [meet, results] = meet_requests(MEET_ID);
    for url in [meet, results, crate::athleticnet::metadata_request(MEET_ID)] {
        store.journal_done(
            "athleticnet",
            &url,
            &serde_json::json!({
                "url":url,"parser":2,"parsed":true,"rows":7
            }),
        )?;
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
struct Retained {
    evidence_generation: u64,
    tables: Vec<TableWalk>,
    observations: Vec<Vec<u8>>,
    completion: Vec<(String, String, Value)>,
}

fn retained(store: &Store) -> TestResult<Retained> {
    let mut completion = Vec::new();
    for phase in ["athleticnet", crate::athleticnet::MEET_COMPLETION_PHASE] {
        let mut keys: Vec<_> = store.journal_keys(phase)?.into_iter().collect();
        keys.sort();
        for key in keys {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("completion payload")?;
            completion.push((phase.to_string(), key, payload));
        }
    }
    Ok(Retained {
        evidence_generation: store.evidence_generation(),
        tables: Table::ALL
            .into_iter()
            .map(|table| store.walk_table(table))
            .collect::<Result<_, _>>()?,
        observations: vec![
            raw::<CanonicalSchool>(store, Table::Schools)?,
            raw::<CanonicalTeam>(store, Table::Teams)?,
            raw::<CanonicalAthlete>(store, Table::Athletes)?,
            raw::<CanonicalMeet>(store, Table::Meets)?,
            raw::<CanonicalEvent>(store, Table::Events)?,
            raw::<CanonicalPerformance>(store, Table::Performances)?,
            raw::<SourceObservation>(store, Table::SourceObservations)?,
            raw::<ReviewCase>(store, Table::ReviewCases)?,
        ],
        completion,
    })
}

fn raw<T: Entity>(store: &Store, table: Table) -> TestResult<Vec<u8>> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(table, |row: T| {
        rows.push(row);
        Ok(())
    })?;
    Ok(serde_json::to_vec(&rows)?)
}

fn cache_files(root: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeMap::new();
    let mut visited = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            visited += 1;
            check!(visited <= 128, "synthetic cache must stay bounded");
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else {
                check!(
                    kind.is_file(),
                    "synthetic cache must contain only files and directories"
                );
                files.insert(
                    entry.path().strip_prefix(root)?.to_path_buf(),
                    std::fs::read(entry.path())?,
                );
            }
        }
    }
    Ok(files)
}
