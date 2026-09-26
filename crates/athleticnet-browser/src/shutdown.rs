//! Draining the browser region.
//!
//! `close_browser` is the actor's whole drain: cancellation first, then a bounded reap of what can
//! still finish on its own, then an abort of what is left, then the process teardown. The counting
//! lives in `cancel_and_drain`, so the certificate is threaded through a body that stays readable,
//! and `teardown_browser` owns the process, so a teardown failure cannot lose finished counts.
//!
//! Every unit leaves a number behind. `accepted` is the set size at the moment the drain looks at
//! it, `aborted` is what the drain itself killed, and a set that is still unresolved when the
//! deadline passes is counted as `timed_out` *and* `remaining` rather than reported as finished.

use super::super::SHUTDOWN_TIMEOUT;
use super::Actor;
use crate::drain::{count, DrainReport, Outcome};
use tokio::time::Instant;

impl Actor {
    /// Drain the region and close the browser.
    ///
    /// Returns the region's certificate and the teardown failure, if any: the counts are finished
    /// before the process is touched, so a browser that refuses to die still leaves an auditable
    /// report.
    pub(super) async fn close_browser(&mut self) -> (DrainReport, Option<anyhow::Error>) {
        let deadline = match self.clock.now_instant().checked_add(SHUTDOWN_TIMEOUT) {
            Some(deadline) => deadline,
            None => self.clock.now_instant(),
        };
        self.cancel_and_drain(deadline).await;
        self.close_pages(deadline).await;
        let failure = self.teardown_browser().await;
        (std::mem::take(&mut self.region), failure)
    }

    /// Cancel admission and reap every task the actor still owns.
    async fn cancel_and_drain(&mut self, deadline: Instant) {
        self.observer_stop.cancel();
        self.drain_observers(deadline).await;
        self.drain_fetch_jobs(deadline).await;
    }

    /// Give the observers a bounded chance to finish, then abort whatever is left.
    ///
    /// An observer that reports an inner error stops the wait: the page is compromised, and the
    /// remaining observers are better killed than waited for.
    async fn drain_observers(&mut self, deadline: Instant) {
        self.region.accept(count(self.observers.len()));
        while !self.observers.is_empty() {
            match tokio::time::timeout_at(deadline, self.observers.join_next()).await {
                Ok(Some(result)) => {
                    let outcome = Outcome::from_join(result);
                    if let Outcome::Err(error) = &outcome {
                        tracing::warn!(%error, "browser page observer failed during cleanup");
                    }
                    let finished = matches!(outcome, Outcome::Ok(_));
                    self.region.record(&outcome);
                    if !finished {
                        break;
                    }
                }
                Ok(None) => return,
                Err(_) => break,
            }
        }
        self.abort_observers(deadline).await;
    }

    /// Abort and join the observers left behind by the bounded join.
    async fn abort_observers(&mut self, deadline: Instant) {
        if self.observers.is_empty() {
            return;
        }
        self.observers.abort_all();
        loop {
            match tokio::time::timeout_at(deadline, self.observers.join_next()).await {
                Ok(Some(result)) => self.region.record_killed(result.map(|_| ())),
                Ok(None) => return,
                Err(_) => break,
            }
        }
        let remaining = count(self.observers.len());
        self.region.abandon(remaining);
        tracing::warn!(
            remaining,
            "browser page observers unresolved at the shutdown deadline"
        );
    }

    /// Abort and reap every outstanding fetch job.
    ///
    /// An in-page JS fetch survives a dropped Rust future, so jobs are terminated explicitly
    /// instead of being given a chance to finish, and the reap loop is bounded so a page that
    /// stopped responding cannot stall the drain.
    async fn drain_fetch_jobs(&mut self, deadline: Instant) {
        self.region.accept(count(self.jobs.len()));
        self.jobs.abort_all();
        loop {
            match tokio::time::timeout_at(deadline, self.jobs.join_next()).await {
                Ok(Some(result)) => self.region.record_killed(result.map(|_| ())),
                Ok(None) => return,
                Err(_) => break,
            }
        }
        let remaining = count(self.jobs.len());
        self.region.abandon(remaining);
        tracing::warn!(
            remaining,
            "browser fetch jobs unresolved at the shutdown deadline"
        );
    }

    /// Close owned pages before waiting the handler — prevents the handler from
    /// holding live Page handles after browser drop.
    ///
    /// Each close is bounded by the drain deadline: a page whose target has stopped answering must
    /// not be able to stall the drain, and a close that does not finish is a page the drain can
    /// only report, not wait for.
    async fn close_pages(&mut self, deadline: Instant) {
        for slot in self.pages.drain(..) {
            match tokio::time::timeout_at(deadline, slot.page.close()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::debug!("page close failed during shutdown: {e}"),
                Err(_) => tracing::warn!("page close timed out during shutdown"),
            }
        }
    }

    /// Take the browser process down and wait for its handler.
    async fn teardown_browser(&mut self) -> Option<anyhow::Error> {
        let Some(browser) = self.browser.take() else {
            return Some(anyhow::anyhow!("browser already closed"));
        };
        let failure = shutdown_browser_process(self.launched, browser).await;
        match self.handler_join.take() {
            Some(handle) => join_browser_handler(handle, failure).await,
            None => failure,
        }
    }
}

/// Close and wait for the browser process.
///
/// Only launched mode owns the process. Attached (loopback) sessions share an
/// existing browser, so closing it would kill the user's own Chrome.
async fn shutdown_browser_process(
    launched: bool,
    mut browser: chromiumoxide::Browser,
) -> Option<anyhow::Error> {
    let mut failure: Option<anyhow::Error> = None;
    if launched {
        match tokio::time::timeout(SHUTDOWN_TIMEOUT, browser.close()).await {
            Ok(Ok(_)) => {}
            Ok(Err(_)) => failure = Some(anyhow::anyhow!("browser close failed")),
            Err(_) => failure = Some(anyhow::anyhow!("browser close timed out")),
        }
        if failure.is_none() {
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, browser.wait()).await {
                Ok(Ok(_)) => {}
                Ok(Err(_)) => failure = Some(anyhow::anyhow!("browser process wait failed")),
                Err(_) => failure = Some(anyhow::anyhow!("browser process wait timed out")),
            }
        }
    } else {
        drop(browser);
    }
    failure
}

/// Wait for the browser handler, aborting it when the shutdown timeout expires.
async fn join_browser_handler(
    mut handle: tokio::task::JoinHandle<()>,
    failure: Option<anyhow::Error>,
) -> Option<anyhow::Error> {
    match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut handle).await {
        Ok(Ok(())) => failure,
        Ok(Err(_)) => Some(anyhow::anyhow!("browser handler panicked")),
        Err(_) => {
            handle.abort();
            if handle.await.is_err() {
                tracing::warn!("browser handler aborted after shutdown timeout");
            }
            Some(anyhow::anyhow!("browser handler wait timed out"))
        }
    }
}
