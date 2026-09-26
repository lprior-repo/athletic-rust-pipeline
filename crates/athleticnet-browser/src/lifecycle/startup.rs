//! Constructing a live manager.
//!
//! Both constructors build the same shape — a command channel, the handler-event channel with its
//! own pump task, the shared status/cooldown/gate handles, the actor's bootstrap navigation, and
//! the actor task — and differ only in how the browser is obtained: `launch` spawns the configured
//! executable, `connect` attaches to an existing CDP endpoint. A failed bootstrap drains the
//! partially built manager through `shutdown` instead of dropping a live actor.

use super::super::{
    actor::{Actor, ActorHandles, BrowserConnection, Command, HandlerEvent},
    gate::ProfileGate,
    pool, BrowserSettings, BrowserState, BrowserStatus,
};
use super::BrowserManager;
use crate::clock::Clock;
use chromiumoxide::{handler::HandlerConfig, Browser, BrowserConfig, Handler};
use futures::StreamExt;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use tracing::Instrument;

const QUEUE_MULTIPLIER: usize = 4;
const HANDLER_QUEUE: usize = 256;

impl BrowserManager {
    #[tracing::instrument(skip_all, fields(tabs = settings.tabs, headed = settings.headed))]
    pub async fn launch(settings: BrowserSettings, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        settings.validate()?;
        pool::prepare_profile(&settings)?;
        let config = browser_config(&settings)?;
        let (browser, handler) = Browser::launch(config).await?;
        finish_startup(browser, handler, true, settings, clock).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn connect(
        cdp_url: url::Url,
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> anyhow::Result<Self> {
        settings.validate()?;
        let handler_config = HandlerConfig {
            request_timeout: settings.request_timeout,
            ..Default::default()
        };
        let (browser, handler) =
            Browser::connect_with_config(cdp_url.as_str(), handler_config).await?;
        finish_startup(browser, handler, false, settings, clock).await
    }
}

/// Build the manager around a live browser and bootstrap it.
///
/// Both constructors converge here: the command channel, the handler pump, the shared
/// status/cooldown/gate handles, the actor and its bootstrap are identical, and `launched` is the
/// only difference that outlives construction.
async fn finish_startup(
    browser: Browser,
    handler: Handler,
    launched: bool,
    settings: BrowserSettings,
    clock: Arc<dyn Clock>,
) -> anyhow::Result<BrowserManager> {
    let (tx, rx) = mpsc::channel(settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1));
    let (handler_tx, handler_rx) = mpsc::channel(HANDLER_QUEUE);
    let handler_join = spawn_handler(handler, handler_tx);
    let status = Arc::new(RwLock::new(BrowserStatus {
        state: BrowserState::Restarting,
        active_requests: 0,
        tabs: settings.tabs,
        cooldown_ms: 0,
    }));
    let cooldown_until = Arc::new(Mutex::new(None));
    let gate = Arc::new(ProfileGate::new());
    let tabs = settings.tabs;
    let actor = Actor::new(
        BrowserConnection { browser, launched },
        settings,
        rx,
        (handler_rx, handler_join),
        ActorHandles {
            status: status.clone(),
            cooldown_until: cooldown_until.clone(),
            gate: gate.clone(),
            clock: clock.clone(),
        },
    );
    let span = tracing::info_span!("browser.actor", tabs, launched);
    let join = tokio::spawn(actor.run().instrument(span));
    let manager = BrowserManager {
        tx,
        status,
        cooldown_until,
        gate,
        join: Arc::new(AsyncMutex::new(Some(join))),
        clock,
    };
    finish_bootstrap(manager).await
}

/// Bootstrap a freshly built manager and drain it when the bootstrap fails.
///
/// A failed bootstrap MUST drain the partially built manager through `shutdown` instead of dropping
/// a live actor: dropping aborts the actor task mid-step, while the drain revokes the gate, closes
/// the browser and counts what it took down. The bootstrap error is the one returned either way.
async fn finish_bootstrap(manager: BrowserManager) -> anyhow::Result<BrowserManager> {
    let (reply, result) = oneshot::channel();
    manager
        .tx
        .send(Command::Bootstrap { reply })
        .await
        .map_err(|_| anyhow::anyhow!("browser actor stopped during startup"))?;
    let bootstrap_result = match result.await {
        Ok(value) => value,
        Err(_) => Err(anyhow::anyhow!("browser actor stopped during startup")),
    };
    if let Err(error) = bootstrap_result {
        let (report, failure) = manager.shutdown().await;
        tracing::warn!(
            ?report,
            ?failure,
            "browser drained after a bootstrap failure"
        );
        return Err(error);
    }
    Ok(manager)
}

/// Spawn the CDP handler pump with its own span.
///
/// `tokio::spawn` does not carry the caller's span into the task, so the pump is instrumented
/// explicitly; the actor task is instrumented by its own `#[instrument]`.
fn spawn_handler(
    handler: Handler,
    events: mpsc::Sender<HandlerEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_handler(handler, events).instrument(tracing::info_span!("browser.handler")))
}

fn browser_config(settings: &BrowserSettings) -> anyhow::Result<BrowserConfig> {
    let builder = BrowserConfig::builder()
        .chrome_executable(&settings.executable)
        .user_data_dir(&settings.profile_dir)
        .request_timeout(settings.request_timeout);
    let builder = if settings.headed {
        builder.with_head()
    } else {
        builder
    };
    builder
        .build()
        .map_err(|_| anyhow::anyhow!("invalid browser launch configuration"))
}

async fn run_handler(mut handler: Handler, events: mpsc::Sender<HandlerEvent>) {
    while let Some(result) = handler.next().await {
        if result.is_err() {
            if let Err(e) = events.send(HandlerEvent::Failed).await {
                tracing::debug!("handler event send failed: {e}");
            }
            return;
        }
    }
    if let Err(e) = events.send(HandlerEvent::Failed).await {
        tracing::debug!("handler event send failed: {e}");
    }
}
