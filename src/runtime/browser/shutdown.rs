use super::super::SHUTDOWN_TIMEOUT;
use super::Actor;

impl Actor {
    pub(super) async fn close_browser(&mut self) -> anyhow::Result<()> {
        self.observer_stop.cancel();
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
        if observer_failure {
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
        // Also abort and join all outstanding fetch jobs — JS fetches survive
        // dropped Rust futures, so we must explicitly terminate them.
        self.jobs.abort_all();
        loop {
            let Some(result) = self.jobs.join_next().await else {
                break;
            };
            if result.is_err() {
                tracing::warn!("browser fetch job aborted during cleanup");
            }
        }
        // Close owned pages before waiting the handler — prevents handler
        // from holding live Page handles after browser drop.
        for slot in self.pages.drain(..) {
            match slot.page.close().await {
                Ok(()) => {}
                Err(e) => tracing::debug!("page close failed during shutdown: {e}"),
            }
        }
        let mut browser = self
            .browser
            .take()
            .ok_or_else(|| anyhow::anyhow!("browser already closed"))?;
        let mut failure: Option<anyhow::Error> = None;
        // Only close and wait the browser process for launched mode.
        // Attached (loopback) sessions share an existing browser;
        // closing it would kill the user's own Chrome.
        if self.launched {
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
        if let Some(handle) = self.handler_join.take() {
            let mut handle = handle;
            match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut handle).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => failure = Some(anyhow::anyhow!("browser handler panicked")),
                Err(_) => {
                    handle.abort();
                    if handle.await.is_err() {
                        tracing::warn!("browser handler aborted after shutdown timeout");
                    }
                    failure = Some(anyhow::anyhow!("browser handler wait timed out"));
                }
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
