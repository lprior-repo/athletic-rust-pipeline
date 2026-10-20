use super::{bound_fetch_options, collect, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, Fetcher, RepresentationHeaders};
use crate::{AdapterContext, CaptureContext, CrawlError};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::{collections::HashMap, time::Duration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn fetcher(store: &Store, offline: bool) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(offline))
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: CaptureContext::DurableV1(&capture),
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-10-14".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-14", "%Y-%m-%d")?,
        recording: None,
    })
}

#[tokio::test]
async fn online_collector_refuses_before_an_empty_success_or_any_work() -> TestResult {
    let root = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&root.path().join("capture"))?;
    let store = Store::open(root.path())?;
    let fetcher = fetcher(&store, false)?;
    let ctx = context(&store, &fetcher, &capture)?;
    let options = Options {
        states: vec![UsJurisdiction::Michigan],
        ..Options::default()
    };
    let result = collect(&ctx, &options).await;
    let Err(CrawlError::Fetch(error @ FetchError::Policy { .. })) = result else {
        return Err(format!("expected online policy refusal, got {result:?}").into());
    };
    check!(!error.retryable());
    let FetchError::Policy { detail } = error else {
        return Err("policy error changed variant".into());
    };
    check!(eq;
        Some(detail.as_str()),
        crate::registry::acquisition_refusal_for_source("bound")
    );
    check!(eq; fetcher.stats().await.requests, 0);
    check!(fetcher.access_conditions().await?.is_empty());
    for table in [Table::Schools, Table::Coaches, Table::SourceObservations] {
        check!(eq; store.walk_table(table)?.rows, 0);
    }
    Ok(())
}

fn seed(ctx: &AdapterContext<'_>, url: &str, body: &[u8]) -> TestResult {
    let options = bound_fetch_options(ctx);
    let representation = RepresentationHeaders::for_request(url, &options.headers)?;
    let key = Fetcher::key_for("GET", url, &representation.identity());
    let (body_path, meta_path) = ctx.fetcher.cache_paths(&key);
    let meta = CacheMeta {
        url: url.into(),
        method: "GET".into(),
        representation,
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-07T10:00:00Z".into(),
        content_type: Some("text/html".into()),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok(())
}

#[tokio::test]
async fn offline_collector_still_decodes_the_held_representation_without_acquisition() -> TestResult
{
    let root = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&root.path().join("capture"))?;
    let store = Store::open(root.path())?;
    let fetcher = fetcher(&store, true)?;
    let ctx = context(&store, &fetcher, &capture)?;
    seed(
        &ctx,
        "https://www.gobound.com/sd/schools",
        br#"<a href="/sd/schools/yankton">Yankton</a>"#,
    )?;
    let held = include_bytes!("../../../tests/fixtures/bound/staff-yankton-sd-girls-xc.html");
    seed(
        &ctx,
        "https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff",
        held,
    )?;
    let report = collect(
        &ctx,
        &Options {
            states: vec![UsJurisdiction::SouthDakota],
            school_names: vec!["Yankton".into()],
            ..Options::default()
        },
    )
    .await?;
    check!(eq; report.rows, 1);
    check!(eq; report.from_cache, 2);
    check!(eq; report.requests, 0);
    check!(eq; store.walk_table(Table::Coaches)?.rows, 2);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}
