use super::{CanonicalPerformance, SchoolYear, Store, Table, TestResult, UsJurisdiction};
use census_crawl::milesplit::{
    collect_result_sets, ResultSetOptions, ResultSetRef, ResultSetRequest,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport, CaptureContext, CrawlResult};
use census_domain::model::{CanonicalSchool, SourceIdentity, SourceNamespace};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const OWNED: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/milesplit/owned/fixtures/troy_725218.json"
));
const RAW: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
));

pub(super) async fn append(store: &Store) -> TestResult<CanonicalPerformance> {
    append_division(store, "High School").await
}

pub(super) async fn append_division(
    store: &Store,
    division: &str,
) -> TestResult<CanonicalPerformance> {
    append_meet(
        store,
        "https://al.milesplit.com/meets/725218/results/1266814/raw",
        &synthetic_body(division)?,
        RAW,
        "milesplit_result:201782263",
    )
    .await
}

async fn append_meet(
    store: &Store,
    url: &str,
    owned: &[u8],
    raw: &[u8],
    source_key: &str,
) -> TestResult<CanonicalPerformance> {
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let reference = ResultSetRef::parse(url).ok_or("result set")?;
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Synthetic selected school",
        "selected school",
        None,
    )
    .0;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "38332",
    ));
    store.append(Table::Schools, &school)?;
    let request = census_crawl::milesplit::owned_meet_request(
        &fetcher,
        &reference,
        &FetchOptions::default(),
    )?;
    seed(&fetcher, request.original_url(), owned)?;
    seed(&fetcher, &reference.url, raw)?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("date")?,
        recording: None,
    };
    let options = ResultSetOptions {
        urls: vec![ResultSetRequest {
            url: reference.url,
            jurisdiction: UsJurisdiction::Alabama,
        }],
    };
    let acquisition: Pin<Box<dyn Future<Output = CrawlResult<AdapterReport>> + '_>> =
        Box::pin(collect_result_sets(&context, &options));
    let report = acquisition.await?;
    check!(eq; report.errors, 0);
    Ok(store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .find(|row| row.source_key == source_key)
        .ok_or("retained owned performance")?)
}

fn synthetic_body(division: &str) -> TestResult<Vec<u8>> {
    let mut body: Value = serde_json::from_slice(OWNED)?;
    let rows = body
        .get_mut("data")
        .and_then(Value::as_array_mut)
        .ok_or("owned data")?;
    rows.truncate(1);
    rows.first_mut()
        .and_then(Value::as_object_mut)
        .ok_or("owned row")?
        .insert("divisionName".into(), json!(division));
    Ok(serde_json::to_vec(&body)?)
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET\x1f");
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let digest = format!("{:x}", hasher.finalize());
    let key = digest.get(..32).ok_or("cache key")?;
    let metadata = json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)), "bytes": body.len(),
        "fetched_at": "2026-10-01T23:44:16Z", "content_type": null,
    });
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(())
}
