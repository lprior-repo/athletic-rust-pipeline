use super::super::lifecycle::ActorExit;
use super::super::SHUTDOWN_TIMEOUT;
use super::Actor;
use crate::drain::{DrainReport, Outcome};
use crate::lifecycle::error::BrowserStartupError;
use tokio::time::Instant;

#[path = "shutdown/process.rs"]
mod process;
#[path = "shutdown/quarantine.rs"]
mod quarantine;
use process::{CloseAttempt, ClosePhase};

impl Actor {
    pub(super) fn finish_shutdown(&mut self, exit: &ActorExit) {
        if let Some(reply) = self.shutdown_reply.take() {
            if reply.send(exit.report.clone()).is_err() {
                tracing::debug!("browser shutdown report receiver dropped");
            }
        }
        self.set_state(if exit.failure.is_some() {
            crate::BrowserState::Restarting
        } else {
            crate::BrowserState::Stopped
        });
    }

    pub(super) async fn handler_stopped(&mut self) -> ActorExit {
        self.gate.revoke();
        self.shutdown.cancel();
        self.reject_pending(crate::BrowserError::Unavailable);
        let (report, failure) = self.close_browser().await;
        tracing::warn!(?report, "browser region drained after a handler failure");
        let failure = match failure {
            Some(error @ BrowserStartupError::DrainAccounting(_)) => error,
            Some(error) => BrowserStartupError::HandlerStoppedCleanupFailed(error.to_string()),
            None => BrowserStartupError::HandlerStopped,
        };
        let exit = ActorExit {
            report,
            failure: Some(failure),
        };
        self.finish_shutdown(&exit);
        exit
    }

    pub(super) async fn close_browser(&mut self) -> (DrainReport, Option<BrowserStartupError>) {
        let now = self.clock.now_instant();
        let (deadline, mut failure) = match now.checked_add(SHUTDOWN_TIMEOUT) {
            Some(deadline) => (deadline, None),
            None => (
                now,
                Some(BrowserStartupError::ShutdownFailed(
                    "browser cleanup deadline overflow".to_owned(),
                )),
            ),
        };
        self.cancel_and_drain(deadline).await;
        self.close_pages(deadline).await;
        let mut attempt = self.teardown_browser(ClosePhase::Graceful).await;
        loop {
            match attempt {
                CloseAttempt::Reaped => {
                    let failure = self
                        .accounting_error
                        .map(BrowserStartupError::DrainAccounting)
                        .or(failure);
                    return (std::mem::take(&mut self.region), failure);
                }
                CloseAttempt::StillOwned(error) => {
                    self.report_cleanup_failure(&error);
                    if failure.is_none() {
                        failure = Some(error);
                    }
                    self.await_reap_command().await;
                    attempt = self.teardown_browser(ClosePhase::ReapOnly).await;
                }
            }
        }
    }

    async fn cancel_and_drain(&mut self, deadline: Instant) {
        self.observer_stop.cancel();
        self.drain_observers(deadline).await;
        self.drain_fetch_jobs().await;
    }

    async fn drain_observers(&mut self, deadline: Instant) {
        self.accept_drain_tasks(self.observers.len());
        while !self.observers.is_empty() {
            match tokio::time::timeout_at(deadline, self.observers.join_next()).await {
                Ok(Some(result)) => {
                    let outcome = Outcome::from_join(result);
                    if let Outcome::Err(error) = &outcome {
                        tracing::warn!(%error, "browser page observer failed during cleanup");
                    }
                    let finished = matches!(outcome, Outcome::Ok(_));
                    let accounting = self.region.record(&outcome);
                    self.record_accounting(accounting);
                    if !finished {
                        break;
                    }
                }
                Ok(None) => return,
                Err(_) => break,
            }
        }
        self.abort_observers().await;
    }

    async fn abort_observers(&mut self) {
        if self.observers.is_empty() {
            return;
        }
        self.observers.abort_all();
        while let Some(result) = self.observers.join_next().await {
            let accounting = self.region.record_killed(result.map(|_| ()));
            self.record_accounting(accounting);
        }
    }

    async fn drain_fetch_jobs(&mut self) {
        self.accept_drain_tasks(self.jobs.len());
        self.jobs.abort_all();
        while let Some(result) = self.jobs.join_next().await {
            let accounting = self.region.record_killed(result.map(|_| ()));
            self.record_accounting(accounting);
        }
    }

    async fn close_pages(&mut self, deadline: Instant) {
        for slot in &self.pages {
            match tokio::time::timeout_at(deadline, slot.page.clone().close()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::debug!("page close failed during shutdown: {e}"),
                Err(_) => tracing::warn!("page close timed out during shutdown"),
            }
        }
    }

    async fn teardown_browser(&mut self, phase: ClosePhase) -> CloseAttempt {
        if self.ownership == crate::actor::BrowserOwnership::Launched {
            let Some(child) = self.child.as_mut() else {
                return CloseAttempt::StillOwned(BrowserStartupError::ShutdownFailed(
                    "launched browser lost its retained child".to_owned(),
                ));
            };
            if let CloseAttempt::StillOwned(error) =
                process::close(self.browser.as_mut(), child, phase).await
            {
                return CloseAttempt::StillOwned(error);
            }
        }
        self.handler_stop.cancel();
        if let Some(handle) = self.handler_join.as_mut() {
            match process::join_handler(handle).await {
                process::HandlerAttempt::Finished(result) => {
                    drop(self.handler_join.take());
                    if let Err(error) = result {
                        return CloseAttempt::StillOwned(error);
                    }
                }
                process::HandlerAttempt::StillOwned(error) => {
                    return CloseAttempt::StillOwned(error);
                }
            }
        }
        drop(self.browser.take());
        drop(self.child.take());
        self.pages.clear();
        CloseAttempt::Reaped
    }
}

#[cfg(test)]
#[path = "shutdown/tests.rs"]
mod tests;
