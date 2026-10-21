use super::TestResult;
use crate::actor::{Actor, Command};
use crate::drain::{DrainAccountingError, DrainReport};
use crate::pool::Pending;
use crate::request::{RequestAction, RequestSpec};
use crate::{BrowserError, BrowserOutcome, BrowserStartupError, BrowserState};
use tokio::sync::{mpsc, oneshot};

fn request() -> Result<RequestSpec, url::ParseError> {
    Ok(RequestSpec {
        url: url::Url::parse("https://www.athletic.net/team/1")?,
        semantic_url: "https://www.athletic.net/team/1".to_owned(),
        action: RequestAction::Fetch {},
        headers: Vec::new(),
    })
}

#[tokio::test(start_paused = true)]
async fn pending_accounting_overflow_refuses_intake_and_finishes_the_owed_reply() -> TestResult {
    for (accepted, cancelled, expected_accepted) in [(u64::MAX, 0, u64::MAX), (0, u64::MAX, 1)] {
        let (_tx, rx) = mpsc::channel(1);
        let handler = tokio::spawn(async {});
        let (reply, report) = oneshot::channel();
        let mut actor = Actor::draining_for_test(rx, handler, reply)?;
        actor.region.accepted = accepted;
        actor.region.cancelled = cancelled;
        let (reply, response) = oneshot::channel();
        actor.pending.push_back(Pending {
            request: request()?,
            reply,
        });
        let reply = actor
            .shutdown_reply
            .take()
            .ok_or("missing shutdown reply")?;

        actor.command(Some(Command::Shutdown { reply })).await;
        check!(!actor.gate.is_ready());
        check!(eq;
            response.await?,
            BrowserOutcome::failed(BrowserError::Shutdown)
        );
        let status = actor.status.clone();
        let join = tokio::spawn(actor.run());

        let reported = report.await?;
        check!(eq;
            reported,
            DrainReport {
                accepted: expected_accepted,
                cancelled,
                ..DrainReport::default()
            }
        );
        let exit = join.await?;
        check!(eq; exit.report, reported);
        check!(matches!(
            exit.failure,
            Some(BrowserStartupError::DrainAccounting(
                DrainAccountingError::Overflow
            ))
        ));
        check!(eq;
            status.read().map_err(|_| "status poisoned")?.state,
            BrowserState::Restarting
        );
    }
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn admitted_observers_are_joined_even_when_accepting_their_count_overflows() -> TestResult {
    let (_tx, rx) = mpsc::channel(1);
    let handler = tokio::spawn(async {});
    let (reply, report) = oneshot::channel();
    let mut actor = Actor::draining_for_test(rx, handler, reply)?;
    actor.region.accepted = u64::MAX;
    let (observed, observation) = oneshot::channel();
    actor
        .observers
        .spawn(async move { observed.send(()).map_err(|_| BrowserError::Transport) });
    let reply = actor
        .shutdown_reply
        .take()
        .ok_or("missing shutdown reply")?;

    actor.command(Some(Command::Shutdown { reply })).await;
    let join = tokio::spawn(actor.run());

    observation.await?;
    let reported = report.await?;
    check!(eq;
        reported,
        DrainReport {
            accepted: u64::MAX,
            completed: 1,
            ..DrainReport::default()
        }
    );
    let exit = join.await?;
    check!(eq; exit.report, reported);
    check!(matches!(
        exit.failure,
        Some(BrowserStartupError::DrainAccounting(
            DrainAccountingError::Overflow
        ))
    ));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_terminal_counter_overflow_survives_completed_handler_cleanup() -> TestResult {
    let (_tx, rx) = mpsc::channel(1);
    let (handler_finished, handler_result) = oneshot::channel();
    let handler = tokio::spawn(async move {
        super::notify_handler_completed(handler_finished);
    });
    let (reply, report) = oneshot::channel();
    let mut actor = Actor::draining_for_test(rx, handler, reply)?;
    actor.region.completed = u64::MAX;
    actor.observers.spawn(async { Ok(()) });
    let reply = actor
        .shutdown_reply
        .take()
        .ok_or("missing shutdown reply")?;

    actor.command(Some(Command::Shutdown { reply })).await;
    let join = tokio::spawn(actor.run());

    let reported = report.await?;
    check!(eq;
        reported,
        DrainReport {
            accepted: 1,
            completed: u64::MAX,
            ..DrainReport::default()
        }
    );
    let exit = join.await?;
    check!(eq; exit.report, reported);
    check!(matches!(
        exit.failure,
        Some(BrowserStartupError::DrainAccounting(
            DrainAccountingError::Overflow
        ))
    ));
    handler_result.await?;
    Ok(())
}
