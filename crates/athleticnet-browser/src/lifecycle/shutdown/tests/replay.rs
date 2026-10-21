use super::{controlled_actor, manager, observe_shutdown, TestResult};
use crate::drain::{DrainAccountingError, DrainReport};
use crate::lifecycle::ActorJoinState;
use crate::{BrowserStartupError, BrowserState};
use tokio::sync::{mpsc, oneshot};

#[tokio::test(start_paused = true)]
async fn typed_accounting_failure_replays_the_actual_report_after_cleanup() -> TestResult {
    let (tx, rx) = mpsc::channel(2);
    let handler = tokio::spawn(async {});
    let (mut actor, _events) = controlled_actor(rx, handler)?;
    actor.region.completed = u64::MAX;
    observe_shutdown(&mut actor);
    let manager = manager(tx, actor);

    let (report, failure) = manager.shutdown().await;

    check!(eq;
        report,
        DrainReport {
            accepted: 1,
            completed: u64::MAX,
            ..DrainReport::default()
        }
    );
    check!(matches!(
        failure,
        Some(BrowserStartupError::DrainAccounting(
            DrainAccountingError::Overflow
        ))
    ));
    let (replayed, failure) = manager.shutdown().await;
    check!(eq; replayed, report);
    check!(matches!(
        failure,
        Some(BrowserStartupError::DrainAccounting(
            DrainAccountingError::Overflow
        ))
    ));
    check!(eq; manager.status().state, BrowserState::Restarting);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_closed_control_channel_does_not_override_the_actual_actor_exit() -> TestResult {
    let (tx, rx) = mpsc::channel(1);
    let handler = tokio::spawn(async {});
    let (mut actor, _events) = controlled_actor(rx, handler)?;
    observe_shutdown(&mut actor);
    actor.draining = true;
    let manager = manager(tx, actor);
    manager.tx.closed().await;
    check!(eq; manager.status().state, BrowserState::Stopped);

    let (report, failure) = manager.shutdown().await;

    check!(failure.is_none());
    check!(eq;
        report,
        DrainReport {
            accepted: 1,
            completed: 1,
            ..DrainReport::default()
        }
    );
    let (replayed, failure) = manager.shutdown().await;
    check!(failure.is_none());
    check!(eq; replayed, report);
    check!(eq; manager.status().state, BrowserState::Stopped);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_missing_actor_exit_is_cached_as_unconfirmed_not_as_a_clean_drain() -> TestResult {
    let (tx, rx) = mpsc::channel(1);
    let (finish, finished) = oneshot::channel();
    let (acknowledge, acknowledged) = oneshot::channel();
    let handler = tokio::spawn(acknowledge_handler_release(finished, acknowledge));
    let (actor, _events) = controlled_actor(rx, handler)?;
    let manager = manager(tx, actor);
    {
        let state = manager.join.lock().await;
        match &*state {
            ActorJoinState::Running(handle) => handle.abort(),
            _ => return Err("the actual actor was not retained".into()),
        }
    }
    finish.send(()).map_err(|_| "the actual handler was lost")?;
    acknowledged.await?;

    let (report, failure) = manager.shutdown().await;

    check!(eq; report, DrainReport::default());
    let failure = failure.ok_or("missing unconfirmed shutdown failure")?;
    check!(
        matches!(&failure, BrowserStartupError::ShutdownFailed(message)
        if message.starts_with("browser actor task join failed: ") && message.contains("cancelled"))
    );
    let (replayed, replay_failure) = manager.shutdown().await;
    check!(eq; replayed, report);
    check!(eq;
        replay_failure
            .ok_or("missing replayed failure")?
            .to_string(),
        failure.to_string()
    );
    check!(eq; manager.status().state, BrowserState::Restarting);
    check!(matches!(
        &*manager.join.lock().await,
        ActorJoinState::Unconfirmed(_)
    ));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_handler_failure_replays_its_actual_report_and_failure() -> TestResult {
    let (tx, rx) = mpsc::channel(1);
    let handler = tokio::spawn(async {});
    let (mut actor, events) = controlled_actor(rx, handler)?;
    observe_shutdown(&mut actor);
    events.send(crate::actor::HandlerEvent::Failed).await?;
    let manager = manager(tx, actor);
    manager.tx.closed().await;

    let (report, failure) = manager.shutdown().await;

    check!(eq;
        report,
        DrainReport {
            accepted: 1,
            completed: 1,
            ..DrainReport::default()
        }
    );
    check!(matches!(failure, Some(BrowserStartupError::HandlerStopped)));
    let (replayed, failure) = manager.shutdown().await;
    check!(eq; replayed, report);
    check!(matches!(failure, Some(BrowserStartupError::HandlerStopped)));
    check!(eq; manager.status().state, BrowserState::Restarting);
    Ok(())
}

async fn acknowledge_handler_release(
    finished: oneshot::Receiver<()>,
    acknowledge: oneshot::Sender<()>,
) {
    assert_eq!(finished.await, Ok(()));
    assert_eq!(acknowledge.send(()), Ok(()));
}
