use super::*;
use crate::restate_services::{CaptureExecution, Jobs};
use crate::spawn::Spawner;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{clock::SystemClock, Store, Table};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn failed_fetcher_setup_does_not_reserve_or_settle_a_source_attempt() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = Arc::new(super::super::tests::request()?);
    let operation = super::super::key(&request);
    {
        let store = Arc::new(Store::open(root.path())?);
        let mut invalid = SourceAccessCondition::new(
            "controlled setup failure",
            "example.org",
            AccessBlockKind::HumanRequired,
            403,
            "2026-10-18T10:00:00Z",
            "operator required",
        );
        invalid.cooldown_until = Some("malformed".into());
        store.replace_many(Table::SourceAccess, &[invalid])?;
        let region = Arc::new(Spawner::new());
        let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
        let owner = crate::restate_services::jurisdiction::JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::LegacyFresh,
            jobs.clone(),
        );
        let source = TeamsSource::new(owner, jobs);
        let identity = Arc::new(ledger::register(&store, &operation, &request)?);
        for _ in 0..2 {
            let result = source
                .execute_attempt(
                    operation.clone(),
                    Arc::clone(&request),
                    Arc::clone(&identity),
                    0,
                    ExecutionLedger::Original,
                )
                .await;
            check!(result.is_err());
            let state = ledger::durable_attempts(&store)?;
            check!(eq; (state.reserved, state.recorded, state.unknown, state.settled), (0, 0, 0, 0));
        }
        let drained = region.drain(Duration::from_secs(5)).await?;
        check!(eq; drained.remaining, 0);
        store.flush()?;
    }
    let store = Store::open(root.path())?;
    let state = ledger::durable_attempts(&store)?;
    check!(eq; (state.reserved, state.recorded, state.unknown, state.settled), (0, 0, 0, 0));
    check!(ledger::status(&store, &operation, Some(&request))?.is_none());
    Ok(())
}
