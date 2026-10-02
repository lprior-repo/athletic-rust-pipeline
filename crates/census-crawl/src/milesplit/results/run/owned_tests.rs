use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;

#[tokio::test]
async fn completed_legacy_raw_parse_does_not_suppress_owned_meet_acquisition() {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let store = Store::open(dir.path().join("store")).expect("isolated store");
    store
        .journal_done(
            "milesplit_result_sets_v2",
            "725218/1266814",
            &serde_json::json!({"disposition":"complete"}),
        )
        .expect("historical raw receipt");
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
            .expect("reference");
    let body = include_bytes!("../../owned/fixtures/troy_725218.json");
    seed(&fetcher, &reference, body);
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("school year"),
        observed_on: "2026-10-01".into(),
        recording: None,
    };
    let mut run = Run {
        index: SchoolIndex::from_schools(&[]),
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        done: store
            .journal_keys("milesplit_result_sets_v2")
            .expect("legacy keys"),
        pending: Vec::new(),
    };
    run.read(&ctx, &reference).await;
    assert_eq!(run.stats.result_sets_resumed, 1);
    assert_eq!(run.stats.failures, Vec::<String>::new());
    let owned = store
        .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)
        .expect("owned evidence");
    let long = owned
        .iter()
        .find(|row| row["result_id"] == 201782263)
        .expect("owner acquired despite old receipt");
    assert_eq!(long["source_athlete"]["id"], "14222592");
    assert_eq!(long["locator"], "data[0]");
    let other = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
        .expect("second result set");
    run.done.insert("725218/1266815".into());
    run.read(&ctx, &other).await;
    assert_eq!(run.stats.result_sets_resumed, 2);
    assert_eq!(fetcher.stats().await.cache_hits, 1);
    assert_eq!(
        store
            .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)
            .expect("dedup evidence"),
        owned
    );
}

fn seed(fetcher: &Fetcher, reference: &ResultSetRef, body: &[u8]) {
    let url = crate::milesplit::fetch::owned_meet_url(reference).expect("URL");
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
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
    .expect("source fixture cache");
}

#[tokio::test]
async fn a_signed_meet_identifier_cannot_rebind_a_public_owned_capture() {
    let dir = tempfile::tempdir().expect("isolated acquisition");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )
    .expect("fetcher")
    .with_offline(true);
    let mut reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .expect("reference");
    seed(
        &fetcher,
        &reference,
        include_bytes!("../../owned/fixtures/troy_725218.json"),
    );
    reference.meet_id = "+725218".to_string();
    match crate::milesplit::fetch_owned_meet(
        &fetcher,
        &reference,
        &crate::net::FetchOptions::default(),
    )
    .await
    {
        Err(crate::CrawlError::Schema { detail, .. }) => {
            assert_eq!(detail, "invalid structured meet ID")
        }
        other => panic!(
            "noncanonical source identity must not acquire another meet's capture: {other:?}"
        ),
    }
    assert_eq!(fetcher.stats().await.cache_hits, 0);
}
