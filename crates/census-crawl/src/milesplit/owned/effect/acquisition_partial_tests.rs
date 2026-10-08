use super::*;
use crate::net::cache::content_digest;
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::collections::{BTreeMap, HashMap};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn setup() -> TestResult<(tempfile::TempDir, Store, Fetcher, ResultSetRef)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("source reference")?;
    Ok((dir, store, fetcher, reference))
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("snapshot date")?,
        recording: None,
    })
}

fn partial_outcome(reference: &ResultSetRef) -> TestResult<OwnedMeetOutcome> {
    let mut document: serde_json::Value =
        serde_json::from_slice(include_bytes!("../fixtures/troy_725218.json"))?;
    document["data"][0]["athleteId"] = serde_json::Value::Null;
    let body = serde_json::to_vec(&document)?;
    Ok(OwnedMeetOutcome {
        verdict: parse_owned_meet(&body, 725218),
        capture: FetchOutcome {
            url: owned_meet_url(reference)?,
            response_url: None,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(&body),
            bytes: body.len(),
            body,
            fetched_at: "2026-10-01T23:44:16Z".into(),
            from_cache: false,
            content_type: Some("application/json".into()),
        },
    })
}

fn snapshot(store: &Store, phase: &str) -> TestResult<BTreeMap<String, serde_json::Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("retained entry")?;
            Ok((key, payload))
        })
        .collect()
}

#[test]
fn partial_persistence_network_metadata_to_cache_replay_is_an_exact_noop() -> TestResult {
    let (_dir, store, fetcher, reference) = setup()?;
    let mut outcome = partial_outcome(&reference)?;
    record_outcome(&context(&store, &fetcher)?, &reference, &outcome)?;
    let originals = snapshot(&store, OWNED_MEET_PHASE)?;
    let captures = snapshot(&store, OWNED_CAPTURE_PHASE)?;
    let summary = originals
        .values()
        .find(|row| row["disposition"] == "partial")
        .ok_or("original partial interpretation")?;
    check!(eq; summary["capture"]["from_cache"], false);
    check!(eq; summary["owned_rows"], 2);
    check!(eq; summary["rejected_individual_rows"], 1);
    outcome.capture.from_cache = true;
    let recording = crate::recording::Recording::new();
    let ctx = AdapterContext {
        recording: Some(&recording),
        ..context(&store, &fetcher)?
    };
    record_outcome(&ctx, &reference, &outcome)?;
    check!(eq; recording.drain(), crate::recording::Recorded::default());
    record_outcome(&context(&store, &fetcher)?, &reference, &outcome)?;
    check!(eq; snapshot(&store, OWNED_MEET_PHASE)?, originals);
    check!(eq; snapshot(&store, OWNED_CAPTURE_PHASE)?, captures);
    Ok(())
}

#[test]
fn partial_persistence_retains_new_acquisition_without_restaging_identical_source_rows(
) -> TestResult {
    let (_dir, store, fetcher, reference) = setup()?;
    let mut outcome = partial_outcome(&reference)?;
    record_outcome(&context(&store, &fetcher)?, &reference, &outcome)?;
    let originals = snapshot(&store, OWNED_MEET_PHASE)?;
    let captures = snapshot(&store, OWNED_CAPTURE_PHASE)?;
    outcome.capture.fetched_at = "2026-10-02T00:00:00Z".into();
    outcome.capture.response_url = Some(outcome.capture.url.clone());
    let recording = crate::recording::Recording::new();
    let ctx = AdapterContext {
        recording: Some(&recording),
        ..context(&store, &fetcher)?
    };
    record_outcome(&ctx, &reference, &outcome)?;
    let recorded = recording.drain();
    let duplicate_rows: Vec<_> = recorded
        .journal
        .iter()
        .filter(|entry| {
            entry.phase == OWNED_MEET_PHASE
                && (entry.payload.get("result_id").is_some()
                    || entry.payload.get("locator").is_some())
        })
        .collect();
    check!(eq;
        duplicate_rows,
        Vec::<&crate::recording::RecordedJournal>::new()
    );
    let summaries: Vec<_> = recorded
        .journal
        .iter()
        .filter(|entry| {
            entry.phase == OWNED_MEET_PHASE && entry.payload["disposition"] == "partial"
        })
        .collect();
    check!(eq; summaries.len(), 1);
    check!(eq;
        summaries[0].payload["capture"]["fetched_at"],
        "2026-10-02T00:00:00Z"
    );
    check!(!originals.contains_key(&summaries[0].key));
    let mut batch = store.write_batch();
    for entry in &recorded.journal {
        batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
    }
    batch.commit()?;
    for (key, payload) in originals {
        check!(eq;
            store
                .journal_payload(OWNED_MEET_PHASE, &key)
                ?,
            Some(payload)
        );
    }
    for (key, payload) in captures {
        check!(eq;
            store
                .journal_payload(OWNED_CAPTURE_PHASE, &key)
                ?,
            Some(payload)
        );
    }
    Ok(())
}
