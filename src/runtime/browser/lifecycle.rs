use super::{
    actor::{Actor, BrowserConnection, Command, HandlerEvent},
    gate::ProfileGate,
    pool, BrowserError, BrowserResponse, BrowserSettings, BrowserState, BrowserStatus,
    SHUTDOWN_TIMEOUT,
};
use chromiumoxide::{handler::HandlerConfig, Browser, BrowserConfig, Handler};
use futures::StreamExt;
use std::{
    sync::{Arc, Mutex, RwLock},
    time::Instant,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex as AsyncMutex},
    task::JoinHandle,
};

const QUEUE_MULTIPLIER: usize = 4;
const HANDLER_QUEUE: usize = 256;

pub(crate) struct BrowserManager {
    tx: mpsc::Sender<Command>,
    status: Arc<RwLock<BrowserStatus>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    gate: Arc<ProfileGate>,
    join: Arc<AsyncMutex<Option<JoinHandle<anyhow::Result<()>>>>>,
}
impl Drop for BrowserManager {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.join.try_lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

impl BrowserManager {
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

    pub(crate) async fn fetch(
        &self,
        request: crate::runtime::source::request::RequestSpec,
    ) -> Result<BrowserResponse, BrowserError> {
        if !self.gate.is_ready() {
            return Err(self.gated_error());
        }
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Fetch { request, reply })
            .await
            .map_err(|_| BrowserError::Transport)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) fn status(&self) -> BrowserStatus {
        let status = read_status(&self.status);
        let cooldown_ms = remaining_ms(&self.cooldown_until);
        // Never report Ready when the gate is closed — a stale Ready
        // observation from a previous bootstrap would incorrectly signal
        // that the browser is available. Convert to Challenged instead.
        // Preserve CoolingDown state without reclassification.
        let state = match status.state {
            BrowserState::Ready if !self.gate.is_ready() => BrowserState::Challenged,
            BrowserState::CoolingDown => BrowserState::CoolingDown,
            other => other,
        };
        BrowserStatus {
            state,
            cooldown_ms,
            ..status
        }
    }

    pub(crate) async fn inspect(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Inspect { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) async fn recover(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Recover { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) fn mark_human_required(&self) {
        self.gate.revoke();
        write_state(&self.status, BrowserState::HumanRequired);
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.gate.revoke();
        write_state(&self.status, BrowserState::Stopped);
        let deadline = match tokio::time::Instant::now().checked_add(SHUTDOWN_TIMEOUT) {
            Some(value) => value,
            None => tokio::time::Instant::now() + std::time::Duration::from_secs(300), // overflow guard
        };
        let (reply, result) = oneshot::channel();
        let send_result =
            tokio::time::timeout_at(deadline, self.tx.send(Command::Shutdown { reply })).await;
        let command_result = match send_result {
            Ok(Ok(())) => match tokio::time::timeout_at(deadline, result).await {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => Err(anyhow::anyhow!("browser actor stopped")),
                Err(_) => Err(anyhow::anyhow!("browser shutdown reply timed out")),
            },
            Ok(Err(_)) => Err(anyhow::anyhow!("browser actor stopped")),
            Err(_) => Err(anyhow::anyhow!("browser shutdown send timed out")),
        };
        let mut guard = self.join.lock().await;
        let join = guard.take();
        drop(guard);
        let join_result = match join {
            Some(mut handle) => match tokio::time::timeout_at(deadline, &mut handle).await {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => Err(anyhow::anyhow!("browser actor panicked")),
                Err(_) => {
                    handle.abort();
                    if handle.await.is_err() {
                        tracing::warn!("browser actor aborted after shutdown timeout");
                    }
                    Err(anyhow::anyhow!("browser shutdown timed out"))
                }
            },
            None => Ok(()),
        };
        command_result.and(join_result)
    }

    fn gated_error(&self) -> BrowserError {
        match read_status(&self.status).state {
            BrowserState::Challenged | BrowserState::HumanRequired => BrowserError::HumanRequired,
            BrowserState::Ready => BrowserError::Unavailable,
            BrowserState::CoolingDown | BrowserState::Restarting | BrowserState::Stopped => {
                BrowserError::Unavailable
            }
        }
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

pub(super) fn read_status(status: &RwLock<BrowserStatus>) -> BrowserStatus {
    match status.read() {
        Ok(value) => value.clone(),
        Err(error) => error.into_inner().clone(),
    }
}

pub(super) fn write_state(status: &RwLock<BrowserStatus>, state: BrowserState) {
    match status.write() {
        Ok(mut value) => value.state = state,
        Err(error) => error.into_inner().state = state,
    }
}

pub(super) fn remaining_ms(cooldown: &Mutex<Option<Instant>>) -> u64 {
    let until = match cooldown.lock() {
        Ok(value) => *value,
        Err(error) => *error.into_inner(),
    };
    until.map_or(0, |value| {
        u64::try_from(value.saturating_duration_since(Instant::now()).as_millis())
            .unwrap_or(u64::MAX)
    })
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
