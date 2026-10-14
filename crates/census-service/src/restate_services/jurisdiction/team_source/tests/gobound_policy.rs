use super::{key, ledger, request, reserve, TestResult};
use crate::restate_services::jobs::AdapterScope;
use crate::restate_services::teams_arms::{self, SourceRuntime};
use crate::restate_services::wire::{TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::JobError;
use census_crawl::net::Fetcher;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

type ResultOf<T> = Result<T, Box<dyn std::error::Error>>;

async fn refused_dispatch(
    root: &Path,
    store: &Arc<Store>,
    request: &TeamsSourceRequest,
    observed_on: &str,
) -> ResultOf<JobError> {
    let locks = root.join("non_directory_origin_lock_root");
    std::fs::write(&locks, b"network acquisition cannot pass origin admission")?;
    let fetcher = Arc::new(
        Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            HashMap::new(),
            Vec::new(),
        )?
        .with_origin_locks(locks),
    );
    let region = Arc::new(crate::spawn::Spawner::new());
    let result = teams_arms::team_source(
        SourceRuntime {
            store,
            fetcher: &fetcher,
            region: &region,
        },
        request.jurisdiction.jurisdiction,
        AdapterScope {
            season: request.jurisdiction.season,
            refresh: request.jurisdiction.refresh,
            at: observed_on,
            as_of: request.jurisdiction.history.as_of(),
        },
        &request.source,
    )
    .await;
    let Err(error @ JobError::Terminal { .. }) = result else {
        return Err(format!("held source must fail terminally: {result:?}").into());
    };
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    check!(eq; fetcher.stats().await.requests, 0);
    check!(fetcher.access_conditions().await?.is_empty());
    Ok(error)
}

fn assert_terminal(outcome: &TeamsSourceOutcome, expected: &str) -> TestResult {
    let TeamsSourceOutcome::Terminal { message, progress } = outcome else {
        return Err(format!("expected terminal source outcome: {outcome:?}").into());
    };
    check!(eq; message.as_str(), expected);
    check!(eq; progress.len(), 1);
    let Some(TeamsAttemptProgress::Terminal {
        attempt,
        outcome,
        message,
    }) = progress.first()
    else {
        return Err("terminal attempt progress missing".into());
    };
    check!(eq; *attempt, 1);
    check!(outcome.is_none());
    check!(eq; message.as_str(), expected);
    Ok(())
}

fn assert_no_source_observations(store: &Store) -> TestResult {
    for table in [
        Table::SourceAccess,
        Table::Schools,
        Table::Coaches,
        Table::SourceObservations,
    ] {
        check!(eq; store.walk_table(table)?.rows, 0);
    }
    Ok(())
}

async fn retain_refusal(jurisdiction: UsJurisdiction) -> TestResult {
    let root = tempfile::tempdir()?;
    let store_path = root.path().join("store");
    let mut request = request()?;
    request.source = "bound".into();
    request.jurisdiction.jurisdiction = jurisdiction;
    request.observed_on = "2026-10-14".into();
    let operation = key(&request);
    let reason = census_crawl::registry::acquisition_refusal_for_source("bound")
        .ok_or("bound policy hold missing")?;
    let expected = format!("policy: {reason}");
    let retained = {
        let store = Arc::new(Store::open(&store_path)?);
        let (attempt, observed_on) = reserve(&store, &request)?;
        check!(eq; attempt, 1);
        let error = refused_dispatch(root.path(), &store, &request, &observed_on).await?;
        let ledger::Completion::Settled(outcome) = ledger::finish(
            &store,
            &operation,
            attempt,
            ledger::Attempt::Failed(error),
            &[],
        )?
        else {
            return Err("policy refusal scheduled a retry".into());
        };
        assert_terminal(&outcome, &expected)?;
        assert_no_source_observations(&store)?;
        store.flush()?;
        serde_json::to_value(outcome)?
    };
    request.observed_on = "2026-10-15".into();
    let store = Store::open(&store_path)?;
    let before = store.journal_keys(ledger::PHASE)?;
    let registered = ledger::register(&store, &operation, &request)?;
    let ledger::Admission::Settled(outcome) =
        ledger::begin(&store, &operation, &request, &registered)?
    else {
        return Err("reopened policy refusal became runnable".into());
    };
    assert_terminal(&outcome, &expected)?;
    check!(eq; serde_json::to_value(outcome)?, retained);
    check!(eq; store.journal_keys(ledger::PHASE)?, before);
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    let counts = ledger::durable_attempts(&store)?;
    check!(eq; (counts.reserved, counts.recorded, counts.settled, counts.unknown), (1, 1, 1, 0));
    assert_no_source_observations(&store)
}

#[tokio::test]
async fn gobound_iowa_and_sd_terminal_refusals_survive_reopen_without_retry_or_http_evidence(
) -> TestResult {
    for jurisdiction in [UsJurisdiction::Iowa, UsJurisdiction::SouthDakota] {
        retain_refusal(jurisdiction).await?;
    }
    Ok(())
}
