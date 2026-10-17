use super::super::{actor::Command, BrowserState, SHUTDOWN_TIMEOUT};
use super::status::write_state;
use super::{ActorExit, ActorJoinState, BrowserManager};
use crate::drain::DrainReport;
use crate::lifecycle::error::BrowserStartupError;
use tokio::sync::oneshot;
use tokio::time::Instant;

impl BrowserManager {
    pub async fn shutdown(&self) -> (DrainReport, Option<BrowserStartupError>) {
        let deadline = self.clock.now_instant().checked_add(SHUTDOWN_TIMEOUT);
        let mut state = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, self.join.lock()).await {
                Ok(state) => state,
                Err(_) => return failed_shutdown("browser shutdown timed out"),
            },
            None => match self.join.try_lock() {
                Ok(state) => state,
                Err(_) => return failed_shutdown("browser shutdown deadline overflow"),
            },
        };
        loop {
            let attempt = match &mut *state {
                ActorJoinState::Finished(exit) => {
                    return (exit.report.clone(), exit.failure.clone());
                }
                ActorJoinState::Unconfirmed(error) => {
                    return (DrainReport::default(), Some(error.clone()));
                }
                ActorJoinState::Running(handle) => {
                    self.gate.revoke();
                    write_state(&self.status, BrowserState::Restarting);
                    let deadline = match deadline {
                        Some(deadline) => deadline,
                        None => return failed_shutdown("browser shutdown deadline overflow"),
                    };
                    self.stop_actor(handle, deadline).await
                }
            };
            match attempt {
                JoinAttempt::Exited(exit) => {
                    let status = if exit.failure.is_none() {
                        BrowserState::Stopped
                    } else {
                        BrowserState::Restarting
                    };
                    write_state(&self.status, status);
                    *state = ActorJoinState::Finished(exit);
                }
                JoinAttempt::Unconfirmed(error) => {
                    write_state(&self.status, BrowserState::Restarting);
                    *state = ActorJoinState::Unconfirmed(error);
                }
                JoinAttempt::TimedOut(report) => {
                    return (report, Some(shutdown_failure("browser shutdown timed out")));
                }
            }
        }
    }

    async fn stop_actor(
        &self,
        handle: &mut tokio::task::JoinHandle<ActorExit>,
        deadline: Instant,
    ) -> JoinAttempt {
        let (reply, mut report) = oneshot::channel();
        match tokio::time::timeout_at(deadline, self.tx.send(Command::Shutdown { reply })).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                tracing::debug!("browser shutdown intake closed; joining its actual exit")
            }
            Err(_) => tracing::warn!("browser shutdown send timed out; retaining its actor owner"),
        }
        tokio::task::yield_now().await;
        match tokio::time::timeout_at(deadline, handle).await {
            Ok(Ok(exit)) => JoinAttempt::Exited(exit),
            Ok(Err(error)) => JoinAttempt::Unconfirmed(BrowserStartupError::ShutdownFailed(
                format!("browser actor task join failed: {error}"),
            )),
            Err(_) => JoinAttempt::TimedOut(partial_report(&mut report)),
        }
    }
}

enum JoinAttempt {
    Exited(ActorExit),
    Unconfirmed(BrowserStartupError),
    TimedOut(DrainReport),
}

fn partial_report(report: &mut oneshot::Receiver<DrainReport>) -> DrainReport {
    match report.try_recv() {
        Ok(report) => report,
        Err(error) => {
            tracing::debug!(%error, "browser shutdown has no confirmed drain report yet");
            DrainReport::default()
        }
    }
}

fn shutdown_failure(message: &str) -> BrowserStartupError {
    BrowserStartupError::ShutdownFailed(message.to_owned())
}

fn failed_shutdown(message: &str) -> (DrainReport, Option<BrowserStartupError>) {
    (DrainReport::default(), Some(shutdown_failure(message)))
}

#[cfg(test)]
mod tests;
