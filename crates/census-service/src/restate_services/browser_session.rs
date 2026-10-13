use std::sync::Arc;

use athleticnet_browser::clock::Clock as EngineClock;
use athleticnet_browser::drain::DrainReport;
use athleticnet_browser::request::{same_origin, RequestSpec};
use athleticnet_browser::{BrowserManager, BrowserOutcome, BrowserSettings, BrowserStatus};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

pub const SESSION_KEY: &str = "profile-0";

fn sanitized_url(url: &url::Url) -> String {
    let mut clean = url.clone();
    let _ = clean.set_username("");
    let _ = clean.set_password(None);
    clean.set_query(None);
    clean.set_fragment(None);
    clean.to_string()
}

#[derive(Default)]
struct Lane {
    epoch: u64,
    manager: Option<Arc<BrowserManager>>,
}

#[derive(Clone)]
pub struct BrowserSession {
    settings: Arc<BrowserSettings>,
    clock: Arc<dyn EngineClock>,
    lane: Arc<AsyncMutex<Lane>>,
    admission: Arc<AsyncMutex<()>>,
}

impl BrowserSession {
    pub fn new(settings: BrowserSettings, clock: Arc<dyn EngineClock>) -> Self {
        Self {
            settings: Arc::new(settings),
            clock,
            lane: Arc::new(AsyncMutex::new(Lane::default())),
            admission: Arc::new(AsyncMutex::new(())),
        }
    }

    async fn launch_admission(&self) -> OwnedMutexGuard<()> {
        Arc::clone(&self.admission).lock_owned().await
    }

    async fn begin_launch(&self) -> Result<u64, HandlerError> {
        let mut lane = self.lane.lock().await;
        if let Some(manager) = &lane.manager {
            if manager.is_alive() {
                return Err(TerminalError::new(
                    "the browser lane is already started in this endpoint process: one process \
                     owns the profile and a second manager is a corruption path, not a second \
                     lane; stop the lane before starting it again",
                )
                .into());
            }
        }
        lane.manager = None;
        lane.epoch = lane.epoch.saturating_add(1);
        Ok(lane.epoch)
    }

    async fn record_stop(&self) -> Option<Arc<BrowserManager>> {
        let mut lane = self.lane.lock().await;
        lane.epoch = lane.epoch.saturating_add(1);
        lane.manager.take()
    }

    fn foreign_origin_refusal(&self, request: &RequestSpec) -> Option<String> {
        if !request.url.username().is_empty() || request.url.password().is_some() {
            return Some(format!(
                "the browser lane refuses a credentialed request: userinfo never reaches a page, \
                 a capture or a receipt ({})",
                sanitized_url(&request.url)
            ));
        }
        let origin = &self.settings.source_origin;
        if same_origin(&request.url, origin) {
            return None;
        }
        Some(format!(
            "the browser lane is bound to {origin} and refuses {}: a request must share the \
             configured source origin's scheme, host and port, so a foreign scheme or host never \
             reaches a page, a capture or a receipt",
            sanitized_url(&request.url)
        ))
    }

    async fn live(&self) -> Result<Arc<BrowserManager>, HandlerError> {
        self.lane.lock().await.manager.clone().ok_or_else(|| {
            TerminalError::new(
                "the browser lane is not started in this endpoint process; \
                 start it before fetching through it",
            )
            .into()
        })
    }

    async fn reading(&self, key: &str) -> BrowserSessionStatus {
        let Some(manager) = self.lane.lock().await.manager.clone() else {
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
    #[tracing::instrument(skip_all, fields(profile = ctx.key(), url = %sanitized_url(&request.url)))]
    async fn fetch(
        &self,
        ctx: SharedObjectContext<'_>,
        Json(request): Json<RequestSpec>,
    ) -> Result<Json<BrowserOutcome>, HandlerError> {
        if let Some(refusal) = self.foreign_origin_refusal(&request) {
            return Err(TerminalError::new(refusal).into());
        }
        let outcome = ctx
            .run(move || async move {
                let manager = self.live().await?;
                Ok::<_, HandlerError>(Json(manager.fetch(request).await))
            })
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
        let _admission = self.launch_admission().await;
        let epoch = self.begin_launch().await?;
        let settings = Arc::clone(&self.settings);
        let clock = Arc::clone(&self.clock);
        let launched = match settings.cdp_endpoint.clone() {
            Some(endpoint) => BrowserManager::connect(endpoint, (*settings).clone(), clock).await,
            None => BrowserManager::launch((*settings).clone(), clock).await,
        };
        let manager = launched.map_err(|error| {
            TerminalError::new(format!("the browser lane could not start: {error}"))
        })?;
        let mut lane = self.lane.lock().await;
        if lane.epoch != epoch {
            drop(lane);
            let (report, error) = manager.shutdown().await;
            let detail = error.map_or_else(String::new, |error| {
                format!("; the launched manager reported: {error}")
            });
            return Err(TerminalError::new(format!(
                "the browser launch was superseded by a stop before it could be installed, so the \
                 launched process was shut down instead of installed with drain {:?}{detail}",
                DrainCounts::from(report)
            ))
            .into());
        }
        lane.manager = Some(Arc::new(manager));
        drop(lane);
        Ok(Json(self.reading(&key).await))
    }

    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn stop(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserSessionDrain>, HandlerError> {
        let key = ctx.key().to_string();
        let Some(manager) = self.record_stop().await else {
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
