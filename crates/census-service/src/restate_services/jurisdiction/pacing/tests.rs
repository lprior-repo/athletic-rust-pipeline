use super::*;
use crate::restate_services::{CaptureExecution, Jobs};
use crate::spawn::Spawner;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{clock::SystemClock, Store, Table};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn owner(store: &Arc<Store>, region: &Arc<Spawner>) -> JurisdictionCensus {
    JurisdictionCensus::new(
        Arc::clone(store),
        Arc::new(SystemClock),
        None,
        CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(store), Arc::clone(region)),
    )
}

fn condition() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "retained",
        "example.org",
        AccessBlockKind::HumanRequired,
        403,
        "2026-10-18T10:00:00Z",
        "operator required",
    )
}

#[tokio::test]
async fn concurrent_cold_owner_fetchers_share_one_restored_pacing_state() -> TestResult {
    let (root, store) = census_store::test_fixture::temp_store()?;
    let row = condition();
    store.replace_many(Table::SourceAccess, std::slice::from_ref(&row))?;
    drop(store);
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::new());
    let owner = owner(&store, &region);
    let hosts = vec!["example.org".into()];
    let (first, second) = tokio::join!(owner.fetcher(&hosts, 1), owner.fetcher(&hosts, 1));
    let first = first.map_err(crate::restate_services::tests::sdk_error)?;
    let second = second.map_err(crate::restate_services::tests::sdk_error)?;
    check!(Arc::ptr_eq(&first, &second));
    check!(eq; first.access_conditions().await?, vec![row]);
    let changed = owner
        .fetcher(&["other.example.org".into()], 1)
        .await
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(Arc::ptr_eq(&first.pacing_state(), &changed.pacing_state()));
    check!(
        changed
            .host_blocked("example.org", "2099-01-01T00:00:00Z")
            .await
    );
    check!(
        !changed
            .host_blocked("other.example.org", "2099-01-01T00:00:00Z")
            .await
    );
    let report = region.drain(Duration::from_secs(5)).await?;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    Ok(())
}

#[tokio::test]
async fn failed_cold_owner_restore_exposes_no_pacing_or_fetcher_and_can_retry() -> TestResult {
    let (_root, store) = census_store::test_fixture::temp_store()?;
    let store = Arc::new(store);
    let mut invalid = condition();
    invalid.cooldown_until = Some("malformed".into());
    store.replace_many(Table::SourceAccess, &[invalid])?;
    let region = Arc::new(Spawner::new());
    let owner = owner(&store, &region);
    check!(owner.fetcher(&[], 1).await.is_err());
    check!(owner.pacing.get().is_none());
    check!(owner.fetcher.lock().await.is_none());
    store.replace_many(Table::SourceAccess, &[condition()])?;
    let fetched = owner
        .fetcher(&[], 1)
        .await
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(
        fetched
            .host_blocked("example.org", "2099-01-01T00:00:00Z")
            .await
    );
    let report = region.drain(Duration::from_secs(5)).await?;
    check!(eq; report.accepted, 2);
    check!(eq; report.completed, 2);
    Ok(())
}

#[tokio::test]
async fn cancelled_cold_owner_initialization_leaves_cell_retryable() -> TestResult {
    let (_root, store) = census_store::test_fixture::temp_store()?;
    let store = Arc::new(store);
    store.replace(Table::SourceAccess, &condition())?;
    let region = Arc::new(Spawner::with_capacity(1));
    let owner = owner(&store, &region);
    let held = region
        .acquire_blocking()
        .await
        .ok_or("blocking lane closed")?;
    let mut pending = Box::pin(owner.restored_pacing());
    check!(futures::poll!(pending.as_mut()).is_pending());
    drop(pending);
    check!(owner.pacing.get().is_none());
    check!(owner.fetcher.lock().await.is_none());
    drop(held);
    let fetched = owner
        .fetcher(&[], 1)
        .await
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(
        fetched
            .host_blocked("example.org", "2099-01-01T00:00:00Z")
            .await
    );
    let report = region.drain(Duration::from_secs(5)).await?;
    check!(eq; report.remaining, 0);
    check!(eq; report.panicked, 0);
    Ok(())
}
