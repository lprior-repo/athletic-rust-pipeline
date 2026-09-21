//! The runtime's own lifecycle: acquire a browser, run bounded work, drain.
//!
//! [`super::Runtime`] is declared in the parent module and implemented here, because this is the
//! half that manages live things: the one browser manager per endpoint, the CPU permit that
//! bounds blocking work, and the drain that certifies what each unit did. Construction and the
//! plain accessors stay in `runtime.rs`; everything that can hold a resource across an `.await`
//! lives here, where the lock discipline is visible in one place.

use super::browser::{BrowserManager, BrowserSettings};
use super::drain::{DrainReport, Outcome, DRAIN_TIMEOUT};
use super::Runtime;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

impl Runtime {
    /// Return a running browser manager, rebuilding a dead one.
    ///
    /// A manager whose actor task has exited (its command channel closed, or a
    /// terminal `Stopped` status) can never serve another command; replacing the
    /// cached handle is the only way back, so a long run recovers without a
    /// worker restart.
    ///
    /// Nothing here holds the slot guard across an await: a launch is a multi-second process
    /// spawn, so the guard is taken for the cached read and again, briefly, for the install.
    pub(crate) async fn ensure_browser(&self) -> Result<Arc<BrowserManager>> {
        if let Some(existing) = self.cached_manager().await {
            return Ok(existing);
        }
        let settings = self.config.browser_settings();
        let manager = Arc::new(self.build_manager(settings).await?);
        self.install_manager(manager).await
    }

    /// The cached manager, when it can still serve a command.
    async fn cached_manager(&self) -> Option<Arc<BrowserManager>> {
        let cached = self.browser.read().await.clone();
        match cached {
            Some(existing) if existing.is_alive() => Some(existing),
            Some(_) => {
                tracing::warn!("cached browser manager is not running; rebuilding it");
                None
            }
            None => None,
        }
    }

    /// Connect or launch, outside every guard.
    async fn build_manager(&self, settings: BrowserSettings) -> Result<BrowserManager> {
        let clock = self.clock();
        match settings.cdp_endpoint.clone() {
            Some(endpoint) => {
                match BrowserManager::connect(endpoint, settings.clone(), clock.clone()).await {
                    Ok(manager) => Ok(manager),
                    Err(error) => {
                        tracing::warn!(
                            "cdp endpoint unreachable ({error}); launching a managed browser"
                        );
                        BrowserManager::launch(settings, clock).await
                    }
                }
            }
            None => BrowserManager::launch(settings, clock).await,
        }
    }

    /// Publish `manager` as the one manager for the configured endpoint.
    ///
    /// Only this write guard installs, so two concurrent builders cannot both win: the loser is
    /// dropped here, and dropping a manager aborts its actor task, which is what keeps the
    /// invariant at one live manager per endpoint. A build that finishes after the drain closed
    /// admission is dropped rather than installed, so no browser outlives the drain.
    async fn install_manager(&self, manager: Arc<BrowserManager>) -> Result<Arc<BrowserManager>> {
        let mut slot = self.browser.write().await;
        if self.tasks.is_closed() {
            anyhow::bail!("worker is draining");
        }
        if let Some(existing) = slot.as_ref() {
            if existing.is_alive() {
                return Ok(existing.clone());
            }
        }
        *slot = Some(manager.clone());
        Ok(manager)
    }

    /// Run one blocking action on the CPU pool, counted as one drained unit.
    pub async fn blocking<T, F>(&self, action: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        let token = self.tasks.token();
        if self.tasks.is_closed() {
            anyhow::bail!("worker is draining");
        }
        let permit = self.cpu.clone().acquire_owned().await?;
        let counts = Arc::clone(&self.drain_counts);
        let joined = tokio::task::spawn_blocking(move || {
            // The tracker token is released after the guard records the terminal state, so a drain
            // that sees an empty tracker has already seen every terminal count.
            let (_token, _permit) = (token, permit);
            let mut unit = counts.accept();
            let result = action();
            unit.complete();
            result
        })
        .await;
        match Outcome::from_join(joined) {
            Outcome::Ok(value) => Ok(value),
            Outcome::Err(error) => Err(error),
            Outcome::Panicked => Err(anyhow::anyhow!("bounded worker action panicked")),
            Outcome::Cancelled => Err(anyhow::anyhow!("bounded worker action was cancelled")),
            Outcome::Timeout => Err(anyhow::anyhow!("bounded worker action timed out")),
        }
    }

    /// Drain every region this runtime owns and certify what each unit did.
    pub async fn drain(&self) -> DrainReport {
        self.drain_within(DRAIN_TIMEOUT).await
    }

    /// Bounded form of [`Runtime::drain`], so the caller can pick the budget.
    ///
    /// Admission closes before anything is drained — a unit admitted mid-drain is a unit the
    /// certificate cannot account for — the browser region is drained, and the blocking pool is
    /// awaited under one absolute deadline. A hard failure is logged here and stays visible in the
    /// certificate (`panicked`, `timed_out`, `remaining`) instead of being returned: the report is
    /// what the caller acts on, and one of its counts is the failure.
    pub(crate) async fn drain_within(&self, timeout: Duration) -> DrainReport {
        let deadline = match self.clock.now_instant().checked_add(timeout) {
            Some(deadline) => deadline,
            // A budget the platform clock cannot represent must not panic: fall back to "now",
            // which is the same immediate-residual path a reached deadline takes.
            None => self.clock.now_instant(),
        };
        self.cpu.close();
        self.tasks.close();
        let mut report = DrainReport::default();
        self.drain_browser(&mut report, deadline).await;
        self.drain_blocking_pool(&mut report, deadline).await;
        tracing::info!(?report, "runtime drained");
        report
    }

    /// Shut the browser region down and fold its own certificate into `report`.
    async fn drain_browser(&self, report: &mut DrainReport, deadline: tokio::time::Instant) {
        let Some(manager) = self.browser.write().await.take() else {
            return;
        };
        report.accept(1);
        let drain = tokio::time::timeout_at(deadline, manager.shutdown()).await;
        match drain {
            Ok((region, failure)) => {
                // The region finished and said what it took down: the counts stand even when the
                // teardown failed, because the failure is the one thing the certificate cannot
                // express as a unit.
                if let Some(error) = failure {
                    tracing::error!(%error, "browser region drained with a teardown failure");
                }
                report.complete(1);
                report.merge(&region);
            }
            Err(_) => {
                report.time_out(1);
                tracing::warn!("browser region did not finish inside the drain budget");
            }
        }
    }

    /// Wait for the blocking pool and fold its counters into `report`.
    async fn drain_blocking_pool(&self, report: &mut DrainReport, deadline: tokio::time::Instant) {
        let drained = tokio::time::timeout_at(deadline, self.tasks.wait())
            .await
            .is_ok();
        let mut pool = self.drain_counts.snapshot();
        if !drained {
            let open = pool.remaining;
            pool.time_out(open);
            tracing::warn!(
                remaining = open,
                "blocking workers are still running at the drain deadline"
            );
        }
        report.merge(&pool);
    }
}
