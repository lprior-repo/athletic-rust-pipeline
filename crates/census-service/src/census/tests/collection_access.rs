use crate::census::{collect_milesplit, CollectOptions, SourceAccessFailure};
use census_crawl::net::{FetchError, Fetcher};
use census_crawl::CrawlError;
use census_domain::model::{AccessBlockKind, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, StoreError, MAX_OBSERVATION_RECORD_BYTES};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn options(jurisdictions: Vec<UsJurisdiction>) -> CollectOptions {
    CollectOptions {
        jurisdictions,
        limit_per_state: Some(0),
        concurrency: 1,
        state_concurrency: 1,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: "2026-10-14".into(),
        revision: std::num::NonZeroU32::MIN,
    }
}

fn fetcher(root: &std::path::Path) -> census_crawl::CrawlResult<Fetcher> {
    Ok(Fetcher::new(
        root.join("http"),
        None,
        std::time::Duration::ZERO,
        std::collections::HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true))
}

async fn oversized_access(fetcher: &Fetcher) {
    fetcher
        .record_access_condition(
            "unrelated.example.org",
            AccessBlockKind::Forbidden,
            403,
            None,
            "x".repeat(MAX_OBSERVATION_RECORD_BYTES),
        )
        .await;
}

#[tokio::test]
async fn state_failure_survives_a_secondary_access_snapshot_refusal() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?;
    oversized_access(&fetcher).await;
    let result =
        collect_milesplit(&fetcher, &store, &options(vec![UsJurisdiction::Wisconsin])).await;
    match result {
        Err(SourceAccessFailure::Retention { primary, secondary }) => {
            check!(matches!(*primary, CrawlError::Invariant { detail }
                if detail.starts_with("1 state(s) failed after 0 rosters: WI: collecting WI team index:")
                    && detail.contains("offline and not cached: https://wi.milesplit.com/teams")));
            check!(
                matches!(*secondary, CrawlError::Store(StoreError::RowTooLarge { bytes, max })
                if bytes > max && max == MAX_OBSERVATION_RECORD_BYTES)
            );
        }
        _ => {
            return Err("expected both the state failure and typed access snapshot refusal".into())
        }
    }
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}

#[tokio::test]
async fn state_failure_with_successful_access_snapshot_stays_primary() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?;
    let original = CrawlError::Fetch(FetchError::Offline {
        url: "https://wi.milesplit.com/teams".into(),
    });
    let expected =
        format!("1 state(s) failed after 0 rosters: WI: collecting WI team index: {original}");
    let result =
        collect_milesplit(&fetcher, &store, &options(vec![UsJurisdiction::Wisconsin])).await;
    check!(
        matches!(result, Err(SourceAccessFailure::Primary(CrawlError::Invariant { detail }))
        if detail == expected)
    );
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}

#[tokio::test]
async fn access_snapshot_refusal_without_state_failure_is_primary() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?;
    oversized_access(&fetcher).await;
    let result = collect_milesplit(&fetcher, &store, &options(Vec::new())).await;
    check!(matches!(result,
        Err(SourceAccessFailure::Primary(CrawlError::Store(StoreError::RowTooLarge { bytes, max })))
            if bytes > max && max == MAX_OBSERVATION_RECORD_BYTES));
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}

#[tokio::test]
async fn successful_collection_keeps_the_completed_report() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(root.path())?;
    let report = collect_milesplit(&fetcher, &store, &options(Vec::new())).await?;
    check!(eq; (report.teams_total, report.rosters_fetched, report.athletes_total, report.errors),
        (0, 0, 0, 0));
    check!(report.states.is_empty());
    check!(report.access_conditions.is_empty());
    check!(report.blocked_hosts.is_empty());
    check!(report.evidence_losses.is_empty());
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}
