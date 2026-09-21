//! Constructing a live manager.
//!
//! Both constructors build the same shape — a command channel, the handler-event channel with its
//! own pump task, the shared status/cooldown/gate handles, the actor's bootstrap navigation, and
//! the actor task — and differ only in how the browser is obtained: `launch` spawns the configured
//! executable, `connect` attaches to an existing CDP endpoint. A failed bootstrap drains the
//! partially built manager through `shutdown` instead of dropping a live actor.

use super::super::{
    actor::{Actor, BrowserConnection, Command, HandlerEvent},
    gate::ProfileGate,
    pool, BrowserSettings, BrowserState, BrowserStatus,
};
use super::BrowserManager;
use chromiumoxide::{handler::HandlerConfig, Browser, BrowserConfig, Handler};
use futures::StreamExt;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

const QUEUE_MULTIPLIER: usize = 4;
const HANDLER_QUEUE: usize = 256;

impl BrowserManager {
    #[tracing::instrument(skip_all, fields(tabs = settings.tabs, headed = settings.headed))]
    pub(crate) async fn launch(settings: BrowserSettings) -> anyhow::Result<Self> {
        settings.validate()?;
        pool::prepare_profile(&settings)?;
        let config = browser_config(&settings)?;
        let (browser, handler) = Browser::launch(config).await?;
        let (tx, rx) = mpsc::channel(settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1));
        let (handler_tx, handler_rx) = mpsc::channel(HANDLER_QUEUE);
        let handler_join = tokio::spawn(run_handler(handler, handler_tx));
        let status = Arc::new(RwLock::new(BrowserStatus {
            state: BrowserState::Restarting,
            active_requests: 0,
            tabs: settings.tabs,
            cooldown_ms: 0,
        }));
        let cooldown_until = Arc::new(Mutex::new(None));
        let gate = Arc::new(ProfileGate::new());
        let actor = Actor::new(
            BrowserConnection {
                browser,
                launched: true,
            },
            settings,
            rx,
            (handler_rx, handler_join),
            status.clone(),
            cooldown_until.clone(),
            gate.clone(),
        );

        let join = tokio::spawn(actor.run());
        let manager = Self {
            tx,
            status,
            cooldown_until,
            gate,
            join: Arc::new(AsyncMutex::new(Some(join))),
        };
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
            if let Err(e) = manager.shutdown().await {
                tracing::warn!("browser shutdown during bootstrap failure: {e}");
            }
            return Err(error);
        }
        Ok(manager)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn connect(
        cdp_url: url::Url,
        settings: BrowserSettings,
    ) -> anyhow::Result<Self> {
        settings.validate()?;
        let handler_config = HandlerConfig {
            request_timeout: settings.request_timeout,
            ..Default::default()
        };
        let (browser, handler) =
            Browser::connect_with_config(cdp_url.as_str(), handler_config).await?;
        let (tx, rx) = mpsc::channel(settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1));
        let (handler_tx, handler_rx) = mpsc::channel(HANDLER_QUEUE);
        let handler_join = tokio::spawn(run_handler(handler, handler_tx));
        let status = Arc::new(RwLock::new(BrowserStatus {
            state: BrowserState::Restarting,
            active_requests: 0,
            tabs: settings.tabs,
            cooldown_ms: 0,
        }));
        let cooldown_until = Arc::new(Mutex::new(None));
        let gate = Arc::new(ProfileGate::new());
        let actor = Actor::new(
            BrowserConnection {
                browser,
                launched: false,
            },
            settings,
            rx,
            (handler_rx, handler_join),
            status.clone(),
            cooldown_until.clone(),
            gate.clone(),
        );
        let join = tokio::spawn(actor.run());
        let manager = Self {
            tx,
            status,
            cooldown_until,
            gate,
            join: Arc::new(AsyncMutex::new(Some(join))),
        };
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
            if let Err(e) = manager.shutdown().await {
                tracing::warn!("browser shutdown during bootstrap failure: {e}");
            }
            return Err(error);
        }
        Ok(manager)
    }
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
