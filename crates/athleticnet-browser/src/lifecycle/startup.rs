use super::super::{
    actor::{Actor, ActorHandles, BrowserOwnership, Command, HandlerEvent},
    gate::ProfileGate,
    BrowserSettings, BrowserState, BrowserStatus,
};
use super::child::OwnedChild;
use super::error::BrowserStartupError;
use super::owner::{BrowserOwner, BrowserStartup, Prepared};
use super::{ActorJoinState, BrowserManager};
use crate::clock::Clock;
use chromiumoxide::{Browser, Handler};
use futures::StreamExt;
use std::process::Stdio;
use std::sync::{Arc, Mutex, RwLock};
use tokio::process::Command as ProcCommand;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

const QUEUE_MULTIPLIER: usize = 4;
const HANDLER_QUEUE: usize = 256;

impl BrowserManager {
    pub fn prepare_launch(
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<(BrowserStartup, BrowserOwner), BrowserStartupError> {
        settings.validate()?;
        let command = build_launch_command(&settings)?;
        Ok(BrowserOwner::prepare(
            Prepared::Launch(command),
            settings,
            clock,
        ))
    }

    pub fn prepare_connect(
        cdp_url: url::Url,
        settings: BrowserSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<(BrowserStartup, BrowserOwner), BrowserStartupError> {
        settings.validate()?;
        crate::profile::validate_cdp_endpoint(&cdp_url)?;
        Ok(BrowserOwner::prepare(
            Prepared::Connect(cdp_url),
            settings,
            clock,
        ))
    }
}

pub(super) fn start_actor(
    browser: (Browser, BrowserOwnership),
    handler: Handler,
    child: Option<OwnedChild>,
    settings: BrowserSettings,
    clock: Arc<dyn Clock>,
) -> Arc<BrowserManager> {
    let (tx, rx) = mpsc::channel(settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1));
    let (handler_tx, handler_rx) = mpsc::channel(HANDLER_QUEUE);
    let handler_stop = CancellationToken::new();
    let handler_join = spawn_handler(handler, handler_tx, handler_stop.clone());
    let status = Arc::new(RwLock::new(BrowserStatus {
        state: BrowserState::Restarting,
        active_requests: 0,
        tabs: settings.tabs,
        cooldown_ms: 0,
    }));
    let cooldown_until = Arc::new(Mutex::new(None));
    let gate = Arc::new(ProfileGate::new());
    let tabs = settings.tabs;
    let mut actor = Actor::new(
        browser,
        settings,
        rx,
        (handler_rx, handler_join),
        ActorHandles {
            status: status.clone(),
            cooldown_until: cooldown_until.clone(),
            gate: gate.clone(),
            clock: clock.clone(),
            handler_stop,
        },
    );
    actor.child = child;
    let span = tracing::info_span!("browser.actor", tabs);
    let join = tokio::spawn(actor.run().instrument(span));
    Arc::new(BrowserManager {
        tx,
        status,
        cooldown_until,
        gate,
        join: Arc::new(AsyncMutex::new(ActorJoinState::Running(join))),
        clock,
    })
}

pub(super) async fn bootstrap(manager: Arc<BrowserManager>) -> Result<(), BrowserStartupError> {
    let (reply, result) = oneshot::channel();
    manager
        .tx
        .send(Command::Bootstrap { reply })
        .await
        .map_err(|_| BrowserStartupError::ActorStopped)?;
    result
        .await
        .map_err(|_| BrowserStartupError::ActorStopped)?
}

fn spawn_handler(
    handler: Handler,
    events: mpsc::Sender<HandlerEvent>,
    stop: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        run_handler(handler, events, stop).instrument(tracing::info_span!("browser.handler")),
    )
}

const DEFAULT_ARGS: [&str; 24] = [
    "--disable-background-networking",
    "--enable-features=NetworkService,NetworkServiceInProcess",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-component-extensions-with-background-pages",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-features=TranslateUI",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--force-color-profile=srgb",
    "--metrics-recording-only",
    "--no-first-run",
    "--enable-automation",
    "--password-store=basic",
    "--use-mock-keychain",
    "--enable-blink-features=IdleDetection",
    "--lang=en_US",
];

pub(super) fn build_launch_command(
    settings: &BrowserSettings,
) -> Result<ProcCommand, BrowserStartupError> {
    let mut profile = std::ffi::OsString::new();
    let profile_bytes = "--user-data-dir="
        .len()
        .checked_add(settings.profile_dir.as_os_str().len())
        .ok_or(BrowserStartupError::ConfigBuildFailed)?;
    profile
        .try_reserve_exact(profile_bytes)
        .map_err(|_| BrowserStartupError::ConfigBuildFailed)?;
    profile.push("--user-data-dir=");
    profile.push(settings.profile_dir.as_os_str());
    let mut command = ProcCommand::new(&settings.executable);
    command
        .kill_on_drop(false)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .args(DEFAULT_ARGS)
        .arg("--remote-debugging-port=0")
        .arg("--disable-extensions")
        .arg(profile);
    if !settings.headed {
        command.args(["--headless", "--hide-scrollbars", "--mute-audio"]);
    }
    Ok(command)
}

async fn run_handler(
    mut handler: Handler,
    events: mpsc::Sender<HandlerEvent>,
    stop: CancellationToken,
) {
    loop {
        let result = tokio::select! {
            biased;
            _ = stop.cancelled() => return,
            result = handler.next() => result,
        };
        if !matches!(result, Some(Ok(_))) {
            if let Err(error) = events.send(HandlerEvent::Failed).await {
                tracing::debug!(%error, "handler event send failed");
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests;
