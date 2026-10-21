use super::{controlled_actor, manager, observe_shutdown, TestResult};
use crate::drain::DrainReport;
use crate::lifecycle::ActorJoinState;
use crate::{BrowserStartupError, BrowserState};
use tokio::sync::{mpsc, oneshot};

#[tokio::test(start_paused = true)]
async fn a_manager_deadline_retains_the_actual_actor_for_a_supervised_reap() -> TestResult {
    let (tx, rx) = mpsc::channel(2);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    let (mut actor, _events) = controlled_actor(rx, handler)?;
    observe_shutdown(&mut actor);
    let manager = manager(tx, actor);

    let (_, failure) = manager.shutdown().await;

    check!(
        matches!(failure, Some(BrowserStartupError::ShutdownFailed(message))
        if message == "browser shutdown timed out")
    );
    check!(eq; manager.inspect().await?.state, BrowserState::Restarting);
    let state = manager.join.lock().await;
    check!(matches!(&*state, ActorJoinState::Running(handle) if !handle.is_finished()));
    drop(state);
    finish.send(()).map_err(|_| "actual handler was aborted")?;
    let (report, failure) = manager.shutdown().await;
    check!(eq;
        report,
        DrainReport {
            accepted: 1,
            completed: 1,
            ..DrainReport::default()
        }
    );
    check!(
        matches!(failure, Some(BrowserStartupError::ShutdownFailed(message))
        if message == "browser handler wait timed out")
    );
    let (replayed, failure) = manager.shutdown().await;
    check!(eq; replayed, report);
    check!(
        matches!(failure, Some(BrowserStartupError::ShutdownFailed(message))
        if message == "browser handler wait timed out")
    );
    check!(eq; manager.status().state, BrowserState::Restarting);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn public_shutdown_after_caller_cancellation_returns_the_actual_drain() -> TestResult {
    let (tx, rx) = mpsc::channel(2);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    let (mut actor, _events) = controlled_actor(rx, handler)?;
    observe_shutdown(&mut actor);
    let manager = manager(tx, actor);
    {
        let stopping = manager.shutdown();
        tokio::pin!(stopping);
        tokio::select! {
            biased;
            result = &mut stopping => return Err(format!("shutdown completed before the actual handler: {result:?}").into()),
            _ = tokio::task::yield_now() => {}
        }
    }
    let state = manager.join.lock().await;
    check!(matches!(&*state, ActorJoinState::Running(handle) if !handle.is_finished()));
    drop(state);
    finish
        .send(())
        .map_err(|_| "the retained handler was aborted")?;

    let (report, failure) = manager.shutdown().await;

    if let Some(failure) = failure {
        return Err(failure.into());
    }
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
async fn concurrent_shutdown_calls_replay_the_same_actual_drain() -> TestResult {
    let (tx, rx) = mpsc::channel(4);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    let (mut actor, _events) = controlled_actor(rx, handler)?;
    observe_shutdown(&mut actor);
    let manager = manager(tx, actor);
    let shutdown1 = manager.shutdown();
    let shutdown2 = manager.shutdown();
    tokio::pin!(shutdown1);
    tokio::pin!(shutdown2);

    tokio::select! {
        biased;
        result = &mut shutdown1 => return Err(format!("shutdown1 completed early: {result:?}").into()),
        result = &mut shutdown2 => return Err(format!("shutdown2 completed early: {result:?}").into()),
        _ = tokio::task::yield_now() => {}
    }
    finish
        .send(())
        .map_err(|_| "the actual handler was aborted")?;
    let ((report1, failure1), (report2, failure2)) = tokio::join!(shutdown1, shutdown2);

    check!(failure1.is_none());
    check!(failure2.is_none());
    check!(eq;
        report1,
        DrainReport {
            accepted: 1,
            completed: 1,
            ..DrainReport::default()
        }
    );
    check!(eq; report2, report1);
    check!(eq; manager.status().state, BrowserState::Stopped);
    Ok(())
}
