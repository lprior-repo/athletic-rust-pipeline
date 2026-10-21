use super::*;
use crate::restate_services::JobError;
use census_crawl::CollectionDisposition;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::Table;
use std::sync::Arc;

mod fixture;
use fixture::{failure, fetcher, index, TestResult, HOST};

#[tokio::test]
async fn owned_root_refusal_persists_once_and_restores_shared_cooldown_after_cold_open(
) -> TestResult {
    let (root, store) = census_store::test_fixture::temp_store()?;
    let fetcher = fetcher(root.path())?;
    let condition = fetcher
        .record_access_condition(
            HOST,
            AccessBlockKind::RateLimited,
            429,
            Some(120),
            "controlled root",
        )
        .await;
    let error = directory_result(failure(429), &fetcher, &store, UsJurisdiction::Michigan)
        .await
        .err()
        .ok_or("root refusal accepted")?;
    check!(
        matches!(&error, SourceAccessFailure::Primary(CrawlError::Fetch(
        FetchError::Http { status: 429, url }
    )) if url == "https://mi.milesplit.com/teams")
    );
    check!(matches!(JobError::from(error), JobError::Transient { .. }));
    check!(eq; index(&store)?.errors, 1);
    check!(eq; index(&store)?.disposition, CollectionDisposition::Partial);
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![condition.clone()]);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    drop((fetcher, store));
    let store = Store::open(root.path())?;
    let restored = census_crawl::net::PacingState::restore(
        &store,
        census_store::ReadBudget {
            rows: 16,
            bytes: 65536,
            record_bytes: 8192,
        },
    )?;
    let fresh = fixture::fetcher(root.path())?.with_shared_pacing(Arc::new(restored));
    check!(eq; fresh.access_conditions().await?, vec![condition.clone()]);
    check!(fresh.host_blocked(HOST, &condition.observed_at).await);
    check!(eq; index(&store)?.errors, 1);
    Ok(())
}

#[tokio::test]
async fn parser_store_error_keeps_its_transient_class_and_one_failure_receipt() -> TestResult {
    let (root, store) = census_store::test_fixture::temp_store()?;
    let fetcher = fetcher(root.path())?;
    let primary = census_store::StoreError::Io {
        path: root.path().join("controlled-index"),
        source: std::io::Error::new(std::io::ErrorKind::WouldBlock, "controlled index IO"),
    };
    let error = census::accept_state_team_index(
        &fetcher,
        &store,
        UsJurisdiction::Michigan,
        Err(primary.into()),
    )
    .await
    .err()
    .ok_or("index error accepted")?;
    check!(
        matches!(&error, SourceAccessFailure::Primary(CrawlError::Store(
        census_store::StoreError::Io { source, .. }
    )) if source.kind() == std::io::ErrorKind::WouldBlock)
    );
    check!(matches!(JobError::from(error), JobError::Transient { .. }));
    check!(eq; index(&store)?.errors, 1);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[tokio::test]
async fn owned_root_retention_failure_preserves_both_causes_without_another_write() -> TestResult {
    let (root, store) = census_store::test_fixture::temp_store()?;
    let fetcher = fetcher(root.path())?;
    store.journal_done(
        &census::teams_phase(UsJurisdiction::Michigan),
        "MI",
        &serde_json::json!({
            "disposition":"partial", "roster_teams":[], "unfinished":[], "errors":usize::MAX
        }),
    )?;
    let revision = (
        store.snapshot().evidence_generation(),
        store.snapshot().derived_generation(),
    );
    let sequence = store.snapshot().sequence();
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    let error = directory_result(failure(429), &fetcher, &store, UsJurisdiction::Michigan)
        .await
        .err()
        .ok_or("root refusal accepted")?;
    check!(
        matches!(&error, SourceAccessFailure::Retention { primary, secondary }
        if matches!(primary.as_ref(), CrawlError::Fetch(FetchError::Http { status:429, .. }))
        && matches!(secondary.as_ref(), CrawlError::Arithmetic { detail }
            if detail == "team index error count overflow"))
    );
    check!(matches!(JobError::from(error), JobError::Transient { .. }));
    check!(eq; (store.snapshot().evidence_generation(), store.snapshot().derived_generation()), revision);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    check!(eq; index(&store)?.errors, usize::MAX);
    Ok(())
}
