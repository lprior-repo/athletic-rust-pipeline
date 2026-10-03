use super::super::{admission, TeamsSource};
use super::{key, ledger, request, TestResult};
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::{blocking, JobError, Jobs};
use crate::spawn::{Spawner, TaskReport};
use census_store::{clock::SystemClock, Store};
use serde_json::{json, Value};
use std::error::Error;
use std::sync::{mpsc::SyncSender, Arc};
use std::time::Duration;
use tokio::sync::{oneshot, Semaphore};

const WAIT: Duration = Duration::from_secs(5);

struct Fixture {
    _root: tempfile::TempDir,
    store: Arc<Store>,
    source: TeamsSource,
    region: Arc<Spawner>,
    load: Arc<Semaphore>,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let store = Arc::new(Store::open(root.path())?);
    let region = Arc::new(Spawner::with_capacity(2));
    let load = Arc::new(Semaphore::new(2));
    let owner = JurisdictionCensus::new(Arc::clone(&store), Arc::new(SystemClock), None);
    let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&load), Arc::clone(&region));
    Ok(Fixture {
        _root: root,
        store,
        source: TeamsSource::new(owner, jobs),
        region,
        load,
    })
}

struct SuspendedRegistration {
    release: SyncSender<()>,
    completed: oneshot::Receiver<ledger::Identity>,
}

async fn cancel_suspended_registration(
    fixture: &Fixture,
    request: Arc<TeamsSourceRequest>,
) -> Result<SuspendedRegistration, Box<dyn Error>> {
    let operation = key(&request);
    let admission = Arc::new(
        fixture
            .source
            .admit(&operation)
            .await
            .map_err(|error| std::io::Error::other(error.to_string()))?,
    );
    let worker = admission::registration_worker(
        Arc::clone(&admission),
        Arc::clone(&fixture.store),
        operation,
        request,
    );
    let region = Arc::clone(&fixture.region);
    let (started, began) = oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let (completed, completion) = oneshot::channel();
    let mut caller = Box::pin(async move {
        let _admission = admission;
        blocking(region, move || {
            started.send(()).map_err(|()| JobError::Terminal {
                message: "registration start observer closed".to_string(),
            })?;
            released
                .recv_timeout(WAIT)
                .map_err(|error| JobError::Terminal {
                    message: format!("registration release failed: {error}"),
                })?;
            let registered = worker()?;
            completed.send(registered).map_err(|_| JobError::Terminal {
                message: "registration completion observer closed".to_string(),
            })?;
            Ok::<(), JobError>(())
        })
        .await
    });
    tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut caller => Err::<(), Box<dyn Error>>(format!("registration caller completed before release: {result:?}").into()),
            result = began => { result?; Ok(()) },
        }
    })
    .await??;
    drop(caller);
    Ok(SuspendedRegistration {
        release,
        completed: completion,
    })
}

fn assert_no_registration(
    store: &Store,
    operation: &str,
    request: &TeamsSourceRequest,
) -> TestResult {
    check!(eq; store.journal_keys(ledger::PHASE)?,
    std::collections::HashSet::new());
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/identity"))?,
    None);
    check!(eq; serde_json::to_value(ledger::inspection(store, operation, Some(request))?)?,
    json!({"status": "unsettled", "progress": []}));
    Ok(())
}

fn assert_original_reservation(
    store: &Store,
    operation: &str,
    request: &TeamsSourceRequest,
    registered: &ledger::Identity,
    expected: &Value,
) -> TestResult {
    check!(eq; serde_json::to_value(registered)?, *expected);
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/identity"))?,
    Some(expected.clone()));
    let ledger::Admission::Reserved {
        attempt,
        observed_on,
    } = ledger::begin(store, operation, request, registered)?
    else {
        return Err("registered source unexpectedly settled".into());
    };
    check!(eq; (attempt, observed_on), (1, "2026-10-02".to_string()));
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/attempt/1/reserved"))?,
    Some(json!({"attempt": 1, "observed_on": "2026-10-02"})));
    check!(eq; store.journal_payload(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?,
    None);
    check!(eq; serde_json::to_value(ledger::inspection(store, operation, Some(request))?)?,
    json!({"status": "unsettled", "progress": [{"status": "unknown", "attempt": 1}]}));
    Ok(())
}

#[test]
fn cancelled_registration_caller_keeps_same_key_closed_until_worker_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let fixture = fixture()?;
            let first = Arc::new(request()?);
            let operation = key(&first);
            let mut next = request()?;
            next.observed_on = "2026-10-03".to_string();
            let next = Arc::new(next);
            check!(eq; key(&next), operation);
            let suspended = cancel_suspended_registration(&fixture, first).await?;
            check!(eq; fixture.load.available_permits(), 1);
            let mut waiting = Box::pin(fixture.source.admit(&operation));
            check!(futures::poll!(waiting.as_mut()).is_pending());
            check!(eq; fixture.load.available_permits(), 0);
            assert_no_registration(&fixture.store, &operation, &next)?;
            suspended.release.send(())?;
            let registered = tokio::time::timeout(WAIT, suspended.completed).await??;
            check!(eq; registered.observed_on, "2026-10-02");
            let expected = serde_json::to_value(&registered)?;
            let admission = Arc::new(
                tokio::time::timeout(WAIT, waiting)
                    .await?
                    .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            check!(eq; fixture.load.available_permits(), 1);
            let replay = blocking(
                Arc::clone(&fixture.region),
                admission::registration_worker(
                    Arc::clone(&admission),
                    Arc::clone(&fixture.store),
                    operation.clone(),
                    Arc::clone(&next),
                ),
            )
            .await?;
            assert_original_reservation(&fixture.store, &operation, &next, &replay, &expected)?;
            drop(admission);
            check!(eq; fixture.load.available_permits(), 2);
            let report = fixture.region.drain(WAIT).await?;
            check!(eq; report,
            TaskReport {
                accepted: 2,
                completed: 2,
                cancelled: 0,
                timed_out: 0,
                remaining: 0,
                aborted: 0,
                panicked: 0
            });
            Ok(())
        })
}
