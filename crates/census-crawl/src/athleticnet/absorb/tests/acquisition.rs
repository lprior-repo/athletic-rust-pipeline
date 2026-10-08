mod journal;

use super::*;
use crate::athleticnet::collect::{PROFILE_ATTEMPT_PHASE, PROFILE_PARSE_VERSION};
use crate::athleticnet::{Options, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, SCOPES};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use census_domain::model::ReviewCase;
use census_store::{Store, Table};

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

fn seed_cache(cache: &std::path::Path, url: &str, body: &str) -> TestResult {
    let representation = crate::net::RepresentationHeaders::canonical(&[(
        "Accept".to_string(),
        "application/json".to_string(),
    )])?;
    let key = crate::net::Fetcher::key_for("GET", url, &representation.identity());
    let meta = CacheMeta {
        url: url.to_string(),
        method: "GET".to_string(),
        representation,
        status: 200,
        content_digest: content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: "2026-09-20T12:00:00Z".to_string(),
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

fn context<'a>(
    store: &'a Store,
    fetcher: &'a crate::net::Fetcher,
) -> TestResult<crate::AdapterContext<'a>> {
    Ok(crate::AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-09-30".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("snapshot date")?,
        recording: None,
    })
}

fn options(dir: &std::path::Path, id: u64) -> TestResult<Options> {
    let registry = dir.join("registry.txt");
    std::fs::write(&registry, format!("{id},WI\n"))?;
    Ok(Options {
        input: Some(registry.display().to_string()),
        observed_on: "2026-09-30".into(),
        ..Options::default()
    })
}

#[test]
fn mismatched_cached_profile_is_durable_rejection_not_a_success_receipt() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            let urls = source_urls(28127170);
            for url in &urls {
                seed_cache(&cache, url, CAPTURE)?;
            }
            let fetcher = crate::net::Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher)?;
            let report =
                crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28127170)?).await?;
            check!(eq; report.errors, 2);
            check!(crate::athleticnet::collect::journaled_urls(&ctx)?.is_empty());
            check!(store.journal_payloads("athleticnet")?.is_empty());
            for payload in store.journal_payloads(PROFILE_ATTEMPT_PHASE)? {
                check!(eq; payload.get("parsed"), Some(&serde_json::json!(false)));
                check!(payload
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("mismatch")?
                    .contains("Requested athlete 28127170 but returned athlete 28872883"));
            }
            for table in [
                Table::Athletes,
                Table::Schools,
                Table::Performances,
                Table::SourceObservations,
            ] {
                check!(eq; store.walk_table(table)?.rows, 0);
            }
            let reviews: Vec<ReviewCase> = store.snapshot().scan(Table::ReviewCases)?;
            for url in &urls {
                check!(reviews.iter().any(|review| review.subject_id
                    == format!("{url}#athlete/IDAthlete")
                    && review
                        .detail
                        .contains("Requested athlete 28127170 but returned athlete 28872883")));
            }
            Ok(())
        })
}

#[test]
fn malformed_bio_does_not_certify_a_completed_parse() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            for url in source_urls(28872883) {
                seed_cache(&cache, &url, r#"{"athlete":{"IDAthlete":"malformed"}}"#)?;
            }
            let fetcher = crate::net::Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher)?;
            let report =
                crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883)?).await?;
            check!(eq; report.errors, 2);
            check!(eq; report.unfinished, source_urls(28872883));
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(crate::athleticnet::collect::journaled_urls(&ctx)?.is_empty());
            check!(store.journal_payloads("athleticnet")?.is_empty());
            let reviews: Vec<ReviewCase> = store.snapshot().scan(Table::ReviewCases)?;
            for url in source_urls(28872883) {
                check!(reviews.iter().any(|review| review.subject_id == url
                    && review.state == census_domain::model::ReviewState::Pending));
            }
            Ok(())
        })
}

#[test]
fn admitted_empty_history_uses_each_exact_profile_fetch_locator() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            let mut profile: serde_json::Value = serde_json::from_str(CAPTURE)?;
            profile["resultsTF"] = serde_json::json!([]);
            profile["resultsXC"] = serde_json::json!([]);
            let profile = serde_json::to_string(&profile)?;
            let urls = source_urls(28872883);
            for url in &urls {
                seed_cache(&cache, url, &profile)?;
            }
            let fetcher = crate::net::Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher)?;
            let report =
                crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883)?).await?;
            check!(eq; report.errors, 0);
            check!(report.unfinished.is_empty());
            check!(eq; report.disposition, crate::CollectionDisposition::Complete);
            check!(eq; report.rows, 1);
            let athletes = store.snapshot().athletes()?;
            let mut evidence_urls = Vec::new();
            let mut index = census_domain::model::AthleteIdentityIndex::default();
            for athlete in &athletes {
                check!(eq; athlete.source.as_ref().ok_or("provider owner")?.id, "28872883");
                for support in &athlete.evidence {
                    check!(eq; support.method, census_domain::model::EvidenceMethod::Parsed);
                    check!(eq; support.observed_on, "2026-09-20T12:00:00Z");
                    evidence_urls.push(support.source.url.clone().ok_or("capture URL")?);
                }
                index.observe(athlete)?;
                check!(index.isolated_source(&athlete.id.cast()));
            }
            evidence_urls.sort();
            let mut expected_urls = urls.clone();
            expected_urls.sort();
            check!(eq; evidence_urls, expected_urls);
            check!(eq;
                crate::athleticnet::collect::journaled_urls(&ctx)
                    ?
                    .len(),
                2
            );
            Ok(())
        })
}
