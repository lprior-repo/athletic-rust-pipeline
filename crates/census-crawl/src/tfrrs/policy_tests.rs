use super::{collect, parse_list_page, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, Fetcher};
use crate::{AdapterContext, AdapterReport, CaptureContext, CollectionDisposition, CrawlError};
use census_domain::model::{CanonicalPerformance, CanonicalSchool, ExactSeconds, Mark, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const LIST: &str = include_str!("../../tests/fixtures/tfrrs/indiana_list_5489_excerpt.html");
const ACQUIRED: &str = "2026-09-22T03:55:31Z";
const PROJECTION_PHASE: &str = "tfrrs_projection_v5";

fn fetcher(root: &std::path::Path) -> TestResult<Fetcher> {
    let locks = root.join("dispatch-disabled");
    std::fs::write(&locks, b"the policy fixture cannot acquire an origin lock")?;
    Ok(Fetcher::new(
        root.join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_origin_locks(locks))
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
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("cutoff")?,
        recording: None,
    })
}

fn seed(fetcher: &Fetcher) -> TestResult<(Vec<u8>, Vec<u8>)> {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    write_cache(
        &body_path,
        &meta_path,
        LIST.as_bytes(),
        &CacheMeta {
            url: URL.into(),
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(LIST.as_bytes()),
            bytes: LIST.len(),
            fetched_at: ACQUIRED.into(),
            ..CacheMeta::default()
        },
    )?;
    Ok((std::fs::read(body_path)?, std::fs::read(meta_path)?))
}

fn schools(store: &Store) -> TestResult {
    let labels: BTreeSet<_> = parse_list_page(LIST)
        .sections
        .into_iter()
        .flat_map(|section| section.rows)
        .filter_map(|row| row.team.map(|team| team.name))
        .collect();
    for name in labels {
        let school = CanonicalSchool::new(
            UsJurisdiction::Indiana,
            &name,
            census_domain::model::normalize_name(&name),
            None,
        )
        .0;
        store.append(Table::Schools, &school)?;
    }
    Ok(())
}

fn receipts(store: &Store) -> TestResult<BTreeMap<String, serde_json::Value>> {
    store
        .journal_keys(PROJECTION_PHASE)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(PROJECTION_PHASE, &key)?
                .ok_or("projection receipt")?;
            Ok((key, payload))
        })
        .collect()
}

async fn replay(
    store: &Store,
    fetcher: &Fetcher,
    capture: &crate::ResultsCaptureContext,
) -> TestResult<AdapterReport> {
    Ok(collect(
        &context(store, fetcher, capture)?,
        &Options {
            urls: vec![URL.into()],
            ..Options::default()
        },
    )
    .await?)
}

#[tokio::test]
async fn online_collector_refuses_empty_and_cached_inputs_before_any_work() -> TestResult {
    let root = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&root.path().join("capture"))?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?;
    seed(&fetcher)?;
    let ctx = context(&store, &fetcher, &capture)?;
    for urls in [Vec::new(), vec![URL.into()]] {
        let result = collect(
            &ctx,
            &Options {
                urls,
                ..Options::default()
            },
        )
        .await;
        let Err(CrawlError::Fetch(error @ FetchError::Policy { .. })) = result else {
            return Err(format!("expected TFRRS collector refusal, got {result:?}").into());
        };
        check!(!error.retryable());
        let FetchError::Policy { detail } = error else {
            return Err("policy error changed variant".into());
        };
        check!(eq;
            Some(detail.as_str()),
            crate::registry::acquisition_refusal_for_source("tfrrs")
        );
    }
    check!(eq; fetcher.stats().await.requests, 0);
    check!(fetcher.access_conditions().await?.is_empty());
    for table in [
        Table::Schools,
        Table::Athletes,
        Table::Performances,
        Table::SourceObservations,
    ] {
        check!(eq; store.walk_table(table)?.rows, 0);
    }
    check!(store.journal_keys(PROJECTION_PHASE)?.is_empty());
    Ok(())
}

#[tokio::test]
async fn explicit_offline_replay_preserves_capture_dates_rows_and_completion_on_reopen(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?.with_offline(true);
    let durable = crate::net::durable_capture_context(&root.path().join("capture"))?;
    let capture = seed(&fetcher)?;
    schools(&store)?;
    let report = replay(&store, &fetcher, &durable).await?;
    check!(eq; report.disposition, CollectionDisposition::Complete);
    check!(eq; (report.errors, report.rows, report.requests, report.from_cache), (0, 20, 0, 1));
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; performances.len(), 20);
    let control = performances
        .iter()
        .find(|row| {
            row.source_athlete
                .as_ref()
                .is_some_and(|owner| owner.id == "8817192")
        })
        .ok_or("retained control performance")?;
    check!(eq; control.mark, Mark::TimeSeconds(ExactSeconds::parse("6.97")?));
    check!(eq; control.date, "2026-03-14");
    check!(!control.evidence.is_empty());
    check!(control
        .evidence
        .iter()
        .all(|evidence| evidence.observed_on == ACQUIRED));
    let physical = store.walk_table(Table::Performances)?;
    let current = receipts(&store)?;
    check!(eq; current.len(), 1);
    drop(store);
    let store = Store::open(root.path().join("store"))?;
    let repeated = replay(&store, &fetcher, &durable).await?;
    check!(eq; repeated.disposition, CollectionDisposition::Complete);
    check!(eq; (repeated.errors, repeated.requests, repeated.from_cache), (0, 0, 1));
    check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, performances);
    check!(eq; store.walk_table(Table::Performances)?, physical);
    check!(eq; receipts(&store)?, current);
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    check!(eq; (std::fs::read(body_path)?, std::fs::read(meta_path)?), capture);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(())
}
