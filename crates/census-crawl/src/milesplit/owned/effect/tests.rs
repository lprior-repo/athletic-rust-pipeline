use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::collections::HashMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod replay;
mod uncaptured;

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
        observed_on: "2026-10-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("snapshot date")?,
        recording: None,
    })
}

fn seed(fetcher: &Fetcher, reference: &ResultSetRef, body: &[u8]) -> TestResult {
    let url = owned_meet_url(reference)?;
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
    std::fs::create_dir_all(fetcher.cache_dir())?;
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-10-01T23:44:16Z".into(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

fn parsed(outcome: &OwnedMeetOutcome) -> TestResult<&super::super::OwnedMeetPage> {
    match &outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => Ok(page),
        other => Err(format!("expected parsed capture, got {other:?}").into()),
    }
}

#[test]
fn successful_receipt_returns_current_owner_and_marks_for_downstream_replay() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let body = crate::milesplit::owned::tests::TROY;
            seed(&fetcher, &reference, body)?;
            let ctx = context(&store, &fetcher)?;
            let first = read_owned_meet(&ctx, &reference).await?;
            let payloads = store.journal_payloads(OWNED_MEET_PHASE)?;
            let captures = store.journal_payloads(OWNED_CAPTURE_PHASE)?;
            let replay = read_owned_meet(&ctx, &reference).await?;
            check!(eq; replay.verdict, first.verdict);
            let spann = parsed(&replay)?
                .rows
                .iter()
                .find(|row| row.result_id == 201782263)
                .ok_or("original long jump")?;
            check!(eq; spann.source_athlete.id, "14222592");
            check!(eq;
                spann.source_athlete.url.as_deref(),
                Some("https://www.milesplit.com/athletes/14222592-adelyn-spann")
            );
            check!(eq;
                spann.mark,
                census_domain::model::Mark::FieldImperial {
                    feet_mark: "13-9".into(),
                    metres: census_domain::model::CentiMetres::new(419),
                }
            );
            check!(eq; replay.capture.body, body);
            check!(eq; replay.capture.content_digest, content_digest(body));
            let receipt = payloads
                .iter()
                .find(|value| value["disposition"] == "parsed")
                .ok_or("parse receipt")?;
            check!(eq; receipt["ownership_complete"], false);
            check!(eq; receipt["completeness"], "unknown");
            check!(eq; receipt["owned_rows"], 3);
            let keys = store.journal_keys(OWNED_MEET_PHASE)?;
            check!(!keys.iter().any(|key| key.starts_with("complete/")));
            check!(eq;
                store
                    .journal_payloads(OWNED_MEET_PHASE)
                    ?,
                payloads
            );
            check!(eq;
                store
                    .journal_payloads(OWNED_CAPTURE_PHASE)
                    ?,
                captures
            );
            let chunk = captures
                .iter()
                .find(|value| value.get("raw_base64").is_some())
                .ok_or("capture chunk")?;
            check!(eq;
                STANDARD
                    .decode(chunk["raw_base64"].as_str().ok_or("base64")?)?,
                body
            );
            check!(eq; chunk["capture"]["content_digest"], content_digest(body));
            let traffic = fetcher.stats().await;
            check!(eq; traffic.physical_requests(), 0);
            check!(eq; traffic.cache_hits, 2);
            Ok(())
        })
}

#[test]
fn malformed_partial_and_refused_captures_never_receive_parse_success_receipts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let ctx = context(&store, &fetcher)?;
            let mut partial: serde_json::Value =
                serde_json::from_slice(crate::milesplit::owned::tests::TROY)?;
            partial["data"][0]["athleteId"] = serde_json::json!("0");
            let partial = serde_json::to_vec(&partial)?;
            for (status, body) in [
                (200, b"<html>Forbidden</html>".as_slice()),
                (200, partial.as_slice()),
                (404, b"not found".as_slice()),
            ] {
                let capture = FetchOutcome {
                    url: owned_meet_url(&reference)?,
                    response_url: None,
                    method: "GET".into(),
                    status,
                    content_digest: content_digest(body),
                    bytes: body.len(),
                    fetched_at: "2026-10-01T23:44:16Z".into(),
                    from_cache: false,
                    content_type: None,
                    body: body.to_vec(),
                };
                let verdict = if status == 200 {
                    parse_owned_meet(body, 725218)
                } else {
                    OwnedMeetVerdict::Refused { status }
                };
                record_outcome(&ctx, &reference, &OwnedMeetOutcome { capture, verdict })?;
                let digest = content_digest(body);
                let summaries = store.journal_payloads(OWNED_MEET_PHASE)?;
                let summary = summaries
                    .iter()
                    .find(|value| value["capture"]["content_digest"] == digest)
                    .ok_or("retained interpretation of the rejected or partial capture")?;
                check!(ne; summary["individual_parse_complete"], true);
                check!(store
                    .journal_keys(OWNED_CAPTURE_PHASE)?
                    .contains(&format!("725218/{digest}/manifest")));
            }
            let rows = store.journal_payloads(OWNED_MEET_PHASE)?;
            check!(rows.iter().any(|row| row["result_id"] == 201782277));
            check!(rows
                .iter()
                .any(|row| row["locator"] == "data[0]" && row["kind"] == "invalid_identity"));
            Ok(())
        })
}

#[test]
fn empty_success_is_capture_parse_success_with_unknown_population() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed(&fetcher, &reference, b"{\"data\":[]}")?;
            let ctx = context(&store, &fetcher)?;
            let outcome = read_owned_meet(&ctx, &reference).await?;
            check!(eq; parsed(&outcome)?.rows, Vec::new());
            check!(eq;
                parsed(&outcome)?.completeness,
                super::super::OwnedCompleteness::Unknown
            );
            check!(!parsed(&outcome)?.ownership_complete());
            let payload = store.journal_payloads(OWNED_MEET_PHASE)?;
            check!(eq; payload[0]["published_rows"], 0);
            check!(eq; payload[0]["ownership_complete"], false);
            check!(eq; payload[0]["completeness"], "unknown");
            Ok(())
        })
}

#[test]
fn recording_route_retains_all_capture_chunks_before_source_receipts_are_applied() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut body = crate::milesplit::owned::tests::TROY.to_vec();
            body.resize(CAPTURE_CHUNK_BYTES * 2 + 7, b' ');
            seed(&fetcher, &reference, &body)?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let outcome = read_owned_meet(&ctx, &reference).await?;
            check!(eq; outcome.capture.body, body);
            check!(eq;
                store
                    .journal_keys(OWNED_MEET_PHASE)
                    ?,
                std::collections::HashSet::new()
            );
            let recorded = recording.drain();
            let mut chunks: Vec<_> = recorded
                .journal
                .iter()
                .filter(|entry| {
                    entry.phase == OWNED_CAPTURE_PHASE && entry.payload.get("raw_base64").is_some()
                })
                .map(|entry| {
                    Ok((
                        entry.payload["chunk_index"]
                            .as_u64()
                            .ok_or("chunk ordinal")?,
                        entry,
                    ))
                })
                .collect::<TestResult<Vec<_>>>()?;
            chunks.sort_by_key(|(ordinal, _)| *ordinal);
            check!(eq; chunks.len(), 3);
            let restored = chunks.iter().try_fold(
                Vec::new(),
                |mut bytes, (_, entry)| -> TestResult<Vec<u8>> {
                    bytes.extend(
                        STANDARD.decode(
                            entry.payload["raw_base64"]
                                .as_str()
                                .ok_or("encoded capture")?,
                        )?,
                    );
                    Ok(bytes)
                },
            )?;
            check!(eq; restored, body);
            check!(recorded
                .journal
                .iter()
                .any(|entry| entry.phase == OWNED_MEET_PHASE && entry.key.starts_with("parsed/")));
            Ok(())
        })
}

#[test]
fn a_recorded_cooldown_refuses_the_meet_without_closing_it() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let live = Fetcher::new(
                dir.path().join("live"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                vec!["milesplit.com".into()],
            )?;
            let cooldown = live
                .record_access_condition(
                    "al.milesplit.com",
                    census_domain::model::AccessBlockKind::RateLimited,
                    429,
                    Some(60),
                    "recorded while the host was cooled down",
                )
                .await;
            check!(cooldown.is_blocking(&crate::net::now_iso8601()));
            let error = match read_owned_meet(&context(&store, &live)?, &reference).await {
                Err(error) => error,
                Ok(_) => return Err("a host inside a cooldown cannot acquire the meet".into()),
            };
            check!(
                matches!(
                    &error,
                    CrawlError::Fetch(crate::net::FetchError::Cooldown { host })
                        if host == "al.milesplit.com"
                ),
                "the refusal is a typed cooldown: {error:?}"
            );
            check!(error.retryable(), "a cooldown is temporary, not terminal");
            check!(
                eq;
                store.journal_payloads(OWNED_MEET_PHASE)?.len(),
                0,
                "a cooldown writes no completion receipt, so the meet stays owed"
            );
            seed(&fetcher, &reference, crate::milesplit::owned::tests::TROY)?;
            let outcome = read_owned_meet(&context(&store, &fetcher)?, &reference).await?;
            parsed(&outcome)?;
            check!(
                !store.journal_payloads(OWNED_MEET_PHASE)?.is_empty(),
                "the retry journals the completed meet under the same store"
            );
            Ok(())
        })
}
