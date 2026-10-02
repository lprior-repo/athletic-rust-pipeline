use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::collections::HashMap;

fn setup() -> (tempfile::TempDir, Store, Fetcher, ResultSetRef) {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let store = Store::open(dir.path().join("store")).expect("isolated store");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )
    .expect("fetcher")
    .with_offline(true);
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .expect("source reference");
    (dir, store, fetcher, reference)
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("school year"),
        observed_on: "2026-10-01".into(),
        recording: None,
    }
}

fn seed(fetcher: &Fetcher, reference: &ResultSetRef, body: &[u8]) {
    let url = owned_meet_url(reference).expect("provider URL");
    let (body_path, meta_path) = fetcher.cache_paths("52e0b5d61b6c7de90be2a35dd5fc3f42");
    std::fs::create_dir_all(fetcher.cache_dir()).expect("cache directory");
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            url,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-10-01T23:44:16Z".into(),
            ..CacheMeta::default()
        },
    )
    .expect("verified source fixture");
}

#[tokio::test]
async fn successful_parse_receipt_reuses_verified_capture_without_population_completion() {
    let (_dir, store, fetcher, reference) = setup();
    let body = crate::milesplit::owned::tests::TROY;
    seed(&fetcher, &reference, body);
    let ctx = context(&store, &fetcher);
    assert_eq!(
        read_owned_meet(&ctx, &reference)
            .await
            .expect("source effect"),
        None
    );
    let digest = content_digest(body);
    let keys = store.journal_keys(OWNED_MEET_PHASE).expect("owned journal");
    assert!(keys.contains(&format!("parsed/725218/{digest}")));
    assert!(!keys.iter().any(|key| key.starts_with("complete/")));
    let first = store
        .journal_payloads(OWNED_MEET_PHASE)
        .expect("source payloads");
    let receipt = first
        .iter()
        .find(|value| value["disposition"] == "parsed")
        .expect("parse receipt");
    assert_eq!(receipt["ownership_complete"], false);
    assert_eq!(receipt["completeness"], "unknown");
    assert_eq!(receipt["owned_rows"], 3);
    let captures = store
        .journal_payloads(OWNED_CAPTURE_PHASE)
        .expect("durable bytes");
    let chunk = captures
        .iter()
        .find(|value| value.get("raw_base64").is_some())
        .expect("capture chunk");
    let bytes = STANDARD
        .decode(chunk["raw_base64"].as_str().expect("encoded capture"))
        .expect("original bytes");
    assert_eq!(bytes, body);
    assert_eq!(chunk["capture"]["content_digest"], digest);
    assert_eq!(
        read_owned_meet(&ctx, &reference).await.expect("replay"),
        None
    );
    assert_eq!(
        store
            .journal_payloads(OWNED_MEET_PHASE)
            .expect("replay payloads"),
        first
    );
    assert_eq!(
        store
            .journal_payloads(OWNED_CAPTURE_PHASE)
            .expect("replay capture"),
        captures
    );
    let traffic = fetcher.stats().await;
    assert_eq!(traffic.physical_requests(), 0);
    assert_eq!(traffic.cache_hits, 2);
}

#[tokio::test]
async fn malformed_partial_and_refused_captures_never_receive_parse_success_receipts() {
    let (_dir, store, fetcher, reference) = setup();
    let ctx = context(&store, &fetcher);
    let mut partial: serde_json::Value =
        serde_json::from_slice(crate::milesplit::owned::tests::TROY).expect("rows");
    partial["data"][0]["athleteId"] = serde_json::json!("0");
    let partial = serde_json::to_vec(&partial).expect("partial source");
    for (status, body) in [
        (200, b"<html>Forbidden</html>".as_slice()),
        (200, partial.as_slice()),
        (404, b"not found".as_slice()),
    ] {
        let capture = FetchOutcome {
            url: owned_meet_url(&reference).expect("URL"),
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
        assert!(
            record_outcome(&ctx, &reference, &OwnedMeetOutcome { capture, verdict })
                .expect("retained failure")
                .is_some()
        );
        let digest = content_digest(body);
        let keys = store.journal_keys(OWNED_MEET_PHASE).expect("journal");
        assert!(keys.contains(&format!("partial/725218/{digest}")));
        assert!(!keys.contains(&format!("parsed/725218/{digest}")));
        assert!(store
            .journal_keys(OWNED_CAPTURE_PHASE)
            .expect("capture")
            .contains(&format!("725218/{digest}/manifest")));
    }
    let rows = store
        .journal_payloads(OWNED_MEET_PHASE)
        .expect("partial rows");
    assert!(rows.iter().any(|row| row["result_id"] == 201782277));
    assert!(rows
        .iter()
        .any(|row| row["locator"] == "data[0]" && row["kind"] == "invalid_identity"));
}

#[tokio::test]
async fn uncaptured_transport_failure_is_explicit_and_does_not_seal_the_meet() {
    let (_dir, store, fetcher, reference) = setup();
    let ctx = context(&store, &fetcher);
    let failure = read_owned_meet(&ctx, &reference)
        .await
        .expect("failure retained")
        .expect("source failure");
    assert!(failure.contains("offline and not cached"));
    assert_eq!(
        store
            .journal_keys(OWNED_MEET_PHASE)
            .expect("failure journal"),
        std::collections::HashSet::from(["failed/725218".to_string()])
    );
    assert_eq!(
        store
            .journal_keys(OWNED_CAPTURE_PHASE)
            .expect("no fabricated capture"),
        std::collections::HashSet::new()
    );
    let payload = store
        .journal_payloads(OWNED_MEET_PHASE)
        .expect("failure payload");
    assert_eq!(payload[0]["disposition"], "failed");
    assert_eq!(payload[0]["capture_available"], false);
}

#[tokio::test]
async fn empty_success_is_capture_parse_success_with_unknown_population() {
    let (_dir, store, fetcher, reference) = setup();
    seed(&fetcher, &reference, b"{\"data\":[]}");
    let ctx = context(&store, &fetcher);
    assert_eq!(
        read_owned_meet(&ctx, &reference)
            .await
            .expect("empty capture parse"),
        None
    );
    let payload = store
        .journal_payloads(OWNED_MEET_PHASE)
        .expect("empty receipt");
    assert_eq!(payload[0]["published_rows"], 0);
    assert_eq!(payload[0]["ownership_complete"], false);
    assert_eq!(payload[0]["completeness"], "unknown");
}

#[tokio::test]
async fn recording_route_retains_all_capture_chunks_before_source_receipts_are_applied() {
    let (_dir, store, fetcher, reference) = setup();
    let mut body = crate::milesplit::owned::tests::TROY.to_vec();
    body.resize(CAPTURE_CHUNK_BYTES * 2 + 7, b' ');
    seed(&fetcher, &reference, &body);
    let recording = crate::recording::Recording::new();
    let ctx = AdapterContext {
        recording: Some(&recording),
        ..context(&store, &fetcher)
    };
    assert_eq!(
        read_owned_meet(&ctx, &reference)
            .await
            .expect("recorded source effect"),
        None
    );
    assert_eq!(
        store
            .journal_keys(OWNED_MEET_PHASE)
            .expect("unapplied journal"),
        std::collections::HashSet::new()
    );
    let recorded = recording.drain();
    let mut chunks: Vec<_> = recorded
        .journal
        .iter()
        .filter(|entry| {
            entry.phase == OWNED_CAPTURE_PHASE && entry.payload.get("raw_base64").is_some()
        })
        .collect();
    chunks.sort_by_key(|entry| {
        entry.payload["chunk_index"]
            .as_u64()
            .expect("chunk ordinal")
    });
    assert_eq!(chunks.len(), 3);
    let restored: Vec<u8> = chunks
        .iter()
        .flat_map(|entry| {
            STANDARD
                .decode(
                    entry.payload["raw_base64"]
                        .as_str()
                        .expect("encoded capture"),
                )
                .expect("captured raw bytes")
        })
        .collect();
    assert_eq!(restored, body);
    assert!(recorded
        .journal
        .iter()
        .any(|entry| entry.phase == OWNED_MEET_PHASE && entry.key.starts_with("parsed/")));
}

#[tokio::test]
async fn uncaptured_access_refusal_is_not_a_transport_failure_or_fabricated_capture() {
    for status in [401, 403] {
        let (_dir, store, fetcher, reference) = setup();
        let ctx = context(&store, &fetcher);
        let url = owned_meet_url(&reference).expect("source URL");
        let error = CrawlError::Fetch(crate::net::FetchError::Http {
            status,
            url: url.clone(),
        });
        let failure = record_failure(&ctx, &reference, &error).expect("durable refusal");
        assert_eq!(failure, Some(format!("{url}: {error}")));
        assert_eq!(
            store.journal_keys(OWNED_MEET_PHASE).expect("refusal keys"),
            std::collections::HashSet::from(["refused/725218".to_string()])
        );
        assert_eq!(
            store
                .journal_keys(OWNED_CAPTURE_PHASE)
                .expect("no manufactured capture"),
            std::collections::HashSet::new()
        );
        let payload = store
            .journal_payloads(OWNED_MEET_PHASE)
            .expect("refusal evidence");
        assert_eq!(payload[0]["disposition"], "refused");
        assert_eq!(payload[0]["capture_available"], false);
        assert_eq!(payload[0]["source_url"], url);
        assert_eq!(payload[0]["error"], error.to_string());
    }
}
