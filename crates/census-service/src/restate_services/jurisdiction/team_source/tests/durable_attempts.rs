use super::{key, ledger, request, reserve, TestResult};
use crate::restate_services::JobError;
use census_store::Store;

#[test]
fn attempt_ledger_bridge_reports_reservations_without_recorded_outcomes() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    for expected in 1..=2 {
        let store = Store::open(root.path())?;
        check!(eq; reserve(&store, &request)?.0, expected);
    }
    let store = Store::open(root.path())?;
    let keys = store.journal_keys(ledger::PHASE)?;
    let state = ledger::durable_attempts(&store)?;
    check!(eq; state.ceiling, 3);
    check!(eq; state.sources, 1);
    check!(eq; state.settled, 0);
    check!(eq; state.reserved, 2);
    check!(eq; state.recorded, 0);
    check!(eq; state.unknown, 2);
    check!(eq; store.journal_keys(ledger::PHASE)?, keys, "the bridge writes nothing");
    check!(eq; state, ledger::durable_attempts(&store)?, "the bridge is a pure read");
    Ok(())
}

#[test]
fn attempt_ledger_bridge_separates_recorded_attempts_from_unknown_ones() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    let (first, _) = reserve(&store, &request)?;
    check!(eq; first, 1);
    let (second, _) = reserve(&store, &request)?;
    check!(eq; second, 2);
    let finished = ledger::finish(
        &store,
        &key(&request),
        second,
        (Err(JobError::Transient {
            message: "source read timed out".to_string(),
        }))
        .into(),
        &[],
    )?;
    check!(matches!(
        finished,
        ledger::Completion::Retry(JobError::Transient { .. })
    ));
    let state = ledger::durable_attempts(&store)?;
    check!(eq; state.ceiling, 3);
    check!(eq; state.sources, 1);
    check!(eq; state.settled, 0);
    check!(eq; state.reserved, 2);
    check!(eq; state.recorded, 1);
    check!(eq; state.unknown, 1);
    let (third, _) = reserve(&store, &request)?;
    check!(eq; third, 3, "the bridge neither consumed nor reused an attempt");
    Ok(())
}

#[test]
fn attempt_ledger_bridge_counts_settled_sources_without_reopening_their_attempts() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    let (attempt, _) = reserve(&store, &request)?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        (Err(JobError::Terminal {
            message: "public source access refused".to_string(),
        }))
        .into(),
        &[],
    )?;
    let state = ledger::durable_attempts(&store)?;
    check!(eq; state.ceiling, 3);
    check!(eq; state.sources, 1);
    check!(eq; state.settled, 1);
    check!(eq; state.reserved, 1);
    check!(eq; state.recorded, 1);
    check!(eq; state.unknown, 0);
    let registered = ledger::register(&store, &key(&request), &request)?;
    check!(matches!(
        ledger::begin(&store, &key(&request), &request, &registered)?,
        ledger::Admission::Settled(_)
    ));
    Ok(())
}

#[test]
fn settled_replay_and_queued_cancellation_preserve_durable_attempt_counts() -> TestResult {
    use crate::outcome::{AttemptState, Outcome};
    use crate::restate_services::wire::StageOutcome;
    use crate::spawn::Spawner;
    use std::sync::Arc;
    use std::time::Duration;

    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    {
        let store = Store::open(root.path())?;
        let (attempt, at) = reserve(&store, &request)?;
        ledger::finish(
            &store,
            &operation,
            attempt,
            ledger::Attempt::Completed(StageOutcome {
                records: 7,
                at,
                errors: Vec::new(),
                notes: Vec::new(),
                disposition: census_crawl::CollectionDisposition::Complete,
                unfinished: Vec::new(),
            }),
            &[],
        )?;
        store.flush()?;
    }
    let store = Arc::new(Store::open(root.path())?);
    let before = ledger::durable_attempts(&store)?;
    check!(eq; before, AttemptState { ceiling: 3, sources: 1, settled: 1, reserved: 1, recorded: 1, unknown: 0 });
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let replay_store = Arc::clone(&store);
            let replay_request = request.clone();
            let replay = spawner.blocking(move || {
                ledger::status(&replay_store, &operation, Some(&replay_request))
            }).await;
            let Outcome::Ok(Some(outcome)) = replay else {
                return Err(format!("settled replay did not return its outcome: {replay:?}").into());
            };
            check!(eq; serde_json::to_value(outcome)?, serde_json::to_value(
                ledger::status(&store, &key(&request), Some(&request))?
                    .ok_or("settled outcome disappeared")?
            )?);
            let (started, began) = tokio::sync::oneshot::channel();
            let (release, released) = std::sync::mpsc::channel();
            let running = spawner.blocking(move || {
                let _ = started.send(());
                released.recv_timeout(Duration::from_secs(5))
            });
            tokio::pin!(running);
            tokio::select! {
                outcome = &mut running => return Err(format!("effect ended early: {outcome:?}").into()),
                result = began => result?,
            }
            let queued_store = Arc::clone(&store);
            let queued_request = request.clone();
            let queued = spawner.blocking(move || {
                reserve(&queued_store, &queued_request).map_err(|error| error.to_string())
            });
            tokio::pin!(queued);
            tokio::select! {
                biased;
                outcome = &mut queued => return Err(format!("queued attempt ran: {outcome:?}").into()),
                () = std::future::ready(()) => {}
            }
            let interrupted =
                tokio::time::timeout(Duration::from_millis(5), spawner.drain(Duration::ZERO)).await;
            release.send(())?;
            check!(interrupted.is_err(), "started blocking work must stay owned");
            let (running, cancelled, report) =
                tokio::join!(running, queued, spawner.drain(Duration::from_secs(5)));
            check!(eq; running, Outcome::Ok(()));
            check!(eq; cancelled, Outcome::Cancelled);
            let report = report?;
            check!(eq; report.accepted, 3);
            check!(eq; report.completed, 2);
            check!(eq; report.cancelled, 1);
            check!(eq; report.aborted, 0);
            check!(eq; report.panicked, 0);
            check!(eq; report.failed, 0);
            check!(eq; report.remaining, 0);
            check!(eq; report.still_running, 0);
            check!(eq; report.timed_out, 2);
            check!(eq; report.disjoint_total(), report.accepted);
            let attempts = ledger::durable_attempts(&store)?;
            check!(eq; attempts, before);
            println!("replayed and cancelled drain certificate: {report:?} attempts={attempts:?}");
            Ok(())
        })
}
