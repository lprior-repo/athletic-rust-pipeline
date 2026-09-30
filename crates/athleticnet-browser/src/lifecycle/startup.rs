use super::super::{
    actor::{Actor, ActorHandles, BrowserConnection, Command, HandlerEvent},
    gate::ProfileGate,
    pool, BrowserSettings, BrowserState, BrowserStatus,
};
use super::error::BrowserStartupError;
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
    pub async fn launch(
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, BrowserStartupError> {
        settings.validate()?;
        pool::prepare_profile(&settings).map_err(|_| BrowserStartupError::ProfileNotDirectory)?;
        let config = browser_config(&settings)?;
        let (browser, handler) = Browser::launch(config)
            .await
            .map_err(|error| BrowserStartupError::LaunchFailed(error.to_string()))?;
        finish_startup(browser, handler, true, settings, clock).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn connect(
        cdp_url: url::Url,
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, BrowserStartupError> {
        settings.validate()?;
        let handler_config = HandlerConfig {
            request_timeout: settings.request_timeout,
            ..Default::default()
        };
        let (browser, handler) = Browser::connect_with_config(cdp_url.as_str(), handler_config)
            .await
            .map_err(|error| BrowserStartupError::ConnectFailed(error.to_string()))?;
        finish_startup(browser, handler, false, settings, clock).await
    }
}

async fn finish_startup(
    browser: Browser,
    handler: Handler,
    launched: bool,
    settings: BrowserSettings,
    clock: Arc<dyn Clock>,
) -> Result<BrowserManager, BrowserStartupError> {
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

async fn finish_bootstrap(manager: BrowserManager) -> Result<BrowserManager, BrowserStartupError> {
    let (reply, result) = oneshot::channel();
    manager
        .tx
        .send(Command::Bootstrap { reply })
        .await
        .map_err(|_| BrowserStartupError::ActorStopped)?;
    let bootstrap_result = match result.await {
        Ok(value) => value,
        Err(_) => Err(BrowserStartupError::ActorStopped),
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

fn spawn_handler(
    handler: Handler,
    events: mpsc::Sender<HandlerEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_handler(handler, events).instrument(tracing::info_span!("browser.handler")))
}

fn browser_config(settings: &BrowserSettings) -> Result<BrowserConfig, BrowserStartupError> {
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
        .map_err(|_| BrowserStartupError::ConfigBuildFailed)
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
