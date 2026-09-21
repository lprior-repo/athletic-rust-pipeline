use super::super::SHUTDOWN_TIMEOUT;
use super::Actor;

impl Actor {
    pub(super) async fn close_browser(&mut self) -> anyhow::Result<()> {
        self.observer_stop.cancel();
        if self.join_observers().await {
            self.abort_observers().await;
        }
        self.drain_fetch_jobs().await;
        self.close_pages().await;
        let browser = self
            .browser
            .take()
            .ok_or_else(|| anyhow::anyhow!("browser already closed"))?;
        let failure = shutdown_browser_process(self.launched, browser).await;
        let failure = match self.handler_join.take() {
            Some(handle) => join_browser_handler(handle, failure).await,
            None => failure,
        };
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Join every observer under the shutdown timeout.
    ///
    /// Returns true when an observer failed or the join timed out, which tells
    /// the caller to abort whatever is left.
    async fn join_observers(&mut self) -> bool {
        let mut observer_failure = false;
        while !self.observers.is_empty() {
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, self.observers.join_next()).await {
                Ok(Some(Ok(Ok(())))) | Ok(None) => {}
                Ok(Some(Ok(Err(_)))) | Ok(Some(Err(_))) | Err(_) => {
                    observer_failure = true;
                    break;
                }
            }
        }
        observer_failure
    }

    /// Abort and join the observers left behind by a failed join.
    async fn abort_observers(&mut self) {
        self.observers.abort_all();
        loop {
            let Some(result) = self.observers.join_next().await else {
                break;
            };
            if result.is_err() {
                tracing::warn!("browser page observer aborted during cleanup");
            }
        }
    }

    /// Abort and join all outstanding fetch jobs — JS fetches survive dropped
    /// Rust futures, so they must be terminated explicitly.
    async fn drain_fetch_jobs(&mut self) {
        self.jobs.abort_all();
        loop {
            let Some(result) = self.jobs.join_next().await else {
                break;
            };
            if result.is_err() {
                tracing::warn!("browser fetch job aborted during cleanup");
            }
        }
    }

    /// Close owned pages before waiting the handler — prevents the handler from
    /// holding live Page handles after browser drop.
    async fn close_pages(&mut self) {
        for slot in self.pages.drain(..) {
            match slot.page.close().await {
                Ok(()) => {}
                Err(e) => tracing::debug!("page close failed during shutdown: {e}"),
            }
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
        // Attached mode: do not close or wait the external browser.
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
