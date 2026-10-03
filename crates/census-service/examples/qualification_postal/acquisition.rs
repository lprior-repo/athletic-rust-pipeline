use anyhow::{ensure, Context};
use census_crawl::coach_directories::{self, parse_directory, parse_summary};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use super::artifacts::{sha256, write_json, write_new, Result};
use super::{CAPTURED, NAME, ORG};

pub(super) const DIRECTORY: &[u8] = include_bytes!(
    "../../../census-crawl/tests/fixtures/coach_directories/nchsaa_directory_p1.json"
);
pub(super) const SUMMARY: &[u8] = include_bytes!(
    "../../../census-crawl/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
);

pub(super) fn run(root: &Path, store: &Store, observed: &str) -> Result<[Value; 2]> {
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::from_secs(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_source("coach_directories")
    .with_offline(true);
    verify_capture_owners()?;
    let captures = [
        (
            coach_directories::directory_page_url("NCHSAA", 1),
            DIRECTORY,
        ),
        (coach_directories::summary_url("ZCUM49"), SUMMARY),
    ];
    let [(directory_url, directory), (summary_url, summary)] = &captures;
    let metadata = [
        seed_capture(&fetcher, directory_url, directory)?,
        seed_capture(&fetcher, summary_url, summary)?,
    ];
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let report = runtime.block_on(acquire(store, &fetcher, observed))?;
    write_json(
        &root.join("acquisition.json"),
        &serde_json::to_value(&report)?,
    )?;
    ensure!(
        report.rows == 1 && report.errors == 0 && report.requests == 0 && report.from_cache == 2,
        "offline collector did not complete exactly one school from two captures: {report:?}"
    );
    runtime.block_on(verify_cached(&fetcher, &captures))?;
    Ok(metadata)
}

fn verify_capture_owners() -> Result<()> {
    let directory = parse_directory(DIRECTORY)?;
    let mut selected = directory
        .results
        .iter()
        .filter(|row| row.short_code.as_deref() == Some("ZCUM49"));
    let row = selected.next().context("selected directory owner absent")?;
    ensure!(
        selected.next().is_none(),
        "directory must contain exactly one ZCUM49 owner"
    );
    ensure!(
        row.org_id.as_deref() == Some(ORG)
            && row.name.as_deref() == Some(NAME)
            && row.state_code.as_deref() == Some("NC"),
        "directory owner differs from qualification contract"
    );
    let summary = parse_summary(SUMMARY)?;
    ensure!(
        summary.id == ORG
            && summary.short_code == "ZCUM49"
            && summary.name == NAME
            && summary.state_code == "NC",
        "summary owner differs from directory owner"
    );
    Ok(())
}

fn seed_capture(fetcher: &Fetcher, url: &str, body: &[u8]) -> Result<Value> {
    ensure!(
        body.len() <= 32 * 1024 * 1024,
        "fixture exceeds fetcher body budget"
    );
    let mut key = Sha256::new();
    key.update(b"GET\x1f");
    key.update(url.as_bytes());
    key.update(b"\x1f");
    let digest = format!("{:x}", key.finalize());
    let key = digest.get(..32).context("cache key digest is too short")?;
    let metadata = json!({"url": url, "method": "GET", "status": 200,
        "content_digest": sha256(body), "bytes": body.len(), "fetched_at": CAPTURED,
        "etag": null, "last_modified": null, "content_type": "application/json"});
    write_new(&fetcher.cache_dir().join(format!("{key}.body")), body)?;
    write_json(
        &fetcher.cache_dir().join(format!("{key}.meta.json")),
        &metadata,
    )?;
    Ok(metadata)
}

#[tracing::instrument(skip(store, fetcher))]
async fn acquire(store: &Store, fetcher: &Fetcher, observed: &str) -> Result<AdapterReport> {
    let context = AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).context("unsupported school year")?,
        observed_on: observed.to_owned(),
        recording: None,
    };
    Ok(coach_directories::collect(
        &context,
        &coach_directories::Options {
            limit: Some(1),
            states: vec![UsJurisdiction::NorthCarolina],
            school_names: vec![NAME.to_owned()],
            ..Default::default()
        },
    )
    .await?)
}

#[tracing::instrument(skip(fetcher, captures))]
async fn verify_cached(fetcher: &Fetcher, captures: &[(String, &[u8]); 2]) -> Result<()> {
    let options = FetchOptions::default();
    let [(directory_url, _), (summary_url, _)] = captures;
    let (directory, summary) = tokio::join!(
        fetcher.get(directory_url, &options),
        fetcher.get(summary_url, &options)
    );
    [directory?, summary?]
        .iter()
        .zip(captures)
        .try_for_each(|(outcome, (url, body))| {
            ensure!(
                outcome.url == *url
                    && outcome.method == "GET"
                    && outcome.status == 200
                    && outcome.from_cache
                    && outcome.fetched_at == CAPTURED
                    && outcome.body == *body
                    && outcome.bytes == body.len()
                    && outcome.content_digest == sha256(body)
                    && outcome.content_type.as_deref() == Some("application/json"),
                "cache readback differs for {url}"
            );
            Ok(())
        })
}
