use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use athleticnet_browser::clock::Clock as EngineClock;
use athleticnet_browser::drain::DrainReport;
use athleticnet_browser::request::{same_origin, RequestSpec};
use athleticnet_browser::{BrowserManager, BrowserOutcome, BrowserSettings, BrowserStatus};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

pub const SESSION_KEY: &str = "profile-0";

#[derive(Clone)]
pub struct BrowserSession {
    settings: Arc<BrowserSettings>,
    clock: Arc<dyn EngineClock>,
    manager: Arc<AsyncMutex<Option<Arc<BrowserManager>>>>,
    starting: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
}

impl BrowserSession {
    pub fn new(settings: BrowserSettings, clock: Arc<dyn EngineClock>) -> Self {
        Self {
            settings: Arc::new(settings),
            clock,
            manager: Arc::new(AsyncMutex::new(None)),
            starting: Arc::new(AtomicBool::new(false)),
            epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    fn begin_launch(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::SeqCst).saturating_add(1)
    }

    fn launch_superseded(&self, epoch: u64) -> bool {
        self.epoch.load(Ordering::SeqCst) != epoch
    }

    fn record_stop(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }

    fn foreign_origin_refusal(&self, request: &RequestSpec) -> Option<String> {
        let origin = &self.settings.source_origin;
        if same_origin(&request.url, origin) {
            return None;
        }
        Some(format!(
            "the browser lane is bound to {origin} and refuses {}: a request must share the \
             configured source origin's scheme, host and port, so a foreign scheme or host never \
             reaches a page, a capture or a receipt",
            request.url
        ))
    }

    async fn live(&self) -> Result<Arc<BrowserManager>, HandlerError> {
        self.manager.lock().await.clone().ok_or_else(|| {
            TerminalError::new(
                "the browser lane is not started in this endpoint process; \
                 start it before fetching through it",
            )
            .into()
        })
    }

    async fn reading(&self, key: &str) -> BrowserSessionStatus {
        let Some(manager) = self.manager.lock().await.clone() else {
            return BrowserSessionStatus {
                key: key.to_string(),
                running: false,
                status: None,
                error: None,
            };
        };
        let (status, error) = match manager.inspect().await {
            Ok(status) => (Some(status), None),
            Err(error) => (None, Some(error.to_string())),
        };
        BrowserSessionStatus {
            key: key.to_string(),
            running: manager.is_alive(),
            status,
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionStatus {
    pub key: String,
    pub running: bool,
    pub status: Option<BrowserStatus>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionDrain {
    pub key: String,
    pub drain: DrainCounts,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrainCounts {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub remaining: u64,
}

impl From<DrainReport> for DrainCounts {
    fn from(report: DrainReport) -> Self {
        Self {
            accepted: report.accepted,
            completed: report.completed,
            cancelled: report.cancelled,
            timed_out: report.timed_out,
            aborted: report.aborted,
            panicked: report.panicked,
            remaining: report.remaining,
        }
    }
}

#[object(
    journal_retention = "90 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(max_attempts = 1, on_max_attempts = "pause")
)]
impl BrowserSession {
    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key(), url = %request.url))]
    async fn fetch(
        &self,
        ctx: SharedObjectContext<'_>,
        Json(request): Json<RequestSpec>,
    ) -> Result<Json<BrowserOutcome>, HandlerError> {
        if let Some(refusal) = self.foreign_origin_refusal(&request) {
            return Err(TerminalError::new(refusal).into());
        }
        let manager = self.live().await?;
        let outcome = ctx
            .run(move || async move { Ok::<_, HandlerError>(Json(manager.fetch(request).await)) })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserSessionStatus>, HandlerError> {
        Ok(Json(self.reading(ctx.key()).await))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserSessionStatus>, HandlerError> {
        let key = ctx.key().to_string();
        let epoch = self.begin_launch();
        if let Some(manager) = self.manager.lock().await.clone() {
            if manager.is_alive() {
                return Err(TerminalError::new(
                    "the browser lane is already started in this endpoint process: one process \
                     owns the profile and a second manager is a corruption path, not a second \
                     lane; stop the lane before starting it again",
                )
                .into());
            }
        }
        if self
            .starting
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(TerminalError::new(
                "a browser launch is already in progress in this endpoint process: one process \
                 owns the profile and a second manager is a corruption path, not a second lane",
            )
            .into());
        }
        let settings = Arc::clone(&self.settings);
        let clock = Arc::clone(&self.clock);
        let launched = match settings.cdp_endpoint.clone() {
            Some(endpoint) => BrowserManager::connect(endpoint, (*settings).clone(), clock).await,
            None => BrowserManager::launch((*settings).clone(), clock).await,
        };
        let manager = launched.map_err(|error| {
            self.starting.store(false, Ordering::SeqCst);
            TerminalError::new(format!("the browser lane could not start: {error}"))
        })?;
        if self.launch_superseded(epoch) {
            self.starting.store(false, Ordering::SeqCst);
            let (_, error) = manager.shutdown().await;
            let detail = error.map_or_else(String::new, |error| {
                format!("; the launched manager reported: {error}")
            });
            return Err(TerminalError::new(format!(
                "the browser launch was superseded by a stop before it could be installed, so the \
                 launched process was shut down instead of installed{detail}"
            ))
            .into());
        }
        *self.manager.lock().await = Some(Arc::new(manager));
        self.starting.store(false, Ordering::SeqCst);
        Ok(Json(self.reading(&key).await))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn stop(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserSessionDrain>, HandlerError> {
        let key = ctx.key().to_string();
        self.record_stop();
        let Some(manager) = self.manager.lock().await.take() else {
            return Ok(Json(BrowserSessionDrain {
                key,
                drain: DrainCounts::default(),
                error: None,
            }));
        };
        let (report, error) = manager.shutdown().await;
        Ok(Json(BrowserSessionDrain {
            key,
            drain: report.into(),
            error: error.map(|error| error.to_string()),
        }))
    }
}

#[cfg(test)]
#[path = "browser_session_tests.rs"]
mod tests;
