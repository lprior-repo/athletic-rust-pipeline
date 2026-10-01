mod journal;

use super::*;
use crate::athleticnet::collect::{PROFILE_ATTEMPT_PHASE, PROFILE_PARSE_VERSION};
use crate::athleticnet::{Options, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, SCOPES};
use census_domain::model::ReviewCase;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};

const CAPTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/athleticnet/samples/live-getathletebiodata-28872883-2026-09-20.json"
));

fn source_urls(id: u64) -> Vec<String> {
    SCOPES
        .iter()
        .map(|scope| {
            format!(
                "{BIO_ENDPOINT}?athleteId={id}&sport={}&level={HIGH_SCHOOL_LEVEL}",
                scope.parameter(),
            )
        })
        .collect()
}

fn seed_cache(cache: &std::path::Path, url: &str, body: &str) {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(), "fetched_at": "2026-09-20T12:00:00Z",
    });
    std::fs::create_dir_all(cache).expect("cache directory");
    std::fs::write(cache.join(format!("{key}.body")), body).expect("body");
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta).expect("metadata"),
    )
    .expect("cache metadata");
}

fn context<'a>(store: &'a Store, fetcher: &'a crate::net::Fetcher) -> crate::AdapterContext<'a> {
    crate::AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("school year"),
        observed_on: "2026-09-30".into(),
        recording: None,
    }
}

fn options(dir: &std::path::Path, id: u64) -> Options {
    let registry = dir.join("registry.txt");
    std::fs::write(&registry, format!("{id},WI\n")).expect("registry");
    Options {
        input: Some(registry.display().to_string()),
        observed_on: "2026-09-30".into(),
        ..Options::default()
    }
}

#[tokio::test]
async fn mismatched_cached_profile_is_durable_rejection_not_a_success_receipt() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let cache = dir.path().join("http");
    let urls = source_urls(28127170);
    for url in &urls {
        seed_cache(&cache, url, CAPTURE);
    }
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    let report = crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28127170))
        .await
        .expect("report rejects independent pages");
    assert_eq!(report.errors, 2);
    assert!(crate::athleticnet::collect::journaled_urls(&ctx)
        .expect("completion keys")
        .is_empty());
    assert!(store
        .journal_payloads("athleticnet")
        .expect("no successful receipts")
        .is_empty());
    for payload in store
        .journal_payloads(PROFILE_ATTEMPT_PHASE)
        .expect("attempt outcomes")
    {
        assert_eq!(payload.get("parsed"), Some(&serde_json::json!(false)));
        assert!(payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .expect("mismatch")
            .contains("Requested athlete 28127170 but returned athlete 28872883"));
    }
    for table in [
        Table::Athletes,
        Table::Schools,
        Table::Performances,
        Table::SourceObservations,
    ] {
        assert_eq!(store.walk_table(table).expect("no foreign rows").rows, 0);
    }
    let reviews: Vec<ReviewCase> = store
        .snapshot()
        .scan(Table::ReviewCases)
        .expect("rejections");
    for url in &urls {
        assert!(reviews.iter().any(|review| review.subject_id
            == format!("{url}#athlete/IDAthlete")
            && review
                .detail
                .contains("Requested athlete 28127170 but returned athlete 28872883")));
    }
}

#[tokio::test]
async fn malformed_bio_does_not_certify_a_completed_parse() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let cache = dir.path().join("http");
    for url in source_urls(28872883) {
        seed_cache(&cache, &url, r#"{"athlete":{"IDAthlete":"malformed"}}"#);
    }
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    let report = crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883))
        .await
        .expect("failed parse report");
    assert_eq!(report.errors, 2);
    assert!(crate::athleticnet::collect::journaled_urls(&ctx)
        .expect("completion keys")
        .is_empty());
    assert!(store
        .journal_payloads("athleticnet")
        .expect("no successful receipts")
        .is_empty());
    let reviews: Vec<ReviewCase> = store
        .snapshot()
        .scan(Table::ReviewCases)
        .expect("rejections");
    for url in source_urls(28872883) {
        assert!(reviews.iter().any(|review| review.subject_id == url
            && review.detail.contains("body is not an athlete bio")));
    }
}

#[tokio::test]
async fn admitted_empty_history_uses_each_exact_profile_fetch_locator() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let cache = dir.path().join("http");
    let mut profile: serde_json::Value = serde_json::from_str(CAPTURE).expect("public capture");
    profile["resultsTF"] = serde_json::Value::Null;
    profile["resultsXC"] = serde_json::Value::Null;
    let profile = serde_json::to_string(&profile).expect("derived empty history");
    let urls = source_urls(28872883);
    for url in &urls {
        seed_cache(&cache, url, &profile);
    }
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    let report = crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883))
        .await
        .expect("admitted profiles");
    assert_eq!(report.errors, 0);
    assert_eq!(report.rows, 1);
    let athletes = store.snapshot().athletes().expect("profile-only subject");
    let athlete = athletes.first().expect("admitted");
    assert_eq!(
        athlete.source.as_ref().expect("provider owner").id,
        "28872883"
    );
    for url in &urls {
        assert!(athlete.evidence.iter().any(|support| {
            support.source.url.as_deref() == Some(url.as_str())
                && support.method == census_domain::model::EvidenceMethod::Parsed
        }));
    }
    assert_eq!(athlete.evidence.len(), 2);
    let mut index = census_domain::model::AthleteIdentityIndex::default();
    index.observe(athlete).expect("owner evidence");
    assert!(index.isolated_source(&athlete.id.cast()));
    assert_eq!(
        crate::athleticnet::collect::journaled_urls(&ctx)
            .expect("completed")
            .len(),
        2
    );
}
