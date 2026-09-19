use super::{
    gate::ProfileGate,
    navigation::NavigationOutcome,
    pool::{self, PageSlot, Pending},
    BrowserError, BrowserResponse, BrowserSettings, BrowserState, BrowserStatus,
};
use chromiumoxide::Browser;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
    time::Instant,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinHandle, JoinSet},
};
use tokio_util::sync::CancellationToken;
// CancellationToken kept only for observer_stop (page observer lifecycle)
#[path = "lifecycle_ops.rs"]
mod lifecycle_ops;
#[path = "shutdown.rs"]
mod shutdown;
#[path = "state.rs"]
mod state;

pub(super) use super::transport;
const QUEUE_MULTIPLIER: usize = 4;

pub(super) struct Actor {
    pub(super) browser: Option<Browser>,
    pub(super) settings: BrowserSettings,
    pub(super) rx: mpsc::Receiver<Command>,
    pub(super) handler_rx: mpsc::Receiver<HandlerEvent>,
    pub(super) handler_join: Option<JoinHandle<()>>,
    pub(super) jobs: JoinSet<JobResult>,
    pub(super) pages: Vec<PageSlot>,
    pub(super) pending: VecDeque<Pending>,
    pub(super) gate: Arc<ProfileGate>,
    pub(super) challenge_latched: bool,
    pub(super) challenge_target: Option<ChallengeTarget>,
    pub(super) status: Arc<RwLock<BrowserStatus>>,
    pub(super) cooldown_until: Arc<Mutex<Option<Instant>>>,
    // ready field removed — gate.is_ready() is the single source of truth
    pub(super) shutdown_reply: Option<oneshot::Sender<anyhow::Result<()>>>,
    pub(super) draining: bool,
    pub(super) queue_capacity: usize,
    pub(super) observers: JoinSet<Result<(), BrowserError>>,
    pub(super) observer_stop: CancellationToken,
    pub(super) shutdown: CancellationToken,
    pub(super) recovery_used: bool,
    pub(super) panic_shutdown: bool,
    pub(super) capture_sequence: u64,
    pub(super) launched: bool,
}

pub(super) struct ChallengeTarget {
    pub(super) url: url::Url,
    pub(super) post: bool,
}

pub(super) enum Command {
    Bootstrap {
        reply: oneshot::Sender<Result<(), anyhow::Error>>,
    },
    Fetch {
        request: crate::runtime::source::request::RequestSpec,
        reply: oneshot::Sender<Result<BrowserResponse, BrowserError>>,
    },
    Inspect {
        reply: oneshot::Sender<Result<BrowserStatus, BrowserError>>,
    },
    Recover {
        reply: oneshot::Sender<Result<BrowserStatus, BrowserError>>,
    },
    Shutdown {
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
}

pub(super) enum HandlerEvent {
    Failed,
}

pub(super) struct JobResult {
    pub(super) slot: usize,
    pub(super) request: crate::runtime::source::request::RequestSpec,
    pub(super) reply: oneshot::Sender<Result<BrowserResponse, BrowserError>>,
    pub(super) result: Result<BrowserResponse, BrowserError>,
}

impl Actor {
    pub(super) fn new(
        browser: Browser,
        settings: BrowserSettings,
        rx: mpsc::Receiver<Command>,
        handler_pair: (mpsc::Receiver<HandlerEvent>, JoinHandle<()>),
        status: Arc<RwLock<BrowserStatus>>,
        cooldown_until: Arc<Mutex<Option<Instant>>>,
        gate: Arc<ProfileGate>,
        launched: bool,
    ) -> Self {
        let queue_capacity = settings.tabs.saturating_mul(QUEUE_MULTIPLIER).max(1);
        Self {
            browser: Some(browser),
            settings,
            rx,
            handler_rx: handler_pair.0,
            handler_join: Some(handler_pair.1),
            jobs: JoinSet::new(),
            pages: Vec::new(),
            pending: VecDeque::new(),
            gate,
            challenge_latched: false,
            challenge_target: None,
            recovery_used: false,
            status,
            cooldown_until,
            shutdown_reply: None,
            draining: false,
            queue_capacity,
            observers: JoinSet::new(),
            observer_stop: CancellationToken::new(),
            shutdown: CancellationToken::new(),
            panic_shutdown: false,
            capture_sequence: 0,
            launched,
        }
    }

    /// Returns the next capture nonce, incrementing the per-Actor sequence.
    pub(super) fn next_capture_nonce(&mut self) -> u64 {
        let nonce = self.capture_sequence;
        self.capture_sequence = self.capture_sequence.wrapping_add(1);
        nonce
    }

    pub(super) async fn run(mut self) -> anyhow::Result<()> {
        loop {
            if self.panic_shutdown || (self.draining && self.jobs.is_empty()) {
                self.panic_shutdown = false;
                self.draining = true;
                self.shutdown.cancel();
                self.observer_stop.cancel();
                self.gate.revoke();
                pool::reject_pending(&mut self.pending, BrowserError::Shutdown);
                let result = self.close_browser().await;
                if let Some(reply) = self.shutdown_reply.take() {
                    if reply
                        .send(
                            result
                                .as_ref()
                                .map(|_| ())
                                .map_err(|error| anyhow::anyhow!(error.to_string())),
                        )
                        .is_err()
                    {
                        tracing::debug!("browser shutdown reply receiver dropped");
                    }
                }
                self.set_state(BrowserState::Stopped);
                return result;
            }
            tokio::select! {
                command = self.rx.recv() => self.command(command).await,
                event = self.handler_rx.recv() => {
                    if event.is_none() || matches!(event, Some(HandlerEvent::Failed)) {
                        self.gate.revoke();
                        self.shutdown.cancel();
                        pool::reject_pending(&mut self.pending, BrowserError::Unavailable);
                        let cleanup = self.close_browser().await;
                        return match cleanup {
                            Ok(()) => Err(anyhow::anyhow!("browser handler stopped")),
                            Err(error) => Err(anyhow::anyhow!("browser handler stopped; cleanup failed: {}", error)),
                        };
                    }
                }
                job = self.jobs.join_next(), if !self.jobs.is_empty() => self.complete_job(job),
                observer = self.observers.join_next(), if !self.observers.is_empty() => self.complete_observer(observer),
                _ = self.gate.closed(), if {
                    !self.challenge_latched
                        && !self.shutdown.is_cancelled()
                        && !self.pages.is_empty()
                        && self.status.read().ok().map(|s| s.state != BrowserState::CoolingDown).unwrap_or(true)
                } => {
                    self.latch_challenge(None);
                }
            }
            self.schedule_pending();
        }
    }
}

impl Drop for Actor {
    fn drop(&mut self) {
        if let Some(handle) = self.handler_join.take() {
            handle.abort();
        }
        self.jobs.abort_all();
        self.observers.abort_all();
    }
}
