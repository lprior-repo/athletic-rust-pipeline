use super::super::{key, ledger, tests::request, TeamsSource};
use crate::admission::WORK_CLASS_STAGE;
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::wire::TeamsSourceOutcome;
use crate::restate_services::{JobError, Jobs};
use crate::spawn::{Spawner, TaskReport};
use census_store::{clock::SystemClock, Store};
use std::sync::Arc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "current_thread")]
async fn teams_source_ledger_work_runs_on_the_blocking_region() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        crate::restate_services::CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(&store), Arc::new(crate::spawn::Spawner::new())),
    );
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let source = TeamsSource::new(owner, jobs);
    let request = Arc::new(request()?);
    let operation = key(&request);
    let held = region.coordinator().reserve(WORK_CLASS_STAGE).await?;
    let mut pending = Box::pin(source.ledger_status(operation.clone(), Some(Arc::clone(&request))));
    check!(futures::poll!(pending.as_mut()).is_pending());
    drop(held);
    check!(pending.await?.is_none());
    let admitted = Arc::new(
        source
            .admit(&operation)
            .await
            .map_err(crate::restate_services::tests::sdk_error)?,
    );
    let registered = Arc::new(ledger::register(&store, &operation, &request)?);
    let baseline = ledger::baseline(&store, &operation)?;
    let ledger::AdmissionReplay::Reserved {
        attempt,
        observed_on,
    } = source
        .begin_at_attempt(
            operation.clone(),
            request,
            registered,
            Arc::clone(&admitted),
            baseline,
        )
        .await?
    else {
        return Err("source settled without an attempt".into());
    };
    check!(eq; (attempt, observed_on), (1, "2026-10-02".to_string()));
    let ledger::Completion::Settled(outcome) = source
        .finish_attempt(
            operation.clone(),
            attempt,
            ledger::Attempt::Failed(JobError::Terminal {
                message: "source access refused".to_string(),
            }),
            Arc::clone(&admitted),
            Vec::new(),
        )
        .await?
    else {
        return Err("terminal source failure became retryable".into());
    };
    check!(
        matches!(&outcome, TeamsSourceOutcome::Terminal { message, .. } if message == "source access refused")
    );
    drop(admitted);
    check!(eq; region.coordinator().remaining().slots.stages, 1);
    check!(eq; serde_json::to_value(source.ledger_status(operation.clone(), None).await?)?, serde_json::to_value(Some(outcome))?);
    let inspection = source.ledger_inspection_observed(operation.clone()).await?;
    check!(eq; serde_json::to_value(inspection)?, serde_json::to_value(ledger::inspection(&store, &operation, None)?)?);
    check!(eq; region.drain(Duration::from_secs(5)).await?, TaskReport {
        accepted: 5,
        completed: 5,
        ..TaskReport::default()
    });
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn offloaded_ledger_errors_retain_the_synchronous_error_and_release_admission() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        crate::restate_services::CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(&store), Arc::new(crate::spawn::Spawner::new())),
    );
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let source = TeamsSource::new(owner, jobs);
    let request = Arc::new(request()?);
    let operation = key(&request);
    let registered = Arc::new(ledger::register(&store, &operation, &request)?);
    let baseline = ledger::baseline(&store, &operation)?;
    store.journal_done(
        ledger::PHASE,
        &format!("{operation}/identity"),
        &serde_json::json!(17),
    )?;
    let Err(JobError::Terminal { message: expected }) =
        ledger::status(&store, &operation, Some(&request))
    else {
        return Err("corrupt identity did not produce the expected terminal error".into());
    };
    let result = source
        .ledger_status(operation.clone(), Some(Arc::clone(&request)))
        .await;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == expected));
    let result = source.ledger_inspection_observed(operation.clone()).await;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == expected));
    let admitted = Arc::new(
        source
            .admit(&operation)
            .await
            .map_err(crate::restate_services::tests::sdk_error)?,
    );
    let result = source
        .begin_at_attempt(
            operation.clone(),
            request,
            registered,
            Arc::clone(&admitted),
            baseline,
        )
        .await;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == expected));
    let result = source
        .finish_attempt(
            operation,
            1,
            ledger::Attempt::Failed(JobError::Transient {
                message: "physical attempt failed".to_string(),
            }),
            admitted,
            Vec::new(),
        )
        .await;
    check!(matches!(result, Err(JobError::Terminal { message }) if message == expected));
    check!(eq; region.coordinator().remaining().slots.stages, 1);
    check!(eq; region.drain(Duration::from_secs(5)).await?, TaskReport {
        accepted: 4,
        completed: 4,
        failed: 4,
        ..TaskReport::default()
    });
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn inspection_reads_run_on_the_control_class_while_the_work_plane_is_saturated() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(1));
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        crate::restate_services::CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(&store), Arc::new(crate::spawn::Spawner::new())),
    );
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let source = TeamsSource::new(owner, jobs);
    let request = Arc::new(request()?);
    let operation = key(&request);
    let registered = Arc::new(ledger::register(&store, &operation, &request)?);
    let baseline = ledger::baseline(&store, &operation)?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } = source
        .begin_at_attempt(
            operation.clone(),
            Arc::clone(&request),
            registered,
            Arc::new(
                source
                    .admit(&operation)
                    .await
                    .map_err(crate::restate_services::tests::sdk_error)?,
            ),
            baseline,
        )
        .await?
    else {
        return Err("source settled without an attempt".into());
    };
    check!(eq; attempt, 1);
    let held = region.coordinator().reserve(WORK_CLASS_STAGE).await?;
    let mut work = Box::pin(source.ledger_status(operation.clone(), Some(Arc::clone(&request))));
    check!(futures::poll!(work.as_mut()).is_pending());
    let observed = source
        .ledger_status_observed(operation.clone(), None)
        .await?;
    check!(eq; serde_json::to_value(&observed)?, serde_json::to_value(ledger::status(&store, &operation, None)?)?);
    let inspection = source.ledger_inspection_observed(operation.clone()).await?;
    check!(eq; serde_json::to_value(&inspection)?, serde_json::to_value(ledger::inspection(&store, &operation, None)?)?);
    drop(held);
    check!(work.await?.is_none());
    check!(eq; region.drain(Duration::from_secs(5)).await?, TaskReport {
        accepted: 4,
        completed: 4,
        ..TaskReport::default()
    });
    Ok(())
}
