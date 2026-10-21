use super::TestResult;
use crate::actor::{Actor, Command};
use crate::drain::DrainReport;
use crate::{BrowserStartupError, BrowserState};
use tokio::sync::{mpsc, oneshot};

#[tokio::test(start_paused = true)]
async fn handler_failure_cleanup_answers_the_supervisors_actual_reap_command() -> TestResult {
    let (tx, rx) = mpsc::channel(2);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    let (reply, first_report) = oneshot::channel();
    let mut actor = Actor::draining_for_test(rx, handler, reply)?;
    actor.draining = false;
    let status = actor.status.clone();

    let join = tokio::spawn(actor.run());
    check!(eq; first_report.await?, DrainReport::default());
    check!(!join.is_finished());
    finish.send(()).map_err(|_| "handler was not retained")?;
    let (reply, result) = oneshot::channel();
    tx.send(Command::Shutdown { reply }).await?;

    check!(eq; result.await?, DrainReport::default());
    let exit = join.await?;
    check!(
        matches!(exit.failure, Some(BrowserStartupError::HandlerStoppedCleanupFailed(message))
        if message == "browser shutdown failed: browser handler wait timed out")
    );
    check!(eq;
        status.read().map_err(|_| "status poisoned")?.state,
        BrowserState::Restarting
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn losing_the_supervisor_sender_still_joins_the_same_retained_handler() -> TestResult {
    let (tx, rx) = mpsc::channel(1);
    let (finish, finished) = oneshot::channel();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    let (reply, first_report) = oneshot::channel();
    let actor = Actor::draining_for_test(rx, handler, reply)?;

    let join = tokio::spawn(actor.run());
    check!(eq; first_report.await?, DrainReport::default());
    finish
        .send(())
        .map_err(|_| "handler was aborted before supervisor loss")?;
    drop(tx);

    let exit = join.await?;
    check!(
        matches!(exit.failure, Some(BrowserStartupError::ShutdownFailed(message))
        if message == "browser handler wait timed out")
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_failed_handler_join_cannot_become_a_clean_drain_on_reap_retry() -> TestResult {
    let (tx, rx) = mpsc::channel(1);
    let (_finish, finished) = oneshot::channel::<()>();
    let handler = tokio::spawn(super::wait_for_handler_release(finished));
    handler.abort();
    let (reply, first_report) = oneshot::channel();
    let actor = Actor::draining_for_test(rx, handler, reply)?;

    let join = tokio::spawn(actor.run());
    check!(eq; first_report.await?, DrainReport::default());
    let (reply, result) = oneshot::channel();
    tx.send(Command::Shutdown { reply }).await?;

    check!(eq; result.await?, DrainReport::default());
    let exit = join.await?;
    check!(
        matches!(exit.failure, Some(BrowserStartupError::ShutdownFailed(message))
        if message.starts_with("browser handler join failed: ") && message.contains("cancelled"))
    );
    Ok(())
}
