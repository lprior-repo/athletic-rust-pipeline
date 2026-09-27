
use super::super::SHUTDOWN_TIMEOUT;
use super::Actor;
use crate::drain::{count, DrainReport, Outcome};
use tokio::time::Instant;

impl Actor {
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

    async fn cancel_and_drain(&mut self, deadline: Instant) {
        self.observer_stop.cancel();
        self.drain_observers(deadline).await;
        self.drain_fetch_jobs(deadline).await;
    }

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

    async fn close_pages(&mut self, deadline: Instant) {
        for slot in self.pages.drain(..) {
            match tokio::time::timeout_at(deadline, slot.page.close()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::debug!("page close failed during shutdown: {e}"),
                Err(_) => tracing::warn!("page close timed out during shutdown"),
            }
        }
    }

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
