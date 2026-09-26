//! `BrowserSession`: the one headed profile, served.
//!
//! The census never opens a browser. `census_crawl::net::bridge::BrowserLane` posts a
//! [`RequestSpec`] to this object's `fetch` handler over the ingress client and reads a
//! [`BrowserOutcome`] back, so the census keeps the evidence shape it already has while exactly one
//! process owns the profile (§26-§28). The spec and the answer are the engine's own wire types, byte
//! for byte what the census mirrors: the two sides are proved to agree by the committed fixtures
//! under `fixtures/wire/`, not by a second definition here.
//!
//! # What this object does not do
//!
//! * **It does not classify.** Page capture, challenge detection and the `Retry-After` reading all
//!   happen in the engine and travel in the answer. A reader here that scanned a body again could
//!   disagree with the gate the transport already revoked.
//! * **It does not retry.** `fetch` attempts once: the census's durable layer owns retries
//!   (ADR-002), and its client leaves a verdict that says "another invocation is worth making" as an
//!   error for that layer to replay. The object's own retry policy is therefore one attempt.
//! * **It does not work around a challenge.** A challenged profile reports `HumanRequired` as data;
//!   that is a stop condition for the source, not an obstacle to route around.
//!
//! # Rules an operator and a deployment follow
//!
//! * **One manager per profile directory.** The engine validates the directory but does not lock it,
//!   so a second manager is a corruption path rather than a second lane. Run one `census-serve`
//!   endpoint against a store; the key is [`SESSION_KEY`] because there is one profile.
//! * **A restart does not restart the lane.** The live manager is process state, not journaled
//!   state: after the endpoint restarts, `status` reports not running and `fetch` refuses until
//!   `start` is called again. That is deliberate - launching a headed browser is an operator
//!   decision, and a replay must never do it twice.
//! * **`stop` reports the engine's own drain.** [`DrainReport`] counters come back whole, so a run's
//!   §42 accounting includes the lane's tasks rather than assuming they stopped.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use athleticnet_browser::clock::Clock as EngineClock;
use athleticnet_browser::drain::DrainReport;
use athleticnet_browser::request::RequestSpec;
use athleticnet_browser::{BrowserManager, BrowserOutcome, BrowserSettings, BrowserStatus};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

/// The key the endpoint serves: one profile, so one key. The census's client addresses the same one.
pub const SESSION_KEY: &str = "profile-0";

/// `BrowserSession`: one headed profile behind one object key.
#[derive(Clone)]
pub struct BrowserSession {
    settings: Arc<BrowserSettings>,
    clock: Arc<dyn EngineClock>,
    /// Process state, deliberately not journaled. A replay must not launch a second browser, so the
    /// live handle lives here and `start` refuses when one is already held.
    manager: Arc<AsyncMutex<Option<Arc<BrowserManager>>>>,
    /// Process state, deliberately not journaled: it guards the launch window itself, so two
    /// concurrent `start` calls cannot both pass the live-handle check and put two managers on
    /// the one profile.
    starting: Arc<AtomicBool>,
}

impl BrowserSession {
    pub fn new(settings: BrowserSettings, clock: Arc<dyn EngineClock>) -> Self {
        Self {
            settings: Arc::new(settings),
            clock,
            manager: Arc::new(AsyncMutex::new(None)),
            starting: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The live manager, or the terminal refusal that says how to get one.
    async fn live(&self) -> Result<Arc<BrowserManager>, HandlerError> {
        self.manager.lock().await.clone().ok_or_else(|| {
            TerminalError::new(
                "the browser lane is not started in this endpoint process; \
                 start it before fetching through it",
            )
            .into()
        })
    }

    /// The operator-facing reading: whether a manager is live, and what the engine says about it.
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

/// What an operator reads back: `running` is the process fact, `status` the engine's reading of the
/// profile, and `error` the reason there is no reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionStatus {
    pub key: String,
    pub running: bool,
    pub status: Option<BrowserStatus>,
    pub error: Option<String>,
}

/// What `stop` drained: the engine's own counters, plus the error the drain itself reported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionDrain {
    pub key: String,
    pub drain: DrainCounts,
    pub error: Option<String>,
}

/// [`DrainReport`] on the wire, field for field: the engine's report is not itself serializable, and
/// a summary here that dropped a counter would hide exactly the tasks a §42 accounting looks for.
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

// The lane holds no completion of its own: `fetch` is a read, `start` and `stop` are operator
// actions, and the profile outlives every invocation.
#[object(
    journal_retention = "90 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(max_attempts = 1, on_max_attempts = "pause")
)]
impl BrowserSession {
    /// The one handler the census calls. Shared: the engine's tab pool is the concurrency control,
    /// and serializing invocations here would cap the lane at one page at a time.
    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key(), url = %request.url))]
    async fn fetch(
        &self,
        ctx: SharedObjectContext<'_>,
        Json(request): Json<RequestSpec>,
    ) -> Result<Json<BrowserOutcome>, HandlerError> {
        let manager = self.live().await?;
        // Journaled: the capture is the evidence a receipt cites, so a replay returns what the
        // transport actually saw rather than a second fetch that might see something else. `Json`
        // is the bridge to the SDK's own serialization traits, which is what `run` journals with;
        // the bytes are still `serde_json`'s encoding of the engine's own wire type.
        let outcome = ctx
            .run(move || async move { Ok::<_, HandlerError>(Json(manager.fetch(request).await)) })
            // Single-attempt run policy (ADR-002): the census's durable layer owns retries, so
            // `fetch` never retries where the journal cannot see it.
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    /// The profile's reading. Never an error for an unstarted lane: "not running" is a state an
    /// operator asks for on purpose.
    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn status(
        &self,
        ctx: SharedObjectContext<'_>,
    ) -> Result<Json<BrowserSessionStatus>, HandlerError> {
        Ok(Json(self.reading(ctx.key()).await))
    }

    /// Launch the profile, or refuse when one is already live or launching.
    ///
    /// A second `start` while a manager is held or a launch is in flight is a terminal error,
    /// never a silent no-op: one process owns the profile and a second manager is a corruption
    /// path, not a second lane.
    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserSessionStatus>, HandlerError> {
        let key = ctx.key().to_string();
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
        // Deliberately not journaled: a browser is process state, not bytes, so process state
        // stays process state. The `is_alive` refusal above, the `starting` claim below and this
        // assignment are what keep a retried or replayed `start` from putting a second manager on
        // the same profile directory, which the engine documents as a corruption path rather than
        // a second lane.
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
        *self.manager.lock().await = Some(Arc::new(manager));
        self.starting.store(false, Ordering::SeqCst);
        Ok(Json(self.reading(&key).await))
    }

    /// Drain the profile and report what it took down.
    #[handler]
    #[tracing::instrument(skip_all, fields(profile = ctx.key()))]
    async fn stop(
        &self,
        ctx: ObjectContext<'_>,
    ) -> Result<Json<BrowserSessionDrain>, HandlerError> {
        let key = ctx.key().to_string();
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
